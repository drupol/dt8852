use chrono::{Local, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use serialport::SerialPort;
use std::io::{Read, Write};
use thiserror::Error;

/// Time weighting applied to sound level measurement.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum TimeWeighting {
    /// Fast response (125 ms time constant).
    Fast,
    /// Slow response (1 s time constant).
    Slow,
}

impl TimeWeighting {
    /// Returns the human-readable string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            TimeWeighting::Fast => "Fast",
            TimeWeighting::Slow => "Slow",
        }
    }
}

/// Frequency weighting curve applied to acoustic measurement.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum FrequencyWeighting {
    /// A-weighting curve approximating human ear perception.
    #[serde(rename = "dB(A)")]
    Dba,
    /// C-weighting curve for higher sound pressure levels.
    #[serde(rename = "dB(C)")]
    Dbc,
}

impl FrequencyWeighting {
    /// Returns the standard acoustic abbreviation string.
    pub fn as_str(&self) -> &'static str {
        match self {
            FrequencyWeighting::Dba => "dB(A)",
            FrequencyWeighting::Dbc => "dB(C)",
        }
    }
}

/// Status of the current measurement relative to the selected range.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum RangeThreshold {
    /// Reading is below the active measurement range.
    Under,
    /// Reading is within the active measurement range.
    Ok,
    /// Reading is above the active measurement range.
    Over,
}

impl RangeThreshold {
    /// Returns the human-readable threshold state.
    pub fn as_str(&self) -> &'static str {
        match self {
            RangeThreshold::Under => "Under range",
            RangeThreshold::Ok => "Within range",
            RangeThreshold::Over => "Over range",
        }
    }
}

/// Hold mode for tracking peak or minimum sound levels.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum HoldMode {
    /// Live continuous sound level display.
    Live,
    /// Minimum recorded value hold.
    Min,
    /// Maximum recorded value hold.
    Max,
}

impl HoldMode {
    /// Returns the human-readable hold mode string.
    pub fn as_str(&self) -> &'static str {
        match self {
            HoldMode::Live => "Live",
            HoldMode::Min => "Minimum hold",
            HoldMode::Max => "Maximum hold",
        }
    }
}

/// Measurement decibel range configuration.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum RangeMode {
    /// 30 dB to 80 dB range.
    R30_80,
    /// 50 dB to 100 dB range.
    R50_100,
    /// 80 dB to 130 dB range.
    R80_130,
    /// Automatic range selection from 30 dB to 130 dB.
    R30_130Auto,
}

impl RangeMode {
    /// Returns the human-readable range description.
    pub fn as_str(&self) -> &'static str {
        match self {
            RangeMode::R30_80 => "30dB - 80dB",
            RangeMode::R50_100 => "50dB - 100dB",
            RangeMode::R80_130 => "80dB - 130dB",
            RangeMode::R30_130Auto => "30dB - 130dB auto range",
        }
    }
}

/// Internal data logger recording status.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum RecordingMode {
    /// Logging is paused or inactive.
    NotRecording,
    /// Logging is actively storing readings.
    Recording,
}

impl RecordingMode {
    /// Returns the human-readable recording status.
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordingMode::NotRecording => "Not recording",
            RecordingMode::Recording => "Recording",
        }
    }
}

/// Storage capacity state of internal device memory.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum MemoryStore {
    /// Storage space is available for recordings.
    Available,
    /// Storage space is completely full.
    Full,
}

impl MemoryStore {
    /// Returns the human-readable storage status.
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryStore::Available => "Storage available",
            MemoryStore::Full => "Storage full",
        }
    }
}

/// Battery power condition.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum BatteryState {
    /// Battery level is sufficient.
    Ok,
    /// Battery level is critically low.
    Low,
}

impl BatteryState {
    /// Returns the human-readable battery condition.
    pub fn as_str(&self) -> &'static str {
        match self {
            BatteryState::Ok => "Battery OK",
            BatteryState::Low => "Battery low",
        }
    }
}

/// Display target destination on the meter front panel.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum OutputTo {
    /// Digital numeric readout.
    Display,
    /// Analogue-style bar graph display.
    BarGraph,
}

impl OutputTo {
    /// Returns the human-readable output display mode.
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputTo::Display => "Digits",
            OutputTo::BarGraph => "Bar graph",
        }
    }
}

/// Serial control command byte sent to the DT-8852.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommandToken {
    /// Power down device command.
    PowerDown = 0x33,
    /// Toggle data logging state.
    ToggleRecordingMode = 0x55,
    /// Toggle hold mode (Live -> Max -> Min).
    ToggleHoldMode = 0x11,
    /// Toggle time weighting (Fast <-> Slow).
    ToggleTimeWeighting = 0x77,
    /// Toggle range mode.
    ToggleRangeMode = 0x88,
    /// Toggle frequency weighting (dBA <-> dBC).
    ToggleFrequencyWeighting = 0x99,
    /// Download stored data logger records.
    GetRecording = 0xac,
}

/// Target device setting to reach when reconfiguring parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigMode {
    /// Configure time weighting.
    Time(TimeWeighting),
    /// Configure frequency weighting.
    Frequency(FrequencyWeighting),
    /// Configure decibel range.
    Range(RangeMode),
    /// Configure hold mode.
    Hold(HoldMode),
    /// Configure recording mode.
    Recording(RecordingMode),
}

/// Decoded protocol token emitted by the DT-8852 meter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Token {
    /// Time weighting token.
    TimeWeighting(TimeWeighting),
    /// Hold mode token.
    HoldMode(HoldMode),
    /// Device real-time clock token.
    CurrentTime(NaiveTime),
    /// Measurement range threshold token.
    RangeThreshold(RangeThreshold),
    /// Memory capacity token.
    MemoryStore(MemoryStore),
    /// Data recording token.
    RecordingMode(RecordingMode),
    /// Screen output display token.
    OutputTo(OutputTo),
    /// Current sound pressure level in decibels (dB).
    CurrentSpl(f64),
    /// Battery condition token.
    BatteryState(BatteryState),
    /// Frequency weighting token.
    FrequencyWeighting(FrequencyWeighting),
    /// Decibel range token.
    RangeMode(RangeMode),
}

impl Token {
    /// Returns the identifier name of the token without heap allocation.
    pub fn name(&self) -> &'static str {
        match self {
            Token::TimeWeighting(_) => "time_weighting",
            Token::HoldMode(_) => "hold_mode",
            Token::CurrentTime(_) => "current_time",
            Token::RangeThreshold(_) => "range_threshold",
            Token::MemoryStore(_) => "memory_store",
            Token::RecordingMode(_) => "recording_mode",
            Token::OutputTo(_) => "output_to",
            Token::CurrentSpl(_) => "current_spl",
            Token::BatteryState(_) => "battery_state",
            Token::FrequencyWeighting(_) => "frequency_weighting",
            Token::RangeMode(_) => "range_mode",
        }
    }
}

/// Complete snapshot of the DT-8852 device state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Dt8852State {
    /// Most recent sound pressure level in decibels (dB).
    pub current_spl: Option<f64>,
    /// Device internal clock time.
    pub current_time: Option<NaiveTime>,
    /// Current time weighting.
    pub time_weighting: Option<TimeWeighting>,
    /// Current frequency weighting.
    pub frequency_weighting: Option<FrequencyWeighting>,
    /// Current range threshold status.
    pub range_threshold: Option<RangeThreshold>,
    /// Current hold mode.
    pub hold_mode: Option<HoldMode>,
    /// Current range mode.
    pub range_mode: Option<RangeMode>,
    /// Current recording mode.
    pub recording_mode: Option<RecordingMode>,
    /// Current memory storage status.
    pub memory_store: Option<MemoryStore>,
    /// Current battery condition.
    pub battery_state: Option<BatteryState>,
    /// Current screen readout target.
    pub output_to: Option<OutputTo>,
}

impl Dt8852State {
    /// Creates a complete timestamp combining `current_time` with today's local date.
    pub fn current_datetime(&self) -> Option<NaiveDateTime> {
        self.current_time.map(|time| {
            let now = Local::now().naive_local();
            NaiveDateTime::new(now.date(), time)
        })
    }
}

/// Errors occurring during communication with the DT-8852 sound level meter.
#[derive(Error, Debug)]
pub enum Dt8852Error {
    /// Serial port communication failure.
    #[error("Serial port error: {0}")]
    Serial(#[from] serialport::Error),

    /// Standard I/O communication failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Unknown or unhandled token byte received from meter.
    #[error("Unknown token byte: 0x{0:02x}")]
    UnknownToken(u8),
}

/// Driver interface for the CEM DT-8852 sound level meter.
pub struct Dt8852<T = Box<dyn SerialPort>> {
    io: T,
    /// Latest synchronized state received from the device.
    pub state: Dt8852State,
    set_modes: Vec<ConfigMode>,
    set_mode_command: Option<CommandToken>,
    wait_for_value: Option<ConfigMode>,
    throttle_counter: usize,
}

impl Dt8852<Box<dyn SerialPort>> {
    /// Initialises a connection over a physical serial port.
    pub fn new(port: Box<dyn SerialPort>) -> Result<Self, Dt8852Error> {
        port.clear(serialport::ClearBuffer::Input)?;
        port.clear(serialport::ClearBuffer::Output)?;
        Ok(Self::from_stream(port))
    }
}

impl<T: Read + Write> Dt8852<T> {
    /// Initialises the driver with any read/write stream (useful for tests and mocks).
    pub fn from_stream(io: T) -> Self {
        Self {
            io,
            state: Dt8852State::default(),
            set_modes: Vec::new(),
            set_mode_command: None,
            wait_for_value: None,
            throttle_counter: 0,
        }
    }

    /// Queues configuration modes to apply to the device.
    pub fn set_mode(&mut self, modes: Vec<ConfigMode>) {
        self.set_modes = modes;
        self.set_next_mode(true);
    }

    fn set_next_mode(&mut self, first: bool) {
        if !first && !self.set_modes.is_empty() {
            self.set_modes.remove(0);
        }

        if self.set_modes.is_empty() {
            self.set_mode_command = None;
            self.wait_for_value = None;
            return;
        }

        let mode = self.set_modes[0];
        self.wait_for_value = Some(mode);

        self.set_mode_command = Some(match mode {
            ConfigMode::Time(_) => CommandToken::ToggleTimeWeighting,
            ConfigMode::Frequency(_) => CommandToken::ToggleFrequencyWeighting,
            ConfigMode::Range(_) => CommandToken::ToggleRangeMode,
            ConfigMode::Hold(_) => CommandToken::ToggleHoldMode,
            ConfigMode::Recording(_) => CommandToken::ToggleRecordingMode,
        });
    }

    /// Returns `true` if device mode changes are still pending.
    pub fn is_configuring(&self) -> bool {
        !self.set_modes.is_empty() || self.set_mode_command.is_some()
    }

    /// Reads and decodes the next protocol token from the device stream.
    ///
    /// When `changes_only` is true, unchanged values are skipped.
    pub fn decode_next_token(
        &mut self,
        changes_only: bool,
    ) -> Result<Option<(Token, bool)>, Dt8852Error> {
        let supported_tokens = [
            0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
            0x11, 0x19, 0x1a, 0x1b, 0x1c, 0x1f, 0x30, 0x40, 0x4b, 0x4c,
        ];

        let mut buf = [0u8; 1];

        loop {
            // Synchronise on start frame byte 0xa5
            loop {
                self.io.read_exact(&mut buf)?;
                if buf[0] == 0xa5 {
                    break;
                }
            }

            self.io.read_exact(&mut buf)?;
            let token_byte = buf[0];

            if !supported_tokens.contains(&token_byte) {
                continue;
            }

            let (decoded_token, value_changed) = self.decode_token(token_byte)?;

            // Handle device configuration state machine
            if let Some(cmd) = self.set_mode_command {
                self.throttle_counter += 1;

                let mode_matched = match (self.wait_for_value, &decoded_token) {
                    (Some(ConfigMode::Time(target)), Token::TimeWeighting(val)) => target == *val,
                    (Some(ConfigMode::Frequency(target)), Token::FrequencyWeighting(val)) => {
                        target == *val
                    }
                    (Some(ConfigMode::Range(target)), Token::RangeMode(val)) => target == *val,
                    (Some(ConfigMode::Hold(target)), Token::HoldMode(val)) => target == *val,
                    (Some(ConfigMode::Recording(target)), Token::RecordingMode(val)) => {
                        target == *val
                    }
                    _ => false,
                };

                if mode_matched {
                    self.set_next_mode(false);
                } else if self.throttle_counter.is_multiple_of(9) {
                    self.io.write_all(&[cmd as u8])?;
                    self.io.flush()?;
                }
            }

            if value_changed || !changes_only {
                return Ok(Some((decoded_token, value_changed)));
            }
        }
    }

    fn decode_token(&mut self, token: u8) -> Result<(Token, bool), Dt8852Error> {
        match token {
            0x02 => {
                let changed = self.state.time_weighting != Some(TimeWeighting::Fast);
                self.state.time_weighting = Some(TimeWeighting::Fast);
                Ok((Token::TimeWeighting(TimeWeighting::Fast), changed))
            }
            0x03 => {
                let changed = self.state.time_weighting != Some(TimeWeighting::Slow);
                self.state.time_weighting = Some(TimeWeighting::Slow);
                Ok((Token::TimeWeighting(TimeWeighting::Slow), changed))
            }
            0x04 => {
                let changed = self.state.hold_mode != Some(HoldMode::Max);
                self.state.hold_mode = Some(HoldMode::Max);
                Ok((Token::HoldMode(HoldMode::Max), changed))
            }
            0x05 => {
                let changed = self.state.hold_mode != Some(HoldMode::Min);
                self.state.hold_mode = Some(HoldMode::Min);
                Ok((Token::HoldMode(HoldMode::Min), changed))
            }
            0x06 => {
                let mut data = [0u8; 3];
                self.io.read_exact(&mut data)?;

                let hours_raw = data[0] & 0x1f;
                let minutes_raw = data[1];
                let seconds_raw = data[2];
                let is_pm = (data[0] >> 5) > 0;

                let mut hours = bcd_byte_to_u8(hours_raw);
                let minutes = bcd_byte_to_u8(minutes_raw);
                let seconds = bcd_byte_to_u8(seconds_raw);

                if hours == 0 && !is_pm {
                    hours = 12;
                }

                if is_pm && hours < 12 {
                    hours += 12;
                } else if !is_pm && hours == 12 {
                    hours = 0;
                }

                let time = NaiveTime::from_hms_opt(hours as u32, minutes as u32, seconds as u32)
                    .unwrap_or_default();

                let changed = self.state.current_time != Some(time);
                self.state.current_time = Some(time);
                Ok((Token::CurrentTime(time), changed))
            }
            0x07 => {
                let changed = self.state.range_threshold != Some(RangeThreshold::Over);
                self.state.range_threshold = Some(RangeThreshold::Over);
                Ok((Token::RangeThreshold(RangeThreshold::Over), changed))
            }
            0x08 => {
                let changed = self.state.range_threshold != Some(RangeThreshold::Under);
                self.state.range_threshold = Some(RangeThreshold::Under);
                Ok((Token::RangeThreshold(RangeThreshold::Under), changed))
            }
            0x09 => {
                let changed = self.state.memory_store != Some(MemoryStore::Full);
                self.state.memory_store = Some(MemoryStore::Full);
                Ok((Token::MemoryStore(MemoryStore::Full), changed))
            }
            0x0a => {
                let changed = self.state.recording_mode != Some(RecordingMode::Recording);
                self.state.recording_mode = Some(RecordingMode::Recording);
                Ok((Token::RecordingMode(RecordingMode::Recording), changed))
            }
            0x0b => {
                let changed = self.state.output_to != Some(OutputTo::Display);
                self.state.output_to = Some(OutputTo::Display);
                Ok((Token::OutputTo(OutputTo::Display), changed))
            }
            0x0c => {
                let changed = self.state.output_to != Some(OutputTo::BarGraph);
                self.state.output_to = Some(OutputTo::BarGraph);
                Ok((Token::OutputTo(OutputTo::BarGraph), changed))
            }
            0x0d => {
                let mut data = [0u8; 2];
                self.io.read_exact(&mut data)?;
                let current_spl = decode_spl_bcd(data);
                let changed = self.state.current_spl != Some(current_spl);
                self.state.current_spl = Some(current_spl);
                Ok((Token::CurrentSpl(current_spl), changed))
            }
            0x0e => {
                let changed = self.state.hold_mode != Some(HoldMode::Live);
                self.state.hold_mode = Some(HoldMode::Live);
                Ok((Token::HoldMode(HoldMode::Live), changed))
            }
            0x0f => {
                let changed = self.state.battery_state != Some(BatteryState::Low);
                self.state.battery_state = Some(BatteryState::Low);
                Ok((Token::BatteryState(BatteryState::Low), changed))
            }
            0x11 => {
                let changed = self.state.range_threshold != Some(RangeThreshold::Ok);
                self.state.range_threshold = Some(RangeThreshold::Ok);
                Ok((Token::RangeThreshold(RangeThreshold::Ok), changed))
            }
            0x19 => {
                let changed = self.state.memory_store != Some(MemoryStore::Available);
                self.state.memory_store = Some(MemoryStore::Available);
                Ok((Token::MemoryStore(MemoryStore::Available), changed))
            }
            0x1a => {
                let changed = self.state.recording_mode != Some(RecordingMode::NotRecording);
                self.state.recording_mode = Some(RecordingMode::NotRecording);
                Ok((Token::RecordingMode(RecordingMode::NotRecording), changed))
            }
            0x1b => {
                let changed = self.state.frequency_weighting != Some(FrequencyWeighting::Dba);
                self.state.frequency_weighting = Some(FrequencyWeighting::Dba);
                Ok((Token::FrequencyWeighting(FrequencyWeighting::Dba), changed))
            }
            0x1c => {
                let changed = self.state.frequency_weighting != Some(FrequencyWeighting::Dbc);
                self.state.frequency_weighting = Some(FrequencyWeighting::Dbc);
                Ok((Token::FrequencyWeighting(FrequencyWeighting::Dbc), changed))
            }
            0x1f => {
                let changed = self.state.battery_state != Some(BatteryState::Ok);
                self.state.battery_state = Some(BatteryState::Ok);
                Ok((Token::BatteryState(BatteryState::Ok), changed))
            }
            0x30 => {
                let changed = self.state.range_mode != Some(RangeMode::R30_80);
                self.state.range_mode = Some(RangeMode::R30_80);
                Ok((Token::RangeMode(RangeMode::R30_80), changed))
            }
            0x40 => {
                let changed = self.state.range_mode != Some(RangeMode::R30_130Auto);
                self.state.range_mode = Some(RangeMode::R30_130Auto);
                Ok((Token::RangeMode(RangeMode::R30_130Auto), changed))
            }
            0x4b => {
                let changed = self.state.range_mode != Some(RangeMode::R50_100);
                self.state.range_mode = Some(RangeMode::R50_100);
                Ok((Token::RangeMode(RangeMode::R50_100), changed))
            }
            0x4c => {
                let changed = self.state.range_mode != Some(RangeMode::R80_130);
                self.state.range_mode = Some(RangeMode::R80_130);
                Ok((Token::RangeMode(RangeMode::R80_130), changed))
            }
            _ => Err(Dt8852Error::UnknownToken(token)),
        }
    }
}

/// Converts a 2-digit packed BCD byte to standard unsigned integer.
#[inline]
fn bcd_byte_to_u8(byte: u8) -> u8 {
    ((byte >> 4) * 10) + (byte & 0x0f)
}

/// Decodes sound pressure level from 2 BCD bytes.
#[inline]
fn decode_spl_bcd(data: [u8; 2]) -> f64 {
    let thousands = (data[0] >> 4) as u32 * 1000;
    let hundreds = (data[0] & 0x0f) as u32 * 100;
    let tens = (data[1] >> 4) as u32 * 10;
    let units = (data[1] & 0x0f) as u32;
    (thousands + hundreds + tens + units) as f64 / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_decode_spl_bcd() {
        assert_eq!(decode_spl_bcd([0x06, 0x48]), 64.8);
        assert_eq!(decode_spl_bcd([0x03, 0x00]), 30.0);
        assert_eq!(decode_spl_bcd([0x13, 0x00]), 130.0);
        assert_eq!(decode_spl_bcd([0x11, 0x24]), 112.4);
    }

    #[test]
    fn test_bcd_byte_to_u8() {
        assert_eq!(bcd_byte_to_u8(0x00), 0);
        assert_eq!(bcd_byte_to_u8(0x12), 12);
        assert_eq!(bcd_byte_to_u8(0x59), 59);
    }

    #[test]
    fn test_decode_tokens_from_stream() {
        let stream = vec![0xa5, 0x02, 0xa5, 0x1b, 0xa5, 0x0d, 0x05, 0x42, 0xa5, 0x0b];
        let cursor = Cursor::new(stream);
        let mut dt8852 = Dt8852::from_stream(cursor);

        let t1 = dt8852.decode_next_token(false).unwrap().unwrap();
        assert_eq!(t1.0, Token::TimeWeighting(TimeWeighting::Fast));
        assert!(t1.1);

        let t2 = dt8852.decode_next_token(false).unwrap().unwrap();
        assert_eq!(t2.0, Token::FrequencyWeighting(FrequencyWeighting::Dba));
        assert!(t2.1);

        let t3 = dt8852.decode_next_token(false).unwrap().unwrap();
        assert_eq!(t3.0, Token::CurrentSpl(54.2));
        assert!(t3.1);

        let t4 = dt8852.decode_next_token(false).unwrap().unwrap();
        assert_eq!(t4.0, Token::OutputTo(OutputTo::Display));
        assert!(t4.1);

        assert_eq!(dt8852.state.time_weighting, Some(TimeWeighting::Fast));
        assert_eq!(
            dt8852.state.frequency_weighting,
            Some(FrequencyWeighting::Dba)
        );
        assert_eq!(dt8852.state.current_spl, Some(54.2));
        assert_eq!(dt8852.state.output_to, Some(OutputTo::Display));
    }

    #[test]
    fn test_decode_time_token() {
        let stream = vec![0xa5, 0x06, 0x21, 0x45, 0x30];
        let cursor = Cursor::new(stream);
        let mut dt8852 = Dt8852::from_stream(cursor);

        let res = dt8852.decode_next_token(false).unwrap().unwrap();
        let expected_time = NaiveTime::from_hms_opt(13, 45, 30).unwrap();
        assert_eq!(res.0, Token::CurrentTime(expected_time));
        assert_eq!(dt8852.state.current_time, Some(expected_time));
    }

    #[test]
    fn test_token_names() {
        assert_eq!(Token::CurrentSpl(45.0).name(), "current_spl");
        assert_eq!(
            Token::TimeWeighting(TimeWeighting::Slow).name(),
            "time_weighting"
        );
        assert_eq!(
            Token::FrequencyWeighting(FrequencyWeighting::Dba).name(),
            "frequency_weighting"
        );
    }

    #[test]
    fn test_changes_only_filter() {
        let stream = vec![0xa5, 0x02, 0xa5, 0x02];
        let cursor = Cursor::new(stream);
        let mut dt8852 = Dt8852::from_stream(cursor);

        let t1 = dt8852.decode_next_token(true).unwrap().unwrap();
        assert!(t1.1);

        let t2 = dt8852.decode_next_token(true);
        assert!(t2.is_err());
    }
}
