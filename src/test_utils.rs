use super::*;
use fuser::MountOption;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;
use std::{fs, mem};
use tempfile::TempDir;
use walkdir::WalkDir;

#[allow(dead_code)]
pub struct TestSetup {
    test_dir: TempDir,            // the test directory to be archived
    archive_path: PathBuf,        // the path to the archive to be made
    mount_path: PathBuf,          // the path to the mount point
    mount_handle: JoinHandle<()>, // the handle to wait for the unmounting
}

impl TestSetup {
    /// Create a new test setup from a directory files to be archived.
    /// This creates the tar and mounts it which you can then test using
    /// the various `assert` methods.
    pub fn from_dir<P: AsRef<Path>>(source_dir: P) -> std::io::Result<Self> {
        let test_dir = TempDir::new()?;
        let archive_path = test_dir.path().join("test.tar");

        let tar_file = fs::File::create(&archive_path)?;
        let mut builder = tar::Builder::new(tar_file);

        let source_dir = source_dir.as_ref();
        for entry in WalkDir::new(source_dir) {
            let entry = entry?;
            let path = entry.path();
            let relative_path = path
                .strip_prefix(source_dir)
                .expect("Path must be under source directory");

            if relative_path.as_os_str().is_empty() {
                continue; // skip root, we don't want to archive _that_
            }

            if path.is_file() {
                builder.append_path_with_name(path, relative_path)?;
            } else if path.is_dir() {
                builder.append_dir(relative_path, path)?;
            }
        }
        builder.finish()?;

        let mount_path = test_dir.path().join("mount");
        fs::create_dir_all(&mount_path)?;

        let filesystem = TartarFS::new(archive_path.to_string_lossy().to_string());
        let mount_path_clone = mount_path.clone();
        let mount_handle = spawn(move || {
            fuser::mount2(
                filesystem,
                &mount_path_clone,
                &[MountOption::RO, MountOption::FSName("tartarfs".to_string())],
            )
            .unwrap();
        });

        // give the mount time to start
        sleep(Duration::from_millis(100));

        Ok(Self {
            test_dir,
            archive_path,
            mount_path,
            mount_handle,
        })
    }

    /// Assert that a path is a file in the mounted filesystem with expected contents and mode.
    pub fn assert_is_file(
        &self,
        path: &str,
        expected_mode: Option<u32>,
        expected_content: Option<&str>,
    ) {
        let full_path = self.mount_path.join(path);
        assert!(
            full_path.is_file(),
            "Path {} should be a file in mounted filesystem",
            path
        );

        if let Some(expected) = expected_content {
            let content = fs::read_to_string(&full_path)
                .unwrap_or_else(|e| panic!("Failed to read file {}: {}", path, e));
            assert_eq!(
                content, expected,
                "File {} content mismatch.\nExpected: {}\nActual: {}",
                path, expected, content
            );
        }

        if let Some(expected_mode) = expected_mode {
            let metadata = fs::metadata(&full_path)
                .unwrap_or_else(|e| panic!("Failed to get metadata for {}: {}", path, e));
            let mode = metadata.mode() & 0o777;
            assert_eq!(
                mode, expected_mode,
                "File {} mode mismatch.\nExpected: {:o}\nActual: {:o}",
                path, expected_mode, mode
            );
        }
    }

    /// Assert that a path is a directory with expected mode
    pub fn assert_is_dir(&self, path: &str, expected_mode: Option<u32>) {
        let full_path = self.mount_path.join(path);
        assert!(
            full_path.is_dir(),
            "Path {} should be a directory in mounted filesystem",
            path
        );

        if let Some(expected_mode) = expected_mode {
            let metadata = fs::metadata(&full_path)
                .unwrap_or_else(|e| panic!("Failed to get metadata for {}: {}", path, e));
            let mode = metadata.mode() & 0o777;
            assert_eq!(
                mode, expected_mode,
                "Directory {} mode mismatch.\nExpected: {:o}\nActual: {:o}",
                path, expected_mode, mode
            );
        }
    }
}

impl Drop for TestSetup {
    fn drop(&mut self) {
        let status = std::process::Command::new("fusermount")
            .arg("-u")
            .arg(&self.mount_path)
            .status()
            .expect("Failed to unmount filesystem");
        assert!(status.success(), "Failed to unmount filesystem");

        // yoink the real handle and replace with a placeholder
        // so we can do a `join` (which wants ownership); we are in drop
        // so it's not like the struct needs the field anymore
        let mount_handle = mem::replace(&mut self.mount_handle, spawn(|| {}));
        mount_handle.join().unwrap();
    }
}
