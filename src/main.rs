use clap::Parser;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_timer(fmt::time::UtcTime::rfc_3339())
        .init();

    let args = tartarfs::cli::Args::parse();
    info!("Archive: {}", args.archive_path);
    info!("Mount: {}", args.mount_path);

    tartarfs::cli::run(args).unwrap_or_else(|e| panic!("failed to mount: {}", e));
}
