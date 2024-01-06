# qhyccd-alpaca-rs

[![Crates.io](https://img.shields.io/crates/v/qhyccd-alpaca-rs.svg)](https://crates.io/crates/qhyccd-alpaca-rs)
[![Documentation](https://docs.rs/qhyccd-alpaca-rs/badge.svg)](https://docs.rs/qhyccd-alpaca-rs/)
[![Codecov](https://codecov.io/github/ivonnyssen/qhyccd-alpaca-rs/coverage.svg?branch=main)](https://codecov.io/gh/ivonnyssen/qhyccd-alpaca-rs)
[![Dependency status](https://deps.rs/repo/github/ivonnyssen/qhyccd-alpaca-rs/status.svg)](https://deps.rs/repo/github/ivonnyssen/qhyccd-alpaca-rs)

ASCOM Alpaca driver for QHYCCD cameras and filter wheels written in Rust.

## Current state

### Tested Operating Systems

- Debian 12 (Bookworm) amd64
- Ubunutu 22.04.3 LTS arm64 (Raspberry Pi 4)

### Tested Cameras

| Camera Model | ASCOM Validation Status |
| ------------ | ------ |
| QHY5III290C  | Passed |
| QHY5III178M  | Failed - exposure fails with error code 0x2001 |
| QHY178M     | Passed |
| QHY600M      | Passed |

### Tested Filter Wheels

| Filter Wheel Model | ASCOM Validation Status |
| ----------- | ------ |
| QHYCFW3L-SR | Passed |

### Tested Software

- SharpCap
- ACP
- NINA
- SGP

```toml
[dependencies]
qhyccd-alpaca-rs = "0.1.0"
```

## Installation

### Prerequisites

The driver relies on the QHYCCD SDK version 23.09.06 and libusb-1.0.0.
The instructions below are for installing from source.

#### Debian / Ubuntu / Raspberry Pi OS

##### Install libusb-1.0.0 and build tools

```bash
sudo apt-get install -y make cmake build-essential libusb-1.0-0-dev
```

##### Install QHYCCD SDK

```bash
wget https://www.qhyccd.com/file/repository/publish/SDK/230906/sdk_Arm64_23.09.06.tgz
tar xzvf sdk_Arm64_23.09.06.tgz 
cd sdk_Arm64_23.09.06/
sudo sh install.sh
```

##### Clone the repository

```bash
git clone https://github.com/ivonnyssen/qhyccd-alpaca-rs.git
cd qhyccd-alpaca-rs
cargo build --release
```

##### Run the driver

```bash
cd target/release
./qhyccd-alpaca-rs --log-level [debug | trace | info | warn | error] \
--port 8000
```

## Rust version requirements

qhyccd-alpaca-rs works with stable Rust. The minimum required Rust version is 1.70.0.

## Missing features

- LiveMode is not implemented
- The driver reports all readout modes, but if setting a readout mode uses a different
  resolution that the full camera resolution, that is not yet recognized by the driver.
- USB transport speed and mode are not implemented, it will use the camera defaults.

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
   <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

All contributions are welcome.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in qhyccd-alpaca-rs by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
