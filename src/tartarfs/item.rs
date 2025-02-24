use crate::inode::Inode;
use fuser::{FileAttr, FileType};
use std::time::SystemTime;

pub struct ArchiveItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub offset: u64,
}

impl ArchiveItem {
    pub fn get_file_attributes(&self, inode: Inode) -> FileAttr {
        let kind = if self.is_dir {
            FileType::Directory
        } else {
            FileType::RegularFile
        };

        FileAttr {
            ino: inode.0,
            size: self.size,
            blocks: 1,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind,
            perm: if self.is_dir { 0o755 } else { 0o644 },
            nlink: 1,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }
}
