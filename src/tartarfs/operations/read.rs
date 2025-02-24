use crate::inode::Inode;
use crate::TartarFS;
use fuser::{ReplyData, Request};
use libc::ENOENT;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

impl TartarFS {
    pub fn read_impl(
        &mut self,
        _req: &Request,
        inode_number: u64,
        _fh: u64,
        _offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        let Some(item) = self.inode_to_item.get(&Inode(inode_number)) else {
            return reply.error(ENOENT);
        };

        let Ok(mut archive_file) = File::open(&self.archive_path) else {
            return reply.error(ENOENT);
        };

        let Ok(_) = archive_file.seek(SeekFrom::Start(item.offset)) else {
            return reply.error(ENOENT);
        };

        let mut buffer = vec![0; size as usize];
        let Ok(n) = archive_file.read(&mut buffer) else {
            return reply.error(ENOENT);
        };

        reply.data(&buffer[..n]);
    }
}
