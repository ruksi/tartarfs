use crate::inode::Inode;
use crate::TartarFS;
use fuser::{ReplyEmpty, Request};
use libc::ENOENT;
use tracing::debug;

impl TartarFS {
    pub fn access_impl(
        &mut self,
        _req: &Request,
        inode_number: u64,
        _mask: i32,
        reply: ReplyEmpty,
    ) {
        debug!("access(inode={})", inode_number);
        if let Some(_) = self.inode_to_item.get(&Inode(inode_number)) {
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
}
