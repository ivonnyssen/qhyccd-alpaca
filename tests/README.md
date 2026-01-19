# Integration Tests

## ConformU Compliance Tests

The `conformu_integration.rs` file contains integration tests that use ConformU to validate ASCOM Alpaca compliance.

### Prerequisites

1. Install ConformU from the ASCOM Initiative
2. Ensure the qhyccd-alpaca server can start (hardware not required for basic protocol testing)

### Running the Tests

```bash
# Run ConformU integration tests with simulated hardware (no real camera required)
cargo test --test conformu_integration --features simulation -- --ignored

# Run ConformU integration tests with real hardware (requires connected QHYCCD camera)
cargo test --test conformu_integration -- --ignored

# Run specific test with simulation
cargo test --test conformu_integration conformu_camera_compliance_tests --features simulation -- --ignored
cargo test --test conformu_integration conformu_filterwheel_compliance_tests --features simulation -- --ignored

# Run specific test with real hardware
cargo test --test conformu_integration conformu_camera_compliance_tests -- --ignored
cargo test --test conformu_integration conformu_filterwheel_compliance_tests -- --ignored
```

### Test Behavior

- Tests start the qhyccd-alpaca server on a random port
- Wait for the server to be ready using health checks
- Run ConformU compliance tests against the running server
- Clean up by terminating the server process

The tests use the `ascom_alpaca::test::run_conformu_tests` function which provides structured ConformU integration for Rust ASCOM Alpaca drivers.
