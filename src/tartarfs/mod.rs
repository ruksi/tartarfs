mod item;
mod operations;

use crate::inode::Inode;
use fuser::{Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry, Request};
use item::ArchiveItem;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tar::Archive;
use tracing::{debug, error, info};

pub struct TartarFS {
    archive_path: PathBuf,
    path_to_inode: HashMap<String, Inode>,
    inode_to_item: HashMap<Inode, ArchiveItem>,
    next_inode: Inode,
}

impl TartarFS {
    pub fn new(archive_path: String) -> Self {
        let root_inode = Inode(1);
        let mut fs = TartarFS {
            archive_path: PathBuf::from(&archive_path),
            path_to_inode: HashMap::new(),
            inode_to_item: HashMap::new(),
            next_inode: root_inode + 1,
        };

        let now = SystemTime::now();
        let root_item = ArchiveItem {
            name: "".into(),
            is_dir: true,
            size: 0,
            offset: 0,
            mode: 0o755,
            uid: 1000,
            gid: 1000,
            atime: now,
            mtime: now,
            ctime: now,
        };
        fs.inode_to_item.insert(root_inode, root_item);
        fs.path_to_inode.insert("".into(), root_inode);

        if let Ok(archive_file) = File::open(&archive_path) {
            let mut archive = Archive::new(archive_file);
            if let Ok(entries) = archive.entries() {
                for entry_result in entries {
                    if let Ok(entry) = entry_result {
                        if let Ok(path) = entry.path() {
                            let entry_path_text = path.to_string_lossy().to_string();
                            debug!("Found archive item: {}", entry_path_text);

                            let inode = fs.next_inode;
                            fs.next_inode += 1;

                            let header = entry.header();
                            let size = header.size().unwrap_or(0);
                            let is_dir = header.entry_type().is_dir();
                            let offset = entry.raw_file_position();
                            let mode = header.mode().unwrap_or(if is_dir { 0o755 } else { 0o644 });
                            let uid = header.uid().unwrap_or(1000);
                            let gid = header.gid().unwrap_or(1000);

                            let mtime = header
                                .mtime()
                                .ok()
                                .and_then(|t| {
                                    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(t))
                                })
                                .unwrap_or_else(SystemTime::now);

                            let item = ArchiveItem {
                                name: entry_path_text.clone(),
                                is_dir,
                                size,
                                offset,
                                mode: mode.try_into().unwrap(),
                                uid: uid.try_into().unwrap(),
                                gid: gid.try_into().unwrap(),
                                atime: mtime, // use mtime for all timestamps since that's what we got
                                mtime,
                                ctime: mtime,
                            };

                            fs.inode_to_item.insert(inode, item);
                            fs.path_to_inode.insert(entry_path_text.clone(), inode);

                            let entry_path = Path::new(&entry_path_text);
                            for ancestor in entry_path.ancestors().skip(1) {
                                let ancestor_text = ancestor.to_string_lossy().to_string();
                                if !fs.path_to_inode.contains_key(&ancestor_text)
                                    && !ancestor_text.is_empty()
                                {
                                    let parent_ino = fs.next_inode;
                                    fs.next_inode += 1;

                                    let now = SystemTime::now();
                                    let parent_item = ArchiveItem {
                                        name: ancestor_text.clone(),
                                        is_dir: true,
                                        size: 0,
                                        offset: 0,
                                        mode: 0o755,
                                        uid: 1000,
                                        gid: 1000,
                                        atime: now,
                                        mtime: now,
                                        ctime: now,
                                    };

                                    fs.inode_to_item.insert(parent_ino, parent_item);
                                    fs.path_to_inode.insert(ancestor_text, parent_ino);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            error!("Failed to open archive: {}", archive_path);
        }

        info!(
            "Initialized filesystem with {} items",
            fs.inode_to_item.len()
        );
        fs
    }
}

impl Filesystem for TartarFS {
    #[rustfmt::skip]
    fn lookup( &mut self, req: &Request, parent_ino: u64, name: &std::ffi::OsStr, reply: ReplyEntry ) {
        self.lookup_impl(req, parent_ino, name, reply);
    }

    fn getattr(&mut self, req: &Request, ino: u64, fh: Option<u64>, reply: ReplyAttr) {
        self.getattr_impl(req, ino, fh, reply);
    }

    #[rustfmt::skip]
    fn read(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, size: u32, flags: i32, lock: Option<u64>, reply: ReplyData) {
        self.read_impl(req, ino, fh, offset, size, flags, lock, reply);
    }

    #[rustfmt::skip]
    fn readdir(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, reply: ReplyDirectory) {
        self.readdir_impl(req, ino, fh, offset, reply);
    }

    fn access(&mut self, req: &Request, ino: u64, mask: i32, reply: ReplyEmpty) {
        self.access_impl(req, ino, mask, reply);
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::TestSetup;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use tempfile::TempDir;

    #[test]
    fn test_basics() -> std::io::Result<()> {
        let source_dir = TempDir::new()?;
        fs::write(source_dir.path().join("greeting.txt"), "Hello from file!")?;
        fs::create_dir_all(source_dir.path().join("alphabet"))?;
        fs::write(source_dir.path().join("alphabet/a.txt"), "Nested content")?;
        fs::create_dir_all(source_dir.path().join("alphabet/greek"))?;
        fs::write(source_dir.path().join("alphabet/greek/alpha.txt"), "αααα")?;
        fs::set_permissions(
            source_dir.path().join("greeting.txt"),
            fs::Permissions::from_mode(0o644),
        )?;
        fs::set_permissions(
            source_dir.path().join("alphabet"),
            fs::Permissions::from_mode(0o755),
        )?;
        fs::set_permissions(
            source_dir.path().join("alphabet/a.txt"),
            fs::Permissions::from_mode(0o600),
        )?;
        fs::set_permissions(
            source_dir.path().join("alphabet/greek"),
            fs::Permissions::from_mode(0o750),
        )?;
        fs::set_permissions(
            source_dir.path().join("alphabet/greek/alpha.txt"),
            fs::Permissions::from_mode(0o644),
        )?;
        let mounted = TestSetup::from_dir(source_dir.path())?;

        mounted.assert_is_file("greeting.txt", Some(0o644), Some("Hello from file!"));
        mounted.assert_is_dir("alphabet", Some(0o755));
        mounted.assert_is_file("alphabet/a.txt", Some(0o600), Some("Nested content"));
        mounted.assert_is_dir("alphabet/greek", Some(0o750));
        mounted.assert_is_file("alphabet/greek/alpha.txt", Some(0o644), Some("αααα"));

        // Verify timestamps are preserved
        let metadata = fs::metadata(source_dir.path().join("greeting.txt"))?;
        let mounted_metadata = fs::metadata(mounted.mount_path.join("greeting.txt"))?;

        assert_eq!(
            metadata.mtime(),
            mounted_metadata.mtime(),
            "mtime should be preserved"
        );

        Ok(())
    }
}
