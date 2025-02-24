use crate::TartarFS;
use clap::Parser;
use fuser::MountOption;
use std::fs::metadata;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    pub archive_path: String,
    pub mount_path: String,
}

pub fn run(args: Args) -> std::io::Result<()> {
    let filesystem = TartarFS::new(args.archive_path);

    let mount_path = Path::new(&args.mount_path);
    if !mount_path.exists() {
        std::fs::create_dir_all(mount_path)?;
    }

    let options = vec![MountOption::FSName("tartarfs".into()), MountOption::RO];

    // the mount will be unmounted when the session is dropped
    let session = fuser::spawn_mount2(filesystem, &mount_path, &options)?;
    let session = Arc::new(Mutex::new(Some(session)));
    let session_for_handler = Arc::clone(&session);

    ctrlc::set_handler(move || {
        info!("Interrupt received, unmounting...");
        // drop the session for clean unmounting
        if let Ok(mut session) = session_for_handler.lock() {
            *session = None;
        }
        std::process::exit(0);
    })
    .expect("Error setting interrupt handler");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));

        // return if the mount was externally unmounted
        if let Ok(session) = session.lock() {
            if session.is_none() {
                info!("Mount session ended");
                return Ok(());
            }

            if !mount_path.exists() {
                info!("Mount point no longer exists");
                return Ok(());
            }

            if !is_mounted(&mount_path) {
                info!("Mount point no longer mounted");
                return Ok(());
            }
        }
    }
}

fn is_mounted(path: &Path) -> bool {
    let path_meta = metadata(path);
    let parent_meta = metadata(path.parent().unwrap_or(Path::new("/")));
    if let (Ok(path_meta), Ok(parent_meta)) = (path_meta, parent_meta) {
        return path_meta.dev() != parent_meta.dev();
    }
    false
}
