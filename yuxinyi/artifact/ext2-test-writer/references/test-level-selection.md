# Test Level Selection

Choose the lowest test level that can prove the target semantics without losing confidence.

## Use `#[ktest]`

Pick `#[ktest]` when the obligation is primarily local to kernel logic:

* helper or validator behavior
* internal state transition
* local lock-free semantic invariant
* pure mapping from input state to return value
* local errno translation that does not require a full userspace path
* edge conditions that are hard to force reliably from userspace

Typical examples:

* directory cookie progression logic
* dentry cache invalidation behavior
* helper rejection of invalid overwrite cases
* page-cache bookkeeping invariants

## Use initramfs integration tests

Pick integration tests when the obligation is syscall-visible or environment-visible:

* `open`, `read`, `write`, `rename`, `unlink`, `rmdir`, `link`, `stat`, `ioctl`, `mount`
* path lookup and namespace visibility
* multi-fd behavior
* remount or persistence checks
* process-visible errno and output
* interactions between VFS and ext2 that must be observed through public behavior

Typical examples:

* `rename` errno and visible tree shape
* `rmdir` on non-empty directory
* remount persistence of data or metadata
* cross-mount rename rejection

## Use both

Split coverage across both levels when:

* one internal invariant explains the bug, but the public behavior also needs to be pinned
* a unit test can cheaply cover several hard-to-trigger branches, while one integration test confirms the public contract
* persistence or namespace visibility must be checked externally, but preparation logic is easier to pin internally

## Do not force the wrong level

Do not use `#[ktest]` to fake a syscall-visible semantic contract when a userspace integration test is cheap and clearer.

Do not use integration tests for a tiny internal invariant when the public harness would be slow, brittle, or indirect.
