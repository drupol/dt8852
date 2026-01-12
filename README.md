# dt8852

`dt8852` is a driver and command-line utility for the CEM DT-8852 sound level meter and compatible hardware.

This programme reads live sound pressure level (SPL) data through a USB serial connection.
It supports configuration of frequency weightings, time weightings, and measurement ranges.

![Live sound level measurement](resources/Screenshot_20261111_194942.jpg)

## Background

This project started in 2022 because of persistent noise disturbance caused by a commercial heat pump near my home.
I needed a simple, reliable solution to record sound levels continuously using a sound level meter connected to a
Raspberry Pi.

I did not want to depend on a Python runtime environment on an embedded device. Therefore, I decided to write my own
programme in Rust.

The programme has run continuously on the Raspberry Pi ever since, collecting objective acoustic data. Unfortunately,
the noise issue remains unresolved... but that's another story.

## Supported Devices

This programme supports sound level meters that share the same serial communication protocol:

- **CEM DT-8852**
- **CEM DT-8851**
- **Trotec SL400**
- **Voltcraft SL-451**
- **ATP SL-8852**

> **Note**: Switch on the meter and press the **Setup** button once.
> The meter must show the PC connection symbol on its display before communication can start.

## Features

- Automatic serial port detection for USB-connected meters.
- Read live sound pressure level values in decibels (dB).
- Configure frequency weighting (dBA and dBC).
- Configure time weighting (Fast and Slow).
- Configure measurement decibel ranges (30–80 dB, 50–100 dB, 80–130 dB, and Auto).
- Configure peak and minimum hold modes.
- List connected serial ports automatically.
- Export measurements in plain text, JSON, or Telegraf/InfluxDB line protocol.
- Use as a standalone command-line tool or as a Rust library.

## Installation

Install the binary using Cargo:

```bash
cargo install --path .
```

Or build the project locally:

```bash
cargo build --release
```

The compiled binary is located at `target/release/dt8852`.

### Device Permissions (Linux)

Serial devices require appropriate user permissions.
Add your user account to the `dialout` group:

```bash
sudo usermod -aG dialout "$USER"
```

Log out and log in again to apply the group change.

Alternatively, install the provided udev rule:

```bash
sudo cp udev/99-dt8852.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

## Usage

The programme detects connected USB meters automatically.
You can also specify a port explicitly with `--serial-port <PORT>`.

### 1. Identify Connected Ports

List all available serial ports:

```bash
dt8852 list-ports
```

Example output:

```text
Port                 Details
-------------------- ----------------------------------------
/dev/ttyUSB0         CP2102 USB to UART Bridge Controller [VID: 10c4, PID: ea60]
```

### 2. Stream Live Measurements

Stream live decibel readings (using automatic port detection):

```bash
dt8852 live
```

Or specify the serial port manually:

```bash
dt8852 --serial-port /dev/ttyUSB0 live
```

Increase verbosity to display timestamps:

```bash
dt8852 live -v
```

Stream measurements formatted as JSON:

```bash
dt8852 live --format json
```

Stream measurements formatted for Telegraf and InfluxDB:

```bash
dt8852 live --format telegraf
```

Example Telegraf output:

```text
dt8852,frequency_weighting=dB(A),time_weighting=Fast,range_mode=30dB\ -\ 130dB\ auto\ range,hold_mode=Live spl=45.2
```

### 3. Display Current Device Configuration

Query the current device settings:

```bash
dt8852 get-mode
```

To output the configuration as JSON:

```bash
dt8852 get-mode --format json
```

### 4. Configure Device Parameters

Set measurement parameters when starting live monitoring or independently:

```bash
# Set dBA frequency weighting and Slow time weighting
dt8852 set-mode --freqweighting dba --timeweighting slow

# Set 30-80 dB range with Max hold mode
dt8852 set-mode --range R_30_80 --hold max
```

Available parameters:

- `--freqweighting`: `dba`, `dbc`
- `--timeweighting`: `fast`, `slow`
- `--range`: `R_30_80`, `R_50_100`, `R_80_130`, `R_30_130_AUTO`
- `--hold`: `live`, `min`, `max`
- `--record`: `recording`, `not-recording`

## Rust Library Usage

Add `dt8852` to your `Cargo.toml` dependencies:

```toml
[dependencies]
dt8852 = { path = "/path/to/dt8852" }
```

Example code:

```rust
use dt8852::{Dt8852, Token};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = serialport::new("/dev/ttyUSB0", 9600)
        .timeout(Duration::from_millis(5000))
        .open()?;

    let mut meter = Dt8852::new(port)?;

    loop {
        if let Some((token, _changed)) = meter.decode_next_token(true)? {
            if let Token::CurrentSpl(spl) = token {
                println!("Sound Level: {:.1} dB", spl);
            }
        }
    }
}
```

## Running Tests

Run the test suite:

```bash
cargo test
```

Run linter checks:

```bash
cargo clippy -- -D warnings
```

Verify formatting:

```bash
cargo fmt --check
```

## Acknowledgements

This project was inspired by the Python [`dt8852`](https://pypi.org/project/dt8852/) package by Randy Simons
(hosted on [Codeberg](https://codeberg.org/randysimons/dt8852)).

## Licence

This project is licensed under the MIT Licence.
See the [LICENSE](LICENSE) file for details.
