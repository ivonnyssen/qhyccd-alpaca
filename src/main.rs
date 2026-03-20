use clap::Parser;
use qhyccd_alpaca::ServerBuilder;

/// ASCOM Alpaca server for QHYCCD cameras and filter wheels
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "8000")]
    port: u16,

    /// valid values: trace, debug, info, warn, error
    #[arg(short, long, default_value = "info")]
    log_level: Option<String>,
}

fn resolve_log_level(cli_arg: Option<String>) -> eyre::Result<tracing::Level> {
    cli_arg
        .unwrap_or_else(|| std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_owned()))
        .parse()
        .map_err(|e| eyre::eyre!("Invalid log level: {}", e))
}

#[tokio::main]
async fn main() -> eyre::Result<std::convert::Infallible> {
    let args = Args::parse();
    let log_level = resolve_log_level(args.log_level)?;

    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt().with_max_level(log_level).finish(),
    )?;

    ServerBuilder::new()
        .with_port(args.port)
        .build()
        .await?
        .start()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_log_level_with_explicit_value() {
        assert_eq!(
            resolve_log_level(Some("debug".to_owned())).unwrap(),
            tracing::Level::DEBUG
        );
    }

    #[test]
    fn resolve_log_level_all_valid_levels() {
        for (input, expected) in [
            ("trace", tracing::Level::TRACE),
            ("debug", tracing::Level::DEBUG),
            ("info", tracing::Level::INFO),
            ("warn", tracing::Level::WARN),
            ("error", tracing::Level::ERROR),
        ] {
            assert_eq!(
                resolve_log_level(Some(input.to_owned())).unwrap(),
                expected,
                "failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn resolve_log_level_case_insensitive() {
        assert_eq!(
            resolve_log_level(Some("DEBUG".to_owned())).unwrap(),
            tracing::Level::DEBUG
        );
        assert_eq!(
            resolve_log_level(Some("Info".to_owned())).unwrap(),
            tracing::Level::INFO
        );
    }

    #[test]
    fn resolve_log_level_invalid() {
        let result = resolve_log_level(Some("invalid".to_owned()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid log level"));
    }

}
