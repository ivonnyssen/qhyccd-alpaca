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

#[tokio::main]
async fn main() -> eyre::Result<std::convert::Infallible> {
    let args = Args::parse();
    let log_level: tracing::Level = args
        .log_level
        .unwrap_or_else(|| std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_owned()))
        .parse()
        .map_err(|e| eyre::eyre!("Invalid log level: {}", e))?;

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
