# Linux Comparison Guidance

Use `/root/linux` as the primary baseline for behavioral semantics, dispatch shape, and ownership boundaries.

## What to compare

For the changed operation, answer these questions:

1. Which layer receives the operation first?
2. Which interface or callback hands control to the concrete filesystem?
3. Which layer owns mutable state and policy?
4. Does Linux solve this generically through an interface, or specifically inside the filesystem?

## Evidence categories

### Direct match
Use this when Linux has an obviously analogous callback, dispatch path, or ownership location.

### Behavioral match
Use this when Linux-visible semantics align, even if Asterinas uses different names or structure.

### Inference
Use this when the architectural conclusion follows from the Linux code path and ownership boundaries, but there is no exact one-to-one implementation match.

When using inference, explicitly say:
"this is an inference from the code path".

## Suggested searches

```bash
git diff --stat
git diff -- kernel/src/fs kernel/src/syscall test
rg -n "downcast_ref|downcast_mut|Any|xfstests|ioctl|special-case|ext2" kernel/src
rg -n "trait .*Inode|trait .*File|fn ioctl|fn metadata|fn set_|fn flags" kernel/src/fs
rg -n "vfs_ioctl|unlocked_ioctl|compat_ioctl|ioctl" /root/linux/fs /root/linux/include
rg -n "fallocate|truncate|setattr|getattr" /root/linux/fs /root/linux/include
rg -n "ext2.*ioctl|ioctl.*ext2|->ioctl|->unlocked_ioctl" /root/linux/fs /root/linux/include
