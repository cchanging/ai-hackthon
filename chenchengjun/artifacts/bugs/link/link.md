# `link` Syscall Review

## Executive Summary

- Syscall: `link`
- Asterinas entry: `/root/asterinas/kernel/src/syscall/link.rs`
- Review date: `2026-03-25`
- Reviewer: `Codex`
- Validation plan: `general test + verify`
- Status: `done`

## Findings (cards; order by severity)

- `F1` `bug` `high`
  `link()` did not enforce write permission on the destination parent directory, so an unprivileged caller could create a hard link where Linux returns `EACCES`.
  Anchors: `kernel/src/fs/vfs/path/dentry.rs:331`, `kernel/src/syscall/link.rs:62`, `test/initramfs/src/apps/fs/link/link.c:42`
  Evidence: `E1`, `E2`, `E3`, `E4`
  Test intent: `general`, module `fs`, `fs/link/link`

Each finding should link to evidence IDs from the appendix and list anchors as `<repo-relative-path>:<line?>`.

## Sources

| Source Type | Location | Notes |
|-------------|----------|-------|
| manual | `https://man7.org/linux/man-pages/man2/link.2.html` | Documents that `link()`/`linkat()` fail with `EACCES` when the caller lacks permission to write in the destination directory. |
| linux | `https://codebrowser.dev/linux/linux/fs/namei.c.html` | Linux `vfs_link()` routes through `may_create()`, which performs the destination-parent permission check. |
| asterinas | `/root/asterinas/kernel/src/syscall/link.rs` | Syscall entry resolves both paths and dispatches to VFS `Path::link()`. |
| asterinas | `/root/asterinas/kernel/src/fs/vfs/path/dentry.rs` | Shared hard-link VFS helper where the missing permission check lived. |

## Linux Semantics Matrix (keep tight and reference evidence IDs)

| Case | Inputs / Preconditions | Linux Semantics | Evidence IDs | Notes |
|------|------------------------|-----------------|--------------|-------|
| destination parent not writable | Caller can traverse the destination directory, owns the source file, but lacks write permission on the parent of `newpath` | `link()` fails with `EACCES` and no new directory entry is created | `E1`, `E2` | The source file ownership matters on Linux because protected hard-link policy can reject unrelated cases earlier. |

## Asterinas Implementation Notes

- Entry path: `sys_link()` delegates to `sys_linkat()`, which resolves `oldpath` and `newpath` and then calls `new_path.link(&old_path, &new_name)` (`kernel/src/syscall/link.rs:66`, `kernel/src/syscall/link.rs:62`).
- Relevant helpers: `Path::link()` checks same-mount constraints and forwards to `DirDentry::link()` (`kernel/src/fs/vfs/path/mod.rs:473`); `DirDentry::link()` forwards to the inode backend (`kernel/src/fs/vfs/path/dentry.rs:331`).
- Argument validation: `sys_linkat()` validates flags and rejects directory targets from `oldpath` (`kernel/src/syscall/link.rs:25`, `kernel/src/syscall/link.rs:51`).
- Return / errno behavior: Before this fix, `DirDentry::link()` performed existence checks and backend dispatch without checking `Permission::MAY_WRITE` on the destination directory. The current branch now enforces that check first (`kernel/src/fs/vfs/path/dentry.rs:332`).
- Side effects: On success, the backend inode `link()` implementation creates the new name and bumps the source inode link count.
- Explicit TODOs or unimplemented paths: None in the syscall entry; `AT_EMPTY_PATH` capability semantics were not revalidated in this turn.

## Comparison Matrix (link to evidence IDs)

| Case | Inputs / Preconditions | Linux Semantics | Asterinas Status | Class | Evidence IDs | Test Status |
|------|------------------------|-----------------|------------------|-------|--------------|-------------|
| destination parent not writable | Searchable but non-writable destination directory; source file owned by caller | `EACCES`, no new link | Fixed in current branch by adding `Permission::MAY_WRITE` enforcement in `DirDentry::link()` | `bug` | `E1`, `E2`, `E3`, `E4` | `Implemented + verified` |

## Evidence Appendix

| ID | Source (`spec\linux-derived\asterinas-contract\diff-intent`) | Path | Note | Supports (Finding IDs) |
|----|--------------------------------------------------------------|------|------|-------------------------|
| E1 | `spec` | `https://man7.org/linux/man-pages/man2/link.2.html` | `link(2)` documents destination-directory permission failure as `EACCES`. | `F1` |
| E2 | `linux-derived` | `https://codebrowser.dev/linux/linux/fs/namei.c.html` | Linux `vfs_link()` calls `may_create()`, which is the destination-parent permission gate. | `F1` |
| E3 | `asterinas-contract` | `kernel/src/fs/vfs/path/dentry.rs:331` | The shared hard-link helper used to skip a destination-parent write check before reaching filesystem backends. | `F1` |
| E4 | `diff-intent` | `test/initramfs/src/apps/fs/link/link.c:42` | New regression test drops to UID/GID `65534` and expects `EACCES` with no link created. | `F1` |

## Validation Log

- Planned targets: `fs/link/link`
- Commands run: `python3 /root/.codex/skills/asterinas-test/scripts/run_targets.py verify fs/link/link`
- Outcomes: `pass` on both Linux and Asterinas.
  Linux log: `/tmp/asterinas-validate-linux-behavior-fs-link-link-linux-20260325T075622Z.log`
  Asterinas log: `/tmp/asterinas-run-test-fs-link-link-20260325T075623Z.log`

## Candidate Regression Tests

- None recorded yet.

## Implemented Tests

- `test/initramfs/src/apps/fs/link/link.c`
- `test/initramfs/src/apps/fs/Makefile`
- `test/initramfs/src/apps/fs/run_test.sh`

## Open Questions

- `AT_EMPTY_PATH` handling was not revalidated against Linux capability semantics in this turn.
