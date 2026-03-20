use ascom_alpaca::api::Camera;
use ascom_alpaca::test::run_conformu_tests;
use qhyccd_alpaca::ServerBuilder;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::test]
#[ignore] // Run with --ignored flag since it requires ConformU installation
async fn conformu_camera_compliance_tests() -> Result<(), Box<dyn std::error::Error>> {
    let _ = fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("ascom_alpaca::conformu=trace,info")),
        )
        .with_test_writer()
        .try_init();

    // Build server with port 0 (OS assigns a free port)
    let bound = ServerBuilder::new().build().await?;
    let port = bound.listen_addr().port();

    // Start server in background
    tokio::spawn(async move {
        let _ = bound.start().await;
    });

    println!("::group::ConformU Camera Compliance Test Results");
    println!("Running ASCOM Alpaca camera compliance tests...");

    // Run ConformU tests for camera device 0
    let result = run_conformu_tests::<dyn Camera>(&format!("http://localhost:{}", port), 0).await;

    match &result {
        Ok(_) => {
            println!("✅ ConformU camera compliance tests PASSED");
            println!("All ASCOM Alpaca camera compliance requirements met");
        }
        Err(e) => {
            println!("❌ ConformU camera compliance tests FAILED");
            println!("Error: {}", e);
        }
    }

    println!("::endgroup::");

    result?;
    Ok(())
}

#[tokio::test]
#[ignore] // Run with --ignored flag since it requires ConformU installation
async fn conformu_filterwheel_compliance_tests() -> Result<(), Box<dyn std::error::Error>> {
    let _ = fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("ascom_alpaca::conformu=trace,info")),
        )
        .with_test_writer()
        .try_init();

    // Build server with port 0 (OS assigns a free port)
    let bound = ServerBuilder::new().build().await?;
    let port = bound.listen_addr().port();

    // Start server in background
    tokio::spawn(async move {
        let _ = bound.start().await;
    });

    println!("::group::ConformU FilterWheel Compliance Test Results");
    println!("Running ASCOM Alpaca filter wheel compliance tests...");

    // Run ConformU tests for filter wheel device 0
    let result = run_conformu_tests::<dyn ascom_alpaca::api::FilterWheel>(
        &format!("http://localhost:{}", port),
        0,
    )
    .await;

    match &result {
        Ok(_) => {
            println!("✅ ConformU filter wheel compliance tests PASSED");
            println!("All ASCOM Alpaca filter wheel compliance requirements met");
        }
        Err(e) => {
            println!("❌ ConformU filter wheel compliance tests FAILED");
            println!("Error: {}", e);
        }
    }

    println!("::endgroup::");

    result?;
    Ok(())
}
