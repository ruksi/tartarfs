# 🥩 TartarFS

> The most delicious of filesystems, although a bit raw. 😋

```bash
sudo apt install libfuse3-dev
```

```bash
RUST_LOG=info cargo run $PWD/files.tar $PWD/out

# the tar file will be mounted until:
# - the process is killed
# - it receives an interrupt signal like Ctrl+C
# - the mount point is unmounted
umount out
```
