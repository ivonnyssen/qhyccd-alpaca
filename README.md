# qhyccd-alpaca-rs

[![Crates.io](https://img.shields.io/crates/v/qhyccd-alpaca-rs.svg)](https://crates.io/crates/qhyccd-alpaca-rs)
[![Documentation](https://docs.rs/qhyccd-alpaca-rs/badge.svg)](https://docs.rs/qhyccd-alpaca-rs/)
[![Codecov](https://codecov.io/github/ivonnyssen/qhyccd-alpaca-rs/coverage.svg?branch=main)](https://codecov.io/gh/ivonnyssen/qhyccd-alpaca-rs)
[![Dependency status](https://deps.rs/repo/github/ivonnyssen/qhyccd-alpaca-rs/status.svg)](https://deps.rs/repo/github/ivonnyssen/qhyccd-alpaca-rs)

ASCOM Alpaca driver for QHYCCD cameras and filter wheels written in Rust.

## Current state

### Tested Operating Systems

- Debian 12 (Bookworm) amd64

### Tested Cameras

- QHY178M
- QHY5III290C
- QHY5III178M
- QHY600M

### Tested Filter Wheels

- QHYCFW3L-SR

```toml
[dependencies]
qhyccd-alpaca-rs = "0.1.0"
```

## Rust version requirements

qhyccd-alpaca-rs works with stable Rust. The minimum required Rust version is 1.70.0.

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
