# `rmdir` Syscall Review

## Executive Summary

- Syscall: `rmdir`
- Asterinas entry: `/root/asterinas/kernel/src/syscall/rmdir.rs`
- Review date: `2026-03-25`
- Reviewer: `Codex`
- Validation plan: `general test + verify`
- Status: `done`

`rmdir` matches Linux on basic path-shape cases such as `""`, `/`, `.`, `..`, and non-empty directories, but it misses two Linux-visible permission checks in the delete path. The syscall wrapper resolves the parent with search permission and then delegates to `Path::rmdir()` without any parent-directory write DAC check or sticky-bit ownership enforcement.

## Findings

- `F1` `bug` `high`
  Claim: `rmdir` can remove an otherwise-empty directory even when the caller lacks write permission on the parent directory.
  Anchors: `kernel/src/syscall/rmdir.rs:17`, `kernel/src/fs/vfs/path/resolver.rs:416`, `kernel/src/fs/vfs/path/mod.rs:487`, `kernel/src/fs/vfs/path/dentry.rs:402`, `kernel/src/fs/vfs/fs_apis/inode.rs:426`, `kernel/src/fs/fs_impls/ext2/inode.rs:297`, `kernel/src/fs/fs_impls/ramfs/fs.rs:951`
  Evidence: `E1`, `E2`, `E3`, `E4`, `E5`, `E8`
  Test: `general`, module `fs`, implemented in `test/initramfs/src/apps/fs/ext2/rmdir.c`

- `F2` `bug` `med`
  Claim: `rmdir` does not enforce sticky-bit ownership rules, so a non-owner can remove another user's directory from a sticky parent.
  Anchors: `kernel/src/syscall/rmdir.rs:17`, `kernel/src/fs/vfs/path/mod.rs:487`, `kernel/src/fs/vfs/path/dentry.rs:402`, `kernel/src/fs/file/inode_attr/mode.rs:74`, `kernel/src/fs/fs_impls/ext2/inode.rs:297`, `kernel/src/fs/fs_impls/ramfs/fs.rs:951`
  Evidence: `E6`, `E3`, `E4`, `E5`, `E7`, `E9`
  Test: `general`, module `fs`, implemented in `test/initramfs/src/apps/fs/ext2/rmdir.c`

## Sources

| Source Type | Location | Notes |
|-------------|----------|-------|
| manual | <https://man7.org/linux/man-pages/man2/rmdir.2.html> | Governing Linux-visible permission and sticky-bit semantics. |
| asterinas | `/root/asterinas/kernel/src/syscall/rmdir.rs` | Syscall entry and delegation path. |
| asterinas | `/root/asterinas/kernel/src/fs/vfs/path/resolver.rs` | Parent-path lookup enforces `MAY_EXEC` only. |
| asterinas | `/root/asterinas/kernel/src/fs/vfs/path/dentry.rs` | Delete path performs no permission check before backend `rmdir()`. |
| validation | `/tmp/asterinas-validate-linux-behavior-fs-ext2-rmdir-linux-20260325T090811Z.log` | Linux passed the new regression cases. |
| validation | `/tmp/asterinas-run-test-fs-ext2-rmdir-20260325T090812Z.log` | Asterinas phase booted, but target output was not captured after the console warning. |

## Linux Semantics Matrix

| Case | Inputs / Preconditions | Linux Semantics | Evidence IDs | Notes |
|------|------------------------|-----------------|--------------|-------|
| Parent not writable | Caller can search parent, but parent lacks write permission | `rmdir()` fails with `EACCES` | `E1`, `E8` | Confirmed by the new Linux regression case. |
| Sticky parent, caller owns neither parent nor child | Parent has `S_ISVTX`; child is otherwise removable | `rmdir()` fails with `EPERM` | `E6`, `E9` | The Linux man page documents this rule. |
| Root directory | Path is `/` or slash-only | `rmdir()` fails with `EBUSY` | `E10` | Asterinas matches this through `split_dirname_and_basename()`. |
| Empty string | Path is `""` | `rmdir()` fails with `ENOENT` | `E10` | Asterinas matches this through `read_cstring` + `FsPath` validation. |

## Asterinas Implementation Notes

- Entry path: `sys_rmdir()` forwards to `sys_rmdirat()`, which splits the pathname into `(parent, basename)` and resolves only the parent path before calling `dir_path.rmdir(name)`.
- Relevant helpers: `PathResolver::lookup_at_path()` checks `MAY_EXEC` on the parent path; `Path::rmdir()` and `DirDentry::rmdir()` forward to backend `rmdir()` implementations.
- Argument validation: empty paths become `ENOENT`; slash-only root paths become `EBUSY`; `"."` becomes `EINVAL`; `".."` becomes `ENOTEMPTY`.
- Return / errno behavior: backend implementations correctly reject non-directories and non-empty directories, but they do not call `check_permission(Permission::MAY_WRITE)` or enforce sticky-bit ownership rules.
- Side effects: successful deletion removes the dentry cache entry and emits VFS notifications.
- Explicit TODOs or unimplemented paths: none on the syscall path itself.

## Comparison Matrix

| Case | Inputs / Preconditions | Linux Semantics | Asterinas Status | Class | Evidence IDs | Test Status |
|------|------------------------|-----------------|------------------|-------|--------------|-------------|
| Parent lacks write permission | Searchable parent, no parent write bit | Fail `EACCES` | No parent write-permission check appears before backend deletion | `bug` | `E1`, `E2`, `E3`, `E4`, `E5`, `E8` | Implemented; Linux passed; Asterinas runtime confirmation blocked by console issue |
| Sticky parent, caller not owner | Sticky parent, caller owns neither parent nor child | Fail `EPERM` | No sticky-bit check appears anywhere in the `rmdir` path | `bug` | `E6`, `E3`, `E4`, `E5`, `E7`, `E9` | Implemented; Linux passed; Asterinas runtime confirmation blocked by console issue |
| Slash-only root path | `"/"` or `"///"` | Fail `EBUSY` | Matches | `none` | `E10` | Covered by code inspection |
| Non-empty directory | Target has children other than `.` and `..` | Fail `ENOTEMPTY` | Matches | `none` | `E4`, `E5` | Covered by existing `fs/ext2/rmdir` test |

## Evidence Appendix

| ID | Source (`spec|linux-derived|asterinas-contract|diff-intent`) | Path | Note | Supports (Finding IDs) |
|----|--------------------------------------------------------------|------|------|-------------------------|
| E1 | `spec` | <https://man7.org/linux/man-pages/man2/rmdir.2.html> | The `ERRORS` section documents `EACCES` when write access to the directory containing `path` is denied. | `F1` |
| E2 | `asterinas-contract` | `kernel/src/fs/vfs/path/resolver.rs:416` | Parent lookup checks only `Permission::MAY_EXEC`, so lookup enforces search permission but not parent write permission. | `F1` |
| E3 | `asterinas-contract` | `kernel/src/fs/vfs/path/dentry.rs:402` | `DirDentry::rmdir()` validates `.`/`..` and mountpoints, then directly calls backend `rmdir()` without a permission gate. | `F1`, `F2` |
| E4 | `asterinas-contract` | `kernel/src/fs/fs_impls/ext2/inode.rs:297` | The ext2 backend checks target type and emptiness, but not parent write permission or sticky-bit ownership. | `F1`, `F2` |
| E5 | `asterinas-contract` | `kernel/src/fs/fs_impls/ramfs/fs.rs:951` | The ramfs backend likewise omits write-permission and sticky-bit checks. | `F1`, `F2` |
| E6 | `spec` | <https://man7.org/linux/man-pages/man2/rmdir.2.html> | The `ERRORS` section documents `EPERM` when the parent has the sticky bit set and the caller lacks the required ownership/capability. | `F2` |
| E7 | `asterinas-contract` | `kernel/src/fs/file/inode_attr/mode.rs:74` | `InodeMode::has_sticky_bit()` exists but is unused in the `rmdir` delete path. | `F2` |
| E8 | `linux-derived` | `/tmp/asterinas-validate-linux-behavior-fs-ext2-rmdir-linux-20260325T090811Z.log` | The new `rmdir_requires_write_permission_on_parent` regression passed on host Linux. | `F1` |
| E9 | `linux-derived` | `/tmp/asterinas-validate-linux-behavior-fs-ext2-rmdir-linux-20260325T090811Z.log` | The new `rmdir_honors_sticky_bit_ownership_rules` regression passed on host Linux. | `F2` |
| E10 | `asterinas-contract` | `kernel/src/fs/vfs/path/resolver.rs:817` | `split_dirname_and_basename()` returns `ENOENT` for `""` and `EBUSY` for slash-only root paths, matching Linux. |  |

## Validation Log

- Planned targets: `fs/ext2/rmdir`
- Commands run: `make --no-print-directory -C test/initramfs/src/apps/fs/ext2 BUILD_DIR=/tmp/asterinas-build`, `python3 /root/.codex/skills/asterinas-test/scripts/run_targets.py verify fs/ext2/rmdir`
- Outcomes:
  - Local compile with writable `BUILD_DIR`: passed
  - Host Linux verification: passed, including both newly added regression cases
  - Asterinas verification: runner reported a mismatch, but the saved log stops after boot with `WARNING: no console will be available to OS`; no target-level assertion output was captured in this environment

## Candidate Regression Tests

- None. Both confirmed bugs have concrete user-visible oracles and now have implemented general tests.

## Implemented Tests

- `test/initramfs/src/apps/fs/ext2/rmdir.c:64`
  Adds `rmdir_requires_write_permission_on_parent`, which drops to uid/gid `65534` and expects `EACCES`.
- `test/initramfs/src/apps/fs/ext2/rmdir.c:100`
  Adds `rmdir_honors_sticky_bit_ownership_rules`, which expects `EPERM` or `EACCES` under a sticky parent.

## Open Questions

- The Asterinas validation environment in this turn did not capture guest test output after the QEMU boot warning `no console will be available to OS`. The code evidence and Linux validation are sufficient to classify `F1` and `F2` as bugs, but an assertion-level Asterinas runtime log is still missing.
