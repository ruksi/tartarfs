# 🥩 TartarFS

> The most delicious of filesystems, although a bit raw. 😋

TartarFS is a simple FUSE filesystem that allows you to navigate [tar](https://en.wikipedia.org/wiki/Tar_(computing))
file's contents as if it were a directory. Currently, it only supports read operations.

Dependencies:

```bash
sudo apt install libfuse3-dev
```

Usage:

```bash
RUST_LOG=info cargo run $PWD/files.tar $PWD/out

# the tar file will be mounted until the process receives an interrupt signal like Ctrl+C

# manually unmounting the mount point also kills the process:
fusermount -u out
# or
fusermount3 -u out
# or
umount out
```
