use ascom_alpaca::api::Camera;
use ascom_alpaca::test::run_conformu_tests;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::{sleep, timeout};
use tracing_subscriber::{EnvFilter, fmt};

fn get_random_port() -> u16 {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener
        .local_addr()
        .expect("Failed to get local addr")
        .port();
    drop(listener);
    port
}

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

    let port = get_random_port();

    // Start qhyccd-alpaca server with minimal logging
    let mut child = Command::new("cargo")
        .args([
            "run",
            "--",
            "--port",
            &port.to_string(),
            "--log-level",
            "error",
        ])
        .stdout(std::process::Stdio::null()) // Suppress stdout
        .stderr(std::process::Stdio::null()) // Suppress stderr
        .spawn()?;

    // Wait for service to be ready with health check
    let client = reqwest::Client::new();
    let mut ready = false;
    for _ in 0..30 {
        sleep(Duration::from_secs(1)).await;
        if let Ok(Ok(resp)) = timeout(
            Duration::from_secs(2),
            client
                .get(format!(
                    "http://localhost:{}/management/v1/description",
                    port
                ))
                .send(),
        )
        .await
        {
            if resp.status().is_success() {
                ready = true;
                break;
            }
        }
    }

    if !ready {
        let _ = child.kill().await;
        let _ = child.wait().await;
        return Err("Service failed to start within 30 seconds".into());
    }

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

    // Cleanup
    let _ = child.kill().await;
    let _ = child.wait().await;

    result?;
    Ok(())
}

// #[tokio::test]
// #[ignore] // Run with --ignored flag since it requires ConformU installation
// async fn conformu_filterwheel_compliance_tests() -> Result<(), Box<dyn std::error::Error>> {
//     let _ = fmt()
//         .with_env_filter(
//             EnvFilter::try_from_default_env()
//                 .unwrap_or_else(|_| EnvFilter::new("ascom_alpaca::conformu=trace,info")),
//         )
//         .with_test_writer()
//         .try_init();
//
//     let port = get_random_port();
//
//     // Start qhyccd-alpaca server
//     let mut child = Command::new("cargo")
//         .args([
//             "run",
//             "--",
//             "--port",
//             &port.to_string(),
//             "--log-level",
//             "debug",
//         ])
//         .spawn()?;
//
//     // Wait for service to be ready
//     let client = reqwest::Client::new();
//     let mut ready = false;
//     for _ in 0..30 {
//         sleep(Duration::from_secs(1)).await;
//         if let Ok(Ok(resp)) = timeout(
//             Duration::from_secs(2),
//             client
//                 .get(format!(
//                     "http://localhost:{}/management/v1/description",
//                     port
//                 ))
//                 .send(),
//         )
//         .await
//         {
//             if resp.status().is_success() {
//                 ready = true;
//                 break;
//             }
//         }
//     }
//
//     if !ready {
//         let _ = child.kill().await;
//         let _ = child.wait().await;
//         return Err("Service failed to start within 30 seconds".into());
//     }
//
//     println!("::group::ConformU FilterWheel Compliance Test Results");
//     println!("Running ASCOM Alpaca filter wheel compliance tests...");
//
//     // Run ConformU tests for filter wheel device 0
//     let result = run_conformu_tests::<dyn ascom_alpaca::api::FilterWheel>(
//         &format!("http://localhost:{}", port),
//         0,
//     )
//     .await;
//
//     match &result {
//         Ok(_) => {
//             println!("✅ ConformU filter wheel compliance tests PASSED");
//             println!("All ASCOM Alpaca filter wheel compliance requirements met");
//         }
//         Err(e) => {
//             println!("❌ ConformU filter wheel compliance tests FAILED");
//             println!("Error: {}", e);
//         }
//     }
//
//     println!("::endgroup::");
//
//     // Cleanup
//     let _ = child.kill().await;
//     let _ = child.wait().await;
//
//     result?;
//     Ok(())
// }
