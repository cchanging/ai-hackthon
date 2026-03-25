# `pwrite64` Syscall Review

## Executive Summary

- Syscall: `pwrite64`
- Asterinas entry: `/root/asterinas/kernel/src/syscall/pwrite64.rs`
- Review date: `2026-03-25`
- Reviewer: `Codex`
- Validation plan: `general test + verify`
- User-space input focus: `count==0 (buf ignored), non-seekable fds, O_RDONLY fds, negative offsets, offset+len overflow, EINTR restart mapping`
- Status: `done`

## Findings (cards; order by severity)

- F1 (`bug`, fixed): `pwrite64(..., count==0)` must still validate seekability and write access; previously Asterinas returned `0` without calling `write_at`, so it could succeed on pipes (`ESPIPE` expected) and on O_RDONLY fds (`EBADF` expected).
  - Anchors: `kernel/src/syscall/pwrite64.rs:37`, `kernel/src/fs/file/file_handle.rs:43`, `test/initramfs/src/apps/io/file_io/pwrite64_zero_len.c:30`
  - Evidence: `E1`, `E2`, `E3`, `E4`, `E5`
  - Input cases: `count==0 with NULL/bad buf ptr`, `pipe/fifo fd`, `O_RDONLY fd`, `negative offset`
  - Test intent: `regression` (general test module `io`)

## Sources

| Source Type | Location | Notes |
|-------------|----------|-------|
| manual | `https://man7.org/linux/man-pages/man2/pread64.2.html` | Documents `pread/pwrite` semantics, including seekability requirement and the Linux `O_APPEND` quirk. |
| linux | `https://linux.googlesource.com/linux/kernel/git/torvalds/linux/+/49ffdb4c7c65082cee24a53a7ebd62e00eb2e9e9/fs/read_write.c` | `ksys_pwrite64()` shows `pos < 0 => -EINVAL` and non-`FMODE_PWRITE => -ESPIPE`, with no special-case for `count==0`. |
| asterinas | `/root/asterinas/kernel/src/syscall/pwrite64.rs` | syscall entry |
| asterinas | `/root/asterinas/kernel/src/fs/file/file_handle.rs` | `FileLike::write_at` contract mentions append-only behavior; helper `write_bytes_at()` enables empty-buffer checks without touching user memory. |
| asterinas | `/root/asterinas/kernel/src/fs/file/inode_handle.rs` | `InodeHandle::write_at` implements `O_APPEND` by ignoring the passed offset (when applicable). |

## User-space Input Corner Cases

- Pointers / buffer validity: `count==0` with `buf==NULL` and with an invalid `buf` value (must not dereference user memory).
- Zero-length / empty inputs: `count==0` must still validate FD access mode and seekability (positional write support).
- Boundary sizes / truncation: `count` too large for `i64` (conversion) and `offset + count` overflow.
- Flag combinations / reserved bits: `O_APPEND` interaction (Linux quirk: positional writes append on Linux when `O_APPEND` is set).
- Partial success / retry / state transitions: `EINTR` mapping to `ERESTARTSYS` for restart behavior (align with `read(2)`/`write(2)` handling in Asterinas).

## Linux Semantics Matrix (keep tight and reference evidence IDs)

| Case | Inputs / Preconditions | Linux Semantics | Evidence IDs | Notes |
|------|------------------------|-----------------|--------------|-------|
| Seekability required | `pwrite64(fd, ..., offset)` | `fd` must be seekable. | `E1`, `E2` | Requirement is stated in man7; kernel enforces via `FMODE_PWRITE` check. |
| Non-seekable FD | `fd` is pipe/fifo/socket | Fails with `ESPIPE`. | `E3` | Linux kernel returns `-ESPIPE` when `!(file->f_mode & FMODE_PWRITE)`. |
| Empty buffer still validates | `count==0` | Still checks seekability/access, but does not access user memory; returns `0` on success. | `E3` | No special-casing for `count==0` around the seekability check. |
| Negative offset | `offset < 0` | Fails with `EINVAL`. | `E3` | Explicit `pos < 0` check in `ksys_pwrite64`. |
| O_APPEND quirk | FD opened with `O_APPEND` | On Linux, `pwrite()` appends regardless of `offset` (documented bug). | `E2` | Asterinas chooses to implement “offset ignored for append-only” too (see `InodeHandle::write_at`). |

## Asterinas Implementation Notes

- Entry path: `sys_pwrite64()` validates `offset`, resolves the file descriptor from the thread-local file table, validates the range, and then dispatches through `write_bytes_at()` for `count==0` or `write_at()` otherwise.
- Relevant helpers: `get_file_fast!()`, `FileLike::write_bytes_at()`, `FileLike::write_at()`, `InodeHandle::write_at()`, and `fs::vfs::notify::on_modify()`.
- Argument validation: negative `offset` returns `EINVAL`; `user_buf_len` must fit in `i64`; `offset + user_buf_len` overflow is rejected before any user-space access.
- Return / errno behavior: empty-buffer writes still validate access and seekability; both empty and non-empty paths now map `EINTR` to `ERESTARTSYS`.
- Side effects: successful non-empty writes notify the VFS via `on_modify`; empty-buffer writes only validate and return `0`.
- Explicit TODOs or unimplemented paths:
  - Empty-buffer behavior: `kernel/src/syscall/pwrite64.rs` now uses `file.write_bytes_at(offset, &[])` so that `count==0` still triggers access/seekability checks without touching user memory.
  - O_APPEND behavior: `kernel/src/fs/file/inode_handle.rs` ignores the passed offset when `StatusFlags::O_APPEND` is set (when applicable).

## Comparison Matrix (link to evidence IDs)

| Case | Inputs / Preconditions | Linux Semantics | Asterinas Status | Class | Evidence IDs | Test Status |
|------|------------------------|-----------------|------------------|-------|--------------|-------------|
| `count==0` on non-seekable fd | `pwrite64(pipe_fd, NULL, 0, 0)` | `ESPIPE` | Fixed to validate via `write_bytes_at` even when empty. | `bug` | `E3`, `E4`, `E5` | `implemented` (`io/file_io/pwrite64_zero_len`) |
| `count==0` on O_RDONLY fd | `pwrite64(ro_fd, NULL, 0, 0)` | `EBADF` | Fixed to validate via `write_bytes_at` even when empty. | `bug` | `E3`, `E5` | `implemented` (`io/file_io/pwrite64_zero_len`) |
| `count==0` ignores buf ptr | `pwrite64(rw_fd, bad_ptr, 0, 0)` | Success, `0` | Should succeed without touching user memory. | `regression-risk` | `E3`, `E5` | `implemented` (`io/file_io/pwrite64_zero_len`) |

## Evidence Appendix

| ID | Source (`spec|linux-derived|asterinas-contract|diff-intent`) | Path | Note | Supports (Finding IDs) |
|----|--------------------------------------------------------------|------|------|-------------------------|
| E1 | `spec` | `https://man7.org/linux/man-pages/man2/pread64.2.html` | `pwrite()` requires a seekable FD and does not change the file offset. | `F1` |
| E2 | `spec` | `https://man7.org/linux/man-pages/man2/pread64.2.html` | Documents Linux `O_APPEND` behavior for `pwrite()` as a bug (appends regardless of `offset`). | `F1` |
| E3 | `linux-derived` | `.../fs/read_write.c` (`ksys_pwrite64`) | Enforces `pos < 0 => -EINVAL` and `!FMODE_PWRITE => -ESPIPE`; no special-case for `count==0`. | `F1` |
| E4 | `asterinas-contract` | `kernel/src/fs/file/file_handle.rs:43` | `FileLike::write_at` is seekable-only and ignores `offset` for append-only files; helper `write_bytes_at()` supports empty-buffer calls. | `F1` |
| E5 | `diff-intent` | `kernel/src/syscall/pwrite64.rs:37` | Empty-buffer path now calls `write_bytes_at` to validate access/seekability; also maps `EINTR => ERESTARTSYS`. | `F1` |

## Validation Log

- Planned targets: `io/file_io/pwrite64_zero_len`
- Confirmation tests used to resolve uncertainty: `none`
- Commands run: `python3 /root/.codex/skills/asterinas-test/scripts/run_targets.py verify io/file_io/pwrite64_zero_len`
- Outcomes: `pass` (Linux + Asterinas)
  - Linux log: `/tmp/asterinas-validate-linux-behavior-io-file_io-pwrite64_zero_len-linux-20260325T095434Z.log`
  - Asterinas log: `/tmp/asterinas-run-test-io-file_io-pwrite64_zero_len-20260325T095435Z.log`

## Candidate Regression Tests

- None recorded yet.

## Implemented Tests

- `test/initramfs/src/apps/io/file_io/pwrite64_zero_len.c` (runs via `io/run_test.sh`)

## Open Questions

- None recorded yet.
