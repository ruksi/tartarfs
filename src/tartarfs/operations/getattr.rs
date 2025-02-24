use crate::inode::Inode;
use crate::TartarFS;
use fuser::{ReplyAttr, Request};
use libc::ENOENT;
use std::time::Duration;
use tracing::debug;

impl TartarFS {
    pub fn getattr_impl(
        &mut self,
        _req: &Request,
        inode_number: u64,
        _fh: Option<u64>,
        reply: ReplyAttr,
    ) {
        debug!("getattr(inode={})", inode_number);
        let Some(item) = self.inode_to_item.get(&Inode(inode_number)) else {
            return reply.error(ENOENT);
        };

        let ttl = Duration::from_secs(1);
        reply.attr(&ttl, &item.get_file_attributes(Inode(inode_number)));
    }
}
