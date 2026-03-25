# `rename` Syscall Review

## Executive Summary

- Syscall: `rename`
- Asterinas entry: `/root/asterinas/kernel/src/syscall/rename.rs`
- Review date: `2026-03-25`
- Reviewer: `Codex`
- Validation plan: `general test + verify`
- User-space input focus: `empty paths, trailing slashes on old/new paths, invalid pointers, maximum-length paths, renameat2 flag combinations, same-path and descendant-directory transitions`
- Status: `done`

## Findings (cards; order by severity)

- `F1`
  Claim: `rename("file", "dir/new/")` returns `EISDIR` in Asterinas, but Linux returns `ENOTDIR` when the source is not a directory and the destination path ends with `/`.
  Anchors: `kernel/src/syscall/rename.rs:53`
  Evidence IDs: `E1`, `E2`, `E3`
  Class: `bug`
  Confidence: `high`
  Input cases: `non-directory old path`, `destination path with trailing slash`, `existing or non-existing destination basename`
  Test intent: `regression`

- `F2`
  Claim: `renameat2` rejects every non-zero flag with `EINVAL`, so `RENAME_NOREPLACE`, `RENAME_EXCHANGE`, and `RENAME_WHITEOUT` are syscall-visible unsupported features rather than Linux-compatible behavior.
  Anchors: `kernel/src/syscall/rename.rs:31`, `kernel/src/syscall/rename.rs:32`
  Evidence IDs: `E1`, `E3`
  Class: `unsupported`
  Confidence: `high`
  Input cases: `supported flag bits`, `flag combinations`, `non-zero flags on otherwise valid paths`
  Test intent: `confirmation`

## Sources

| Source Type | Location | Notes |
|-------------|----------|-------|
| manual | `https://man7.org/linux/man-pages/man2/rename.2.html` | Linux-visible `rename` and `renameat2` contract, including flag behavior and errno classes. |
| linux | `https://codebrowser.dev/linux/linux/fs/namei.c.html#6048` | `do_renameat2()` returns `ENOTDIR` when a non-directory source is paired with a trailing slash on the destination. |
| asterinas | `/root/asterinas/kernel/src/syscall/rename.rs` | Syscall entry under review. |
| asterinas | `/root/asterinas/test/initramfs/src/apps/fs/ext2/rename.c` | Existing ext2 general test target used for validation. |

## User-space Input Corner Cases

- Pointers / buffer validity:
  `read_cstring()` can return `EFAULT` for invalid user pointers and `ENAMETOOLONG` when no NUL appears within `MAX_FILENAME_LEN`.
- Zero-length / empty inputs:
  Empty old or new paths are rejected by `SplitPath::split_dirname_and_basename()` with `ENOENT`.
- Boundary sizes / truncation:
  Both paths are capped at `MAX_FILENAME_LEN = 4096`; overlong user strings are rejected before VFS lookup.
- Flag combinations / reserved bits:
  Unknown bits return `EINVAL`; all known non-zero `renameat2` flags currently return `EINVAL` as unsupported.
- Partial success / retry / state transitions:
  Same-path rename is a no-op success; descendant-directory rename fails with `EINVAL`; cross-mount rename is delegated to VFS and returns `EXDEV`.

## Linux Semantics Matrix (keep tight and reference evidence IDs)

| Case | Inputs / Preconditions | Linux Semantics | Evidence IDs | Notes |
|------|------------------------|-----------------|--------------|-------|
| `rename(file, "dir/new/")` | source is not a directory; destination ends with `/` | Fails with `ENOTDIR` | `E1`, `E2` | Derived from Linux manual plus `do_renameat2()` control flow. |
| `rename(dir, dir/sub)` | destination is inside renamed directory subtree | Fails with `EINVAL` | `E1`, `E3` | Asterinas matches this case. |
| `renameat2(..., flags != 0)` | valid paths, known Linux flag bits | Linux defines semantics for `NOREPLACE`, `EXCHANGE`, `WHITEOUT` | `E1`, `E3` | Asterinas rejects all of them today. |

## Asterinas Implementation Notes

- Entry path:
  `sys_rename()` and `sys_renameat()` both funnel into `sys_renameat2()`.
- Relevant helpers:
  `CurrentUserSpace::read_cstring()`, `SplitPath::split_dirname_and_basename()`, `PathResolver::lookup_at_path()`, `Path::rename()`.
- Argument validation:
  Invalid flag bits are rejected early; trailing slash handling is split between old-path and new-path special cases before VFS rename.
- Return / errno behavior:
  The old-path trailing slash check returns `ENOTDIR`, but the new-path trailing slash check returns `EISDIR` for non-directory sources.
- Side effects:
  Successful rename delegates to VFS/backend rename after both parent paths are resolved.
- Explicit TODOs or unimplemented paths:
  The syscall explicitly marks `RENAME_NOREPLACE`, `RENAME_EXCHANGE`, and `RENAME_WHITEOUT` as unsupported.

## Comparison Matrix (link to evidence IDs)

| Case | Inputs / Preconditions | Linux Semantics | Asterinas Status | Class | Evidence IDs | Test Status |
|------|------------------------|-----------------|------------------|-------|--------------|-------------|
| Destination trailing slash with non-directory source | `rename("file", "dir/new/")` | `ENOTDIR` | Returns `EISDIR` at syscall entry | `bug` | `E1`, `E2`, `E3` | Linux target passed; Asterinas verify exited non-zero before emitting target output |
| Descendant directory rename | `rename("dir", "dir/sub")` | `EINVAL` | Matches | `n/a` | `E1`, `E3` | Covered by existing ext2 test |
| Non-zero `renameat2` flags | `RENAME_NOREPLACE`, `RENAME_EXCHANGE`, `RENAME_WHITEOUT` | Defined Linux feature surface | Unconditionally rejected with `EINVAL` | `unsupported` | `E1`, `E3` | Report-only |

## Evidence Appendix

| ID | Source (`spec|linux-derived|asterinas-contract|diff-intent`) | Path | Note | Supports (Finding IDs) |
|----|--------------------------------------------------------------|------|------|-------------------------|
| `E1` | `spec` | `https://man7.org/linux/man-pages/man2/rename.2.html` | Linux man page defines destination-trailing-slash handling and `renameat2` flag semantics. | `F1`, `F2` |
| `E2` | `linux-derived` | `https://codebrowser.dev/linux/linux/fs/namei.c.html#6048` | `do_renameat2()` sets `-ENOTDIR` when the old dentry is not a directory and either path carries a directory-only trailing slash requirement. | `F1` |
| `E3` | `asterinas-contract` | `kernel/src/syscall/rename.rs` | Asterinas returns `EISDIR` for a destination trailing slash on non-directory sources and rejects all non-zero `renameat2` flags. | `F1`, `F2` |

## Validation Log

- Planned targets: `fs/ext2/rename`
- Confirmation tests used to resolve uncertainty: `host Linux errno probe for rename(file, "new/")`
- Commands run: `python3 - <<'PY' ... os.rename(old, new_with_trailing_slash) ... PY`, `python3 /root/.codex/skills/asterinas-test/scripts/run_targets.py verify fs/ext2/rename`
- Outcomes: `Host Linux returned errno 20 (ENOTDIR). The Linux phase of the shared general test passed, including the new trailing-slash case, with log /tmp/asterinas-validate-linux-behavior-fs-ext2-rename-linux-20260325T114532Z.log. The Asterinas phase exited non-zero and saved /tmp/asterinas-run-test-fs-ext2-rename-20260325T114533Z.log, but that log stops at early guest boot output before the targeted test lines, so runtime confirmation on Asterinas was inconclusive.`

## Candidate Regression Tests

- None recorded yet.

## Implemented Tests

- `test/initramfs/src/apps/fs/ext2/rename.c`: added `rename_non_dir_to_trailing_slash_path`, which expects `ENOTDIR` and checks that the source file remains in place.

## Open Questions

- The current Asterinas verify run for `fs/ext2/rename` returned non-zero before the guest emitted targeted test output. The saved log is useful as an artifact path, but not as direct behavioral evidence for this syscall review.
