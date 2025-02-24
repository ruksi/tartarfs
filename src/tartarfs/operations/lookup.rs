use crate::inode::Inode;
use crate::TartarFS;
use fuser::{ReplyEntry, Request};
use libc::ENOENT;
use std::time::Duration;
use tracing::debug;

impl TartarFS {
    pub fn lookup_impl(
        &mut self,
        _req: &Request,
        parent_inode_number: u64,
        name: &std::ffi::OsStr,
        reply: ReplyEntry,
    ) {
        debug!("lookup(parent={}, name={:?})", parent_inode_number, name);
        let Some(parent_item) = self.inode_to_item.get(&Inode(parent_inode_number)) else {
            return reply.error(ENOENT);
        };

        let parent_path = &parent_item.name;
        let lookup_path = match parent_path {
            path if path.is_empty() => name.to_string_lossy().into_owned(),
            path => format!("{path}/{}", name.to_string_lossy()),
        };

        let Some(&inode) = self.path_to_inode.get(&lookup_path) else {
            return reply.error(ENOENT);
        };

        let Some(item) = self.inode_to_item.get(&inode) else {
            return reply.error(ENOENT);
        };

        let ttl = Duration::from_secs(1);
        reply.entry(&ttl, &item.get_file_attributes(inode), 0);
    }
}
