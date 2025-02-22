use clap::Parser;
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, Request,
};
use libc::ENOENT;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::{Add, AddAssign};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tar::Archive;
use tracing::{debug, error, info};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    archive_path: String,
    mount_path: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct Inode(u64);

impl Add<u64> for Inode {
    type Output = Self;

    fn add(self, other: u64) -> Self {
        Self(self.0 + other)
    }
}
impl AddAssign<u64> for Inode {
    fn add_assign(&mut self, other: u64) {
        self.0 += other;
    }
}

struct TartarFS {
    archive_path: PathBuf,
    path_to_inode: HashMap<String, Inode>,
    inode_to_item: HashMap<Inode, ArchiveItem>,
    next_inode: Inode,
}

struct ArchiveItem {
    name: String,
    is_dir: bool,
    size: u64,
    offset: u64,
}

impl TartarFS {
    fn new(archive_path: String) -> Self {
        let root_inode = Inode(1);
        let mut fs = TartarFS {
            archive_path: PathBuf::from(&archive_path),
            path_to_inode: HashMap::new(),
            inode_to_item: HashMap::new(),
            next_inode: root_inode + 1,
        };

        let root_item = ArchiveItem {
            name: "".into(),
            is_dir: true,
            size: 0,
            offset: 0,
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
                            let item = ArchiveItem {
                                name: entry_path_text.clone(),
                                is_dir,
                                size,
                                offset,
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

                                    let parent_item = ArchiveItem {
                                        name: ancestor_text.clone(),
                                        is_dir: true,
                                        size: 0,
                                        offset: 0,
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
            error!("Failed to open archive file: {}", archive_path);
        }

        info!("Initialized with {} entries", fs.inode_to_item.len());
        fs
    }

    fn get_entry_attr(&self, item: &ArchiveItem, inode: Inode) -> FileAttr {
        let kind = if item.is_dir {
            FileType::Directory
        } else {
            FileType::RegularFile
        };

        FileAttr {
            ino: inode.0,
            size: item.size,
            blocks: 1,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind,
            perm: if item.is_dir { 0o755 } else { 0o644 },
            nlink: 1,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            blksize: 512,
            flags: 0,
        }
    }
}

impl Filesystem for TartarFS {
    fn access(&mut self, _req: &Request, inode_number: u64, _mask: i32, reply: ReplyEmpty) {
        debug!("access(inode={})", inode_number);
        if let Some(_) = self.inode_to_item.get(&Inode(inode_number)) {
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn lookup(
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

        let Some(entry) = self.inode_to_item.get(&inode) else {
            return reply.error(ENOENT);
        };

        let ttl = Duration::from_secs(1);
        reply.entry(&ttl, &self.get_entry_attr(entry, inode), 0);
    }

    fn getattr(&mut self, _req: &Request, inode_number: u64, _fh: Option<u64>, reply: ReplyAttr) {
        debug!("getattr(inode={})", inode_number);
        let Some(item) = self.inode_to_item.get(&Inode(inode_number)) else {
            return reply.error(ENOENT);
        };

        let ttl = Duration::from_secs(1);
        reply.attr(&ttl, &self.get_entry_attr(item, Inode(inode_number)));
    }

    fn readdir(
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

    fn read(
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

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_timer(fmt::time::UtcTime::rfc_3339())
        .init();

    let args = Args::parse();
    info!("Archive: {}", args.archive_path);
    info!("Mount: {}", args.mount_path);

    let filesystem = TartarFS::new(args.archive_path);

    let mount_path = Path::new(&args.mount_path);
    if !mount_path.exists() {
        std::fs::create_dir_all(mount_path).expect("Failed to create mount directory");
    }

    let options = vec![MountOption::FSName("tartarfs".into()), MountOption::RO];

    fuser::mount2(filesystem, &mount_path, &options)
        .unwrap_or_else(|e| panic!("Failed to mount: {}", e));
}
