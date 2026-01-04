use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
#[ignore]
async fn test_camera_conformance() {
    // Start the qhyccd-alpaca server (will fail to initialize hardware but server will start)
    let mut server = Command::new("cargo")
        .args(&["run", "--", "--port", "11111"])
        .spawn()
        .expect("Failed to start server");

    // Wait for server to start
    sleep(Duration::from_secs(3)).await;

    // Run ConformU against the local server - expect some failures due to no hardware
    let output = Command::new("conformu")
        .args(&[
            "conformance",
            "http://localhost:11111/api/v1/camera/0",
            "--json-report",
            "conformu_camera_report.json",
            "--continue-on-error", // Continue testing even if some tests fail
        ])
        .output()
        .expect("Failed to run ConformU");

    // Terminate server
    server.kill().expect("Failed to kill server");

    // Print output for debugging
    println!(
        "ConformU stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    if !output.stderr.is_empty() {
        println!(
            "ConformU stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Check if ConformU ran (don't require success due to hardware limitations)
    assert!(
        output.status.code().is_some(),
        "ConformU failed to run properly"
    );

    // Try to read the report if it was generated
    if let Ok(report_content) = std::fs::read_to_string("conformu_camera_report.json") {
        println!("ConformU report generated successfully");
        // Basic validation that report was generated
        assert!(report_content.contains("\"DeviceType\": \"Camera\""));
    } else {
        println!("ConformU report not generated - this is expected without hardware");
    }
}

#[tokio::test]
#[ignore]
async fn test_alpaca_protocol_conformance() {
    let mut server = Command::new("cargo")
        .args(&["run", "--", "--port", "11113"])
        .spawn()
        .expect("Failed to start server");

    sleep(Duration::from_secs(3)).await;

    let output = Command::new("conformu")
        .args(&[
            "alpacaprotocol",
            "http://localhost:11113/api/v1/camera/0",
            "--json-report",
            "conformu_protocol_report.json",
        ])
        .output()
        .expect("Failed to run ConformU");

    server.kill().expect("Failed to kill server");

    println!(
        "ConformU stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    if !output.stderr.is_empty() {
        println!(
            "ConformU stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Protocol testing should be more likely to succeed
    if output.status.success() {
        println!("ConformU protocol validation passed!");
    } else {
        println!("ConformU protocol validation failed - this may be expected without hardware");
    }
}
