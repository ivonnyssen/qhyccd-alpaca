# Integration Tests

## ConformU Compliance Tests

The `conformu_integration.rs` file contains integration tests that use ConformU to validate ASCOM Alpaca compliance.

### Prerequisites

1. Install ConformU from the ASCOM Initiative
2. Ensure the qhyccd-alpaca server can start (hardware not required for basic protocol testing)

### Running the Tests

```bash
# Run ConformU integration tests (requires ConformU installation)
cargo test --test conformu_integration -- --ignored

# Run specific test
cargo test --test conformu_integration conformu_camera_compliance_tests -- --ignored
cargo test --test conformu_integration conformu_filterwheel_compliance_tests -- --ignored
```

### Test Behavior

- Tests start the qhyccd-alpaca server on a random port
- Wait for the server to be ready using health checks
- Run ConformU compliance tests against the running server
- Clean up by terminating the server process

The tests use the `ascom_alpaca::test::run_conformu_tests` function which provides structured ConformU integration for Rust ASCOM Alpaca drivers.
