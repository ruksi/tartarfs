use crate::inode::Inode;
use fuser::{FileAttr, FileType};
use std::time::SystemTime;

pub struct ArchiveItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub offset: u64,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub atime: SystemTime,
    pub mtime: SystemTime,
    pub ctime: SystemTime,
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
            atime: self.atime,
            mtime: self.mtime,
            ctime: self.ctime,
            crtime: self.ctime,
            kind,
            perm: self.mode & 0o777,
            nlink: 1,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }
}
