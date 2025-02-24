use crate::inode::Inode;
use crate::TartarFS;
use fuser::{FileType, ReplyDirectory, Request};
use libc::ENOENT;
use std::ffi::OsString;
use tracing::debug;

impl TartarFS {
    pub fn readdir_impl(
        &mut self,
        _req: &Request,
        inode_number: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        debug!("readdir(inode={}, offset={})", inode_number, offset);
        if let Some(dir_entry) = self.inode_to_item.get(&Inode(inode_number)) {
            if offset == 0 {
                let mut entries = vec![
                    (inode_number, FileType::Directory, ".".into()),
                    (inode_number, FileType::Directory, "..".into()),
                ];

                let dir_path_text = &dir_entry.name;
                for (child_inode_number, entry) in &self.inode_to_item {
                    if entry.name.starts_with(dir_path_text) {
                        let remaining = if dir_path_text.is_empty() {
                            &entry.name
                        } else {
                            entry
                                .name
                                .strip_prefix(&format!("{}/", dir_path_text))
                                .unwrap_or(&entry.name)
                        };

                        if !remaining.is_empty() && !remaining.contains('/') {
                            entries.push((
                                child_inode_number.0,
                                if entry.is_dir {
                                    FileType::Directory
                                } else {
                                    FileType::RegularFile
                                },
                                OsString::from(remaining),
                            ));
                        }
                    }
                }

                for (i, (cur_ino, kind, name)) in entries.into_iter().enumerate() {
                    let _ = reply.add(cur_ino, (i + 1) as i64, kind, name);
                }
            }
            reply.ok();
            return;
        }
        reply.error(ENOENT);
    }
}
