use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use dt8852::{
    ConfigMode, Dt8852, FrequencyWeighting, HoldMode, OutputTo, RangeMode, RecordingMode,
    TimeWeighting, Token,
};
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(
    name = "dt8852",
    author,
    version,
    about = "Command-line interface for the CEM DT-8852 sound level meter",
    long_about = None
)]
struct Cli {
    /// Serial port device path (e.g. /dev/ttyUSB0 or COM3). Auto-detected if omitted.
    #[arg(long, global = true)]
    serial_port: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Stream live decibel sound measurements
    Live {
        /// Increase output verbosity (0=SPL only, 1=time+SPL, 2=SPL per token, 3=time+SPL per token, 4=raw tokens)
        #[arg(short, long, action = clap::ArgAction::Count)]
        verbosity: u8,

        #[clap(flatten)]
        mode_args: ModeArgs,

        /// Output data format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Reconfigure meter settings (range, weightings, hold mode, etc.)
    SetMode {
        #[clap(flatten)]
        mode_args: ModeArgs,
    },
    /// Query and display current device configuration and parameters
    GetMode {
        /// Maximum seconds to wait for meter state synchronisation
        #[arg(long, default_value_t = 5)]
        timeout: u64,

        /// Output data format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// List detected serial communication ports
    ListPorts,
}

#[derive(Parser, Debug)]
struct ModeArgs {
    #[arg(long, value_enum)]
    range: Option<RangeModeEnum>,

    #[arg(long, value_enum)]
    freqweighting: Option<FrequencyWeightingEnum>,

    #[arg(long, value_enum)]
    timeweighting: Option<TimeWeightingEnum>,

    #[arg(long, value_enum)]
    hold: Option<HoldModeEnum>,

    #[arg(long, value_enum)]
    record: Option<RecordingModeEnum>,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum RangeModeEnum {
    #[value(name = "R_30_80")]
    R30_80,
    #[value(name = "R_50_100")]
    R50_100,
    #[value(name = "R_80_130")]
    R80_130,
    #[value(name = "R_30_130_AUTO")]
    R30_130Auto,
}

impl From<RangeModeEnum> for RangeMode {
    fn from(val: RangeModeEnum) -> Self {
        match val {
            RangeModeEnum::R30_80 => RangeMode::R30_80,
            RangeModeEnum::R50_100 => RangeMode::R50_100,
            RangeModeEnum::R80_130 => RangeMode::R80_130,
            RangeModeEnum::R30_130Auto => RangeMode::R30_130Auto,
        }
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum FrequencyWeightingEnum {
    #[value(name = "dBA", alias = "dba", alias = "DBA")]
    Dba,
    #[value(name = "dBC", alias = "dbc", alias = "DBC")]
    Dbc,
}

impl From<FrequencyWeightingEnum> for FrequencyWeighting {
    fn from(val: FrequencyWeightingEnum) -> Self {
        match val {
            FrequencyWeightingEnum::Dba => FrequencyWeighting::Dba,
            FrequencyWeightingEnum::Dbc => FrequencyWeighting::Dbc,
        }
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum TimeWeightingEnum {
    Fast,
    Slow,
}

impl From<TimeWeightingEnum> for TimeWeighting {
    fn from(val: TimeWeightingEnum) -> Self {
        match val {
            TimeWeightingEnum::Fast => TimeWeighting::Fast,
            TimeWeightingEnum::Slow => TimeWeighting::Slow,
        }
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum HoldModeEnum {
    Live,
    Min,
    Max,
}

impl From<HoldModeEnum> for HoldMode {
    fn from(val: HoldModeEnum) -> Self {
        match val {
            HoldModeEnum::Live => HoldMode::Live,
            HoldModeEnum::Min => HoldMode::Min,
            HoldModeEnum::Max => HoldMode::Max,
        }
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum RecordingModeEnum {
    NotRecording,
    Recording,
}

impl From<RecordingModeEnum> for RecordingMode {
    fn from(val: RecordingModeEnum) -> Self {
        match val {
            RecordingModeEnum::NotRecording => RecordingMode::NotRecording,
            RecordingModeEnum::Recording => RecordingMode::Recording,
        }
    }
}

fn get_modes_from_args(args: &ModeArgs) -> Vec<ConfigMode> {
    let mut modes = Vec::new();
    if let Some(range) = args.range {
        modes.push(ConfigMode::Range(range.into()));
    }
    if let Some(freq) = args.freqweighting {
        modes.push(ConfigMode::Frequency(freq.into()));
    }
    if let Some(time) = args.timeweighting {
        modes.push(ConfigMode::Time(time.into()));
    }
    if let Some(hold) = args.hold {
        modes.push(ConfigMode::Hold(hold.into()));
    }
    if let Some(rec) = args.record {
        modes.push(ConfigMode::Recording(rec.into()));
    }
    modes
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Telegraf,
    Json,
}

fn resolve_serial_port(specified: Option<&str>) -> Result<String> {
    if let Some(port) = specified {
        return Ok(port.to_string());
    }

    let ports = serialport::available_ports().context("Failed to enumerate serial ports")?;

    let usb_ports: Vec<_> = ports
        .into_iter()
        .filter_map(|p| match p.port_type {
            serialport::SerialPortType::UsbPort(info) => Some((p.port_name, info)),
            _ => None,
        })
        .collect();

    if usb_ports.is_empty() {
        bail!(
            "No USB serial port detected. Connect the sound level meter or specify --serial-port <PORT>."
        );
    }

    // Prefer Silicon Labs CP2102 (standard for DT-8852, VID: 0x10c4, PID: 0xea60)
    let cp210x_matches: Vec<_> = usb_ports
        .iter()
        .filter(|(_, info)| info.vid == 0x10c4 && info.pid == 0xea60)
        .collect();

    if cp210x_matches.len() == 1 {
        let port_name = &cp210x_matches[0].0;
        eprintln!("Auto-detected sound meter on: {port_name} (CP2102 USB-UART)");
        return Ok(port_name.clone());
    }

    // If exactly one USB serial port exists overall, use it
    if usb_ports.len() == 1 {
        let (port_name, info) = &usb_ports[0];
        let product = info.product.as_deref().unwrap_or("USB Serial Device");
        eprintln!("Auto-detected sound meter on: {port_name} ({product})");
        return Ok(port_name.clone());
    }

    // Multiple ambiguous USB serial ports
    let mut candidate_list = String::new();
    for (name, info) in &usb_ports {
        let prod = info.product.as_deref().unwrap_or("Unknown");
        candidate_list.push_str(&format!(
            "  - {name} ({prod}, VID: {:04x}, PID: {:04x})\n",
            info.vid, info.pid
        ));
    }

    bail!(
        "Multiple USB serial ports detected. Please specify one with --serial-port <PORT>:\n{candidate_list}"
    );
}

fn open_port(port_name: &str) -> Result<Box<dyn serialport::SerialPort>> {
    serialport::new(port_name, 9600)
        .timeout(Duration::from_millis(10000))
        .open()
        .with_context(|| {
            format!(
                "Failed to open serial port '{port_name}'. Check device path and user permissions."
            )
        })
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::ListPorts => {
            let ports = serialport::available_ports().context("Failed to list serial ports")?;
            if ports.is_empty() {
                println!("No serial ports found.");
            } else {
                println!("{:<20} Details", "Port");
                println!("{:-<20} {:-<40}", "", "");
                for p in ports {
                    let details = match &p.port_type {
                        serialport::SerialPortType::UsbPort(info) => {
                            let product = info.product.as_deref().unwrap_or("USB Device");
                            let mfg = info.manufacturer.as_deref().unwrap_or("");
                            format!(
                                "{product} ({mfg}) [VID: {:04x}, PID: {:04x}]",
                                info.vid, info.pid
                            )
                        }
                        serialport::SerialPortType::PciPort => "PCI Port".to_string(),
                        serialport::SerialPortType::BluetoothPort => "Bluetooth Port".to_string(),
                        serialport::SerialPortType::Unknown => "Unknown Port".to_string(),
                    };
                    println!("{:<20} {}", p.port_name, details);
                }
            }
        }
        Commands::Live {
            verbosity,
            mode_args,
            format,
        } => {
            let port_name = resolve_serial_port(cli.serial_port.as_deref())?;
            let port = open_port(&port_name)?;
            let mut dt8852 = Dt8852::new(port)?;
            dt8852.set_mode(get_modes_from_args(mode_args));

            loop {
                let result = dt8852.decode_next_token(*verbosity >= 4)?;
                if let Some((token, _value_changed)) = result {
                    match format {
                        OutputFormat::Telegraf => {
                            if let Token::OutputTo(OutputTo::Display) = token
                                && let Some(spl) = dt8852.state.current_spl
                            {
                                let escape = |s: &str| {
                                    s.replace(' ', "\\ ")
                                        .replace(',', "\\,")
                                        .replace('=', "\\=")
                                };
                                let mut tags = Vec::new();
                                if let Some(fw) = dt8852.state.frequency_weighting {
                                    tags.push(format!(
                                        "frequency_weighting={}",
                                        escape(fw.as_str())
                                    ));
                                }
                                if let Some(tw) = dt8852.state.time_weighting {
                                    tags.push(format!("time_weighting={}", escape(tw.as_str())));
                                }
                                if let Some(rm) = dt8852.state.range_mode {
                                    tags.push(format!("range_mode={}", escape(rm.as_str())));
                                }
                                if let Some(hm) = dt8852.state.hold_mode {
                                    tags.push(format!("hold_mode={}", escape(hm.as_str())));
                                }

                                let tag_str = if tags.is_empty() {
                                    String::new()
                                } else {
                                    format!(",{}", tags.join(","))
                                };
                                println!("dt8852{} spl={}", tag_str, spl);
                            }
                        }
                        OutputFormat::Json => {
                            if let Token::OutputTo(OutputTo::Display) = token
                                && let Some(spl) = dt8852.state.current_spl
                            {
                                let json_obj = serde_json::json!({
                                    "spl": spl,
                                    "time": dt8852.state.current_time.map(|t| t.format("%H:%M:%S").to_string()),
                                    "frequency_weighting": dt8852.state.frequency_weighting.map(|fw| fw.as_str()),
                                    "time_weighting": dt8852.state.time_weighting.map(|tw| tw.as_str()),
                                    "range_mode": dt8852.state.range_mode.map(|rm| rm.as_str()),
                                    "hold_mode": dt8852.state.hold_mode.map(|hm| hm.as_str()),
                                    "battery_state": dt8852.state.battery_state.map(|bs| bs.as_str()),
                                });
                                println!("{}", json_obj);
                            }
                        }
                        OutputFormat::Text => {
                            if *verbosity <= 1 {
                                if let Token::OutputTo(OutputTo::Display) = token
                                    && let Some(spl) = dt8852.state.current_spl
                                {
                                    if *verbosity == 0 {
                                        println!("{}", spl);
                                    } else {
                                        let time_str = dt8852
                                            .state
                                            .current_time
                                            .map(|t| t.to_string())
                                            .unwrap_or_else(|| "Unknown".to_string());
                                        println!("{},{}", time_str, spl);
                                    }
                                }
                            } else if *verbosity <= 3 {
                                if let Token::CurrentSpl(spl) = token {
                                    if *verbosity == 2 {
                                        println!("{}", spl);
                                    } else {
                                        let time_str = dt8852
                                            .state
                                            .current_time
                                            .map(|t| t.to_string())
                                            .unwrap_or_else(|| "Unknown".to_string());
                                        println!("{},{}", time_str, spl);
                                    }
                                }
                            } else {
                                println!("{:?}", token);
                            }
                        }
                    }
                }
            }
        }
        Commands::SetMode { mode_args } => {
            let port_name = resolve_serial_port(cli.serial_port.as_deref())?;
            let port = open_port(&port_name)?;
            let mut dt8852 = Dt8852::new(port)?;
            let modes = get_modes_from_args(mode_args);
            dt8852.set_mode(modes);

            loop {
                dt8852.decode_next_token(true)?;
                if !dt8852.is_configuring() {
                    break;
                }
            }
            println!("{:?}", dt8852.state);
        }
        Commands::GetMode { timeout, format } => {
            let port_name = resolve_serial_port(cli.serial_port.as_deref())?;
            let port = open_port(&port_name)?;
            let mut dt8852 = Dt8852::new(port)?;
            let start = Instant::now();
            let max_duration = Duration::from_secs(*timeout);

            loop {
                dt8852.decode_next_token(true)?;
                let s = &dt8852.state;
                let is_complete = s.current_spl.is_some()
                    && s.current_time.is_some()
                    && s.time_weighting.is_some()
                    && s.frequency_weighting.is_some()
                    && s.range_threshold.is_some()
                    && s.hold_mode.is_some()
                    && s.range_mode.is_some()
                    && s.recording_mode.is_some()
                    && s.memory_store.is_some()
                    && s.battery_state.is_some()
                    && s.output_to.is_some();

                if is_complete || start.elapsed() >= max_duration {
                    break;
                }
            }

            match format {
                OutputFormat::Json => {
                    let json_str = serde_json::to_string_pretty(&dt8852.state)?;
                    println!("{}", json_str);
                }
                _ => {
                    let s = &dt8852.state;
                    println!("DT-8852 Meter Configuration & State:");
                    println!(
                        "  Sound Pressure Level: {}",
                        s.current_spl
                            .map(|v| format!("{:.1} dB", v))
                            .unwrap_or_else(|| "Unknown".to_string())
                    );
                    println!(
                        "  Time:                 {}",
                        s.current_time
                            .map(|t| t.format("%H:%M:%S").to_string())
                            .unwrap_or_else(|| "Unknown".to_string())
                    );
                    println!(
                        "  Frequency Weighting:  {}",
                        s.frequency_weighting
                            .map(|w| w.as_str())
                            .unwrap_or("Unknown")
                    );
                    println!(
                        "  Time Weighting:       {}",
                        s.time_weighting.map(|w| w.as_str()).unwrap_or("Unknown")
                    );
                    println!(
                        "  Range Mode:           {}",
                        s.range_mode.map(|r| r.as_str()).unwrap_or("Unknown")
                    );
                    println!(
                        "  Hold Mode:            {}",
                        s.hold_mode.map(|h| h.as_str()).unwrap_or("Unknown")
                    );
                    println!(
                        "  Range Threshold:      {}",
                        s.range_threshold.map(|t| t.as_str()).unwrap_or("Unknown")
                    );
                    println!(
                        "  Recording Status:     {}",
                        s.recording_mode.map(|r| r.as_str()).unwrap_or("Unknown")
                    );
                    println!(
                        "  Memory Storage:       {}",
                        s.memory_store.map(|m| m.as_str()).unwrap_or("Unknown")
                    );
                    println!(
                        "  Battery State:        {}",
                        s.battery_state.map(|b| b.as_str()).unwrap_or("Unknown")
                    );
                    println!(
                        "  Display Output:       {}",
                        s.output_to.map(|o| o.as_str()).unwrap_or("Unknown")
                    );
                }
            }
        }
    }

    Ok(())
}
