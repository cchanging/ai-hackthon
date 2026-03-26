# `Revalidate Mechanism for Existing File Systems` Change Review

## Summary

- Review slug: `revalidate-mechanism-existing-file-systems`
- Review date: `2026-03-25`
- Reviewer: `Codex`
- Input mode: `git-range`
- Commit range: `4491ff504c60c3f7715ab8f91041a3c4d7f351f9..41c380cfb0d4badcb859784417f4813fdf9f9383`
- Change under review: `Implement revalidate mechanism to some existing file systems`
- Review summary: The diff replaces several `is_dentry_cacheable()` decisions with explicit positive and negative dentry revalidation hooks, and rewrites procfs directory enumeration to synthesize entries on demand instead of reusing cached child inodes.

## Changed Paths

- `kernel/src/fs/fs_impls/cgroupfs/inode.rs`
- `kernel/src/fs/fs_impls/devpts/mod.rs`
- `kernel/src/fs/fs_impls/devpts/ptmx.rs`
- `kernel/src/fs/fs_impls/devpts/slave.rs`
- `kernel/src/fs/fs_impls/exfat/inode.rs`
- `kernel/src/fs/fs_impls/procfs/mod.rs`
- `kernel/src/fs/fs_impls/procfs/pid/mod.rs`
- `kernel/src/fs/fs_impls/procfs/pid/task/fd.rs`
- `kernel/src/fs/fs_impls/procfs/pid/task/mod.rs`
- `kernel/src/fs/fs_impls/procfs/pid/task/ns.rs`
- `kernel/src/fs/fs_impls/procfs/sys/kernel/mod.rs`
- `kernel/src/fs/fs_impls/procfs/sys/kernel/yama.rs`
- `kernel/src/fs/fs_impls/procfs/sys/mod.rs`
- `kernel/src/fs/fs_impls/procfs/template/builder.rs`
- `kernel/src/fs/fs_impls/procfs/template/dir.rs`
- `kernel/src/fs/fs_impls/procfs/template/file.rs`
- `kernel/src/fs/fs_impls/procfs/template/mod.rs`
- `kernel/src/fs/fs_impls/procfs/template/sym.rs`
- `kernel/src/fs/utils/systree_inode.rs`
- `kernel/src/process/task_set.rs`

## Review Units

| Unit | Kind | Anchor Paths | Spec Surface | Validation Surface | Routed Skill |
|------|------|--------------|--------------|--------------------|--------------|
| `feature-interface-fs` | `feature-interface` | `kernel/src/fs/utils/systree_inode.rs` | external spec if available; upstream Linux behavior; Asterinas VFS inode contracts | `L3 general test + verify` when behavior is concrete | `asterinas-module-review` |
| `feature-interface-fs-impl` | `feature-interface` | `kernel/src/fs/fs_impls/cgroupfs/inode.rs`, `kernel/src/fs/fs_impls/devpts/mod.rs`, `kernel/src/fs/fs_impls/procfs/mod.rs`, `kernel/src/fs/fs_impls/procfs/pid/mod.rs`, `kernel/src/fs/fs_impls/procfs/pid/task/fd.rs`, `kernel/src/fs/fs_impls/procfs/pid/task/mod.rs`, `kernel/src/fs/fs_impls/procfs/pid/task/ns.rs`, `kernel/src/fs/fs_impls/procfs/sys/kernel/mod.rs`, `kernel/src/fs/fs_impls/procfs/sys/kernel/yama.rs`, `kernel/src/fs/fs_impls/procfs/sys/mod.rs`, `kernel/src/fs/fs_impls/procfs/template/builder.rs`, `kernel/src/fs/fs_impls/procfs/template/dir.rs`, `kernel/src/fs/fs_impls/procfs/template/file.rs`, `kernel/src/fs/fs_impls/procfs/template/sym.rs` | upstream Linux procfs behavior; Asterinas procfs contracts; diff-local call path | `L3 general test + verify` when behavior is concrete | `asterinas-module-review` |
| `module-fs-impl` | `module` | `kernel/src/fs/fs_impls/devpts/ptmx.rs`, `kernel/src/fs/fs_impls/devpts/slave.rs`, `kernel/src/fs/fs_impls/exfat/inode.rs`, `kernel/src/fs/fs_impls/procfs/template/mod.rs` | upstream Linux behavior where relevant; Asterinas internal contracts | `L3 general test + verify` when behavior is concrete | `asterinas-module-review` |
| `module-process` | `module` | `kernel/src/process/task_set.rs` | Asterinas internal task-set and observer contracts | report-only unless externally visible behavior is affected | `asterinas-module-review` |

## Sources

| Source Type | Location | Notes |
|-------------|----------|-------|
| `linux-observation` | host Linux `/proc`, `/proc/self/ns`, `/proc/self/fd` | Confirmed that `readdir` `d_ino` matches `lstat().st_ino` for procfs entries on Linux. |
| `asterinas` | `kernel/src/fs/vfs/path/dentry.rs` | VFS caches child dentries and only revalidates cached positives when the child inode advertises `need_revalidation()`. |
| `asterinas` | `kernel/src/fs/fs_impls/procfs/template/dir.rs` | New procfs `readdir` path consumes freshly constructed `ReaddirEntry` inodes directly. |
| `asterinas` | `kernel/src/fs/fs_impls/procfs/mod.rs` | Root procfs `readdir` and `lookup` now both construct fresh static and PID child inodes instead of reusing cached child inodes. |
| `asterinas` | `kernel/src/fs/fs_impls/procfs/pid/task/fd.rs` | `/proc/[pid]/fd*` enumeration and lookup both construct fresh child inodes on demand. |
| `asterinas` | `kernel/src/fs/fs_impls/procfs/template/file.rs` | Every new procfs regular file inode allocates a fresh procfs inode number with `procfs.alloc_id()`. |
| `asterinas` | `kernel/src/fs/fs_impls/procfs/template/sym.rs` | Every new procfs symlink inode allocates a fresh procfs inode number with `procfs.alloc_id()`. |
| `asterinas-prechange` | `41c380cfb0d4badcb859784417f4813fdf9f9383^:kernel/src/fs/fs_impls/procfs/template/dir.rs` | Before this diff, procfs `lookup` and `readdir` shared `cached_children`, which preserved inode identity for the lifetime of a cached entry. |
| `test` | `test/initramfs/src/apps/fs/procfs/dentry_cache.c` | Added an L3 regression that asserts `readdir` and `lstat` expose the same inode number for procfs entries. |

## Behavior Matrix

| Unit | Behavior | Expected Semantics | Evidence | Asterinas Status | Confidence | Class | Testability |
|------|----------|--------------------|----------|------------------|------------|-------|-------------|
| `feature-interface-fs-impl` | Procfs entries returned by `readdir` must expose the same inode number later observed by `lookup` and `lstat` for the same pathname. | Procfs inode identity must be stable across directory enumeration and pathname lookup. On Linux, `d_ino` from `readdir` matches `st_ino` from `lstat` for `/proc/self`, `/proc/self/ns/user`, and `/proc/self/fd/<fd>`. Pre-change Asterinas also preserved this by reusing cached child inodes for both lookup and readdir. | Host Linux observation; pre-change `cached_children` reuse; post-change `RootDirOps::static_entries()` and `process_entries()` allocate fresh inodes, `FdDirOps::fd_entries()` allocates fresh inodes, `ProcDir::readdir_at()` emits those inodes directly, and `ProcFile::new()` / `ProcSym::new()` allocate a new procfs inode number on each construction. | Confirmed regression. The new procfs path can hand userspace one inode number during `readdir` and a different inode number during later `lookup` or `lstat` of the same entry. | `high` | `bug` | `L3 implemented` |

## Candidate Regressions

- `/proc/<pid>` and `/proc/<pid>/task/<tid>` likely share the same inode-identity problem because their `readdir` paths now also construct fresh child directory inodes on demand. The current L3 regression already covers the same bug class through `/proc/self`, `/proc/self/ns/user`, and `/proc/self/fd/<fd>`.

## Implemented Tests

- Added `readdir_inode_matches_lstat` to `test/initramfs/src/apps/fs/procfs/dentry_cache.c`.
- The test checks three externally visible procfs surfaces:
- `/proc/self`
- `/proc/self/ns/user`
- `/proc/self/fd/<fd>`
- Each check asserts that `readdir` `d_ino` matches `lstat().st_ino`.

## Validation Status

- Direct Linux validation of the touched procfs binary passed:
- Build command: `make --no-print-directory -C test/initramfs/src/apps/fs/procfs BUILD_DIR=/tmp/asterinas-procfs-review TEST_PLATFORM=linux HOST_PLATFORM=x86_64-linux`
- Binary: `/tmp/asterinas-procfs-review/initramfs/test/fs/procfs/dentry_cache`
- Result: passed on host Linux, including the new `readdir_inode_matches_lstat` regression.
- Shared `verify fs` could not reach the Asterinas phase because the existing Linux `fs` module fails earlier in `./fdatasync/fdatasync /exfat` on this host environment.
- Linux log for the blocked shared verification: `/tmp/asterinas-validate-linux-behavior-fs-linux-20260325T021138Z.log`

## Open Questions

- No additional confirmed bugs were found in the cgroupfs, devpts, exfat, systree, or task-set slices of this diff after reviewing the old `is_dentry_cacheable()` behavior against the new revalidation hooks.
- Asterinas execution of the new regression remains pending until the unrelated Linux-side `fs` verification blockage around `/exfat` is addressed or the procfs test can be isolated in the shared runner.

## Next Actions

- Fix procfs inode identity by reusing stable child inodes across both `readdir` and `lookup`, or by threading stable per-entry inode numbers through procfs inode construction instead of allocating a fresh procfs inode number on every on-demand constructor call.
- After the fix lands, rerun the procfs regression on Linux and rerun the shared Asterinas validation path once the unrelated `fs` `/exfat` Linux blocker is removed or bypassed.
