use clap::Parser;
use fuser::MountOption;
use std::path::Path;
use tartarfs::TartarFS;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    archive_path: String,
    mount_path: String,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_timer(fmt::time::UtcTime::rfc_3339())
        .init();

    let args = Args::parse();
    info!("Archive: {}", args.archive_path);
    info!("Mount: {}", args.mount_path);

    let filesystem = TartarFS::new(args.archive_path);

    let mount_path = Path::new(&args.mount_path);
    if !mount_path.exists() {
        std::fs::create_dir_all(mount_path).expect("Failed to create mount directory");
    }

    let options = vec![MountOption::FSName("tartarfs".into()), MountOption::RO];

    fuser::mount2(filesystem, &mount_path, &options)
        .unwrap_or_else(|e| panic!("Failed to mount: {}", e));
}
