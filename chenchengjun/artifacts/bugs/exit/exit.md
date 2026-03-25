# `exit` Syscall Review

## Scope

- Syscall: `exit`
- Asterinas entry: `/root/asterinas/kernel/src/syscall/exit.rs`
- Review date: `2026-03-25`
- Reviewer: `Codex`

## Findings

- `open-question` No confirmed `sys_exit` semantic mismatch was found in the reviewed Asterinas path, but full in-kernel confirmation of the procfs-visible main-thread-exit case is blocked by an Asterinas general-test runtime failure in `/root/asterinas/test/initramfs/src/apps/process/exit/exit_procfs.c` and `/root/asterinas/test/initramfs/src/apps/process/run_test.sh`. The batched QEMU run reached userspace and then failed with `exit_procfs: applet not found`, so the review concludes with one validation blocker rather than a confirmed syscall bug.

## Sources

| Source Type | Location | Notes |
|-------------|----------|-------|
| linux | `/usr/include/x86_64-linux-gnu/bits/waitstatus.h` | `WEXITSTATUS` encoding: parent-visible normal exit status is the low 8 bits shifted into bits 8..15. |
| linux | `/usr/include/x86_64-linux-gnu/sys/wait.h` | Public wait macros map to the wait-status encoding. |
| linux | `/usr/include/x86_64-linux-gnu/asm/unistd_64.h` | Distinguishes raw `__NR_exit` from `__NR_exit_group` on x86-64. |
| linux | `/usr/include/x86_64-linux-gnu/bits/siginfo-consts.h` | `CLD_EXITED` value for `waitid`. |
| linux | `/tmp/exit_status_probe.c` | Host-Linux probe for wait-status truncation (`_exit(0x1234)` -> `WEXITSTATUS == 0x34`). |
| linux | `/tmp/waitid_exit_probe.c` | Host-Linux probe for `waitid(..., WEXITED)` -> `si_code == CLD_EXITED`, `si_status == 0x34`. |
| linux | `/tmp/sys_exit_thread_probe.c` | Host-Linux probe showing raw `SYS_exit` kills only the calling thread in a multithreaded process. |
| linux | `/lib/x86_64-linux-gnu/libc.so.6` | Local libc disassembly shows `_exit` issues `exit_group`, which is distinct from raw `SYS_exit`. |
| asterinas | `/root/asterinas/kernel/src/syscall/exit.rs` | Syscall entry truncates the ABI argument to `u8` via `TermStatus::Exited`. |
| asterinas | `/root/asterinas/kernel/src/process/posix_thread/exit.rs` | Shared exit path for thread-vs-process termination and process exit-code updates. |
| asterinas | `/root/asterinas/kernel/src/process/term_status.rs` | Wait-visible encoding for normal exit vs signal termination. |
| asterinas | `/root/asterinas/kernel/src/syscall/wait4.rs` | `wait4` returns the stored status word directly. |
| asterinas | `/root/asterinas/kernel/src/syscall/waitid.rs` | `waitid` reconstructs `si_code` and `si_status` from the stored exit code. |
| test | `/root/asterinas/test/initramfs/src/apps/process/exit/exit_code.c` | Existing regression coverage for raw `SYS_exit` vs `SYS_exit_group` in multithreaded processes. |
| test | `/root/asterinas/test/initramfs/src/apps/process/exit/exit_procfs.c` | Existing regression coverage for procfs-visible thread counts when threads exit. |
| test | `/root/asterinas/test/initramfs/src/apps/process/exit/exit_waitid_filter.c` | Local untracked workspace test for `waitid(WSTOPPED|WNOHANG)` not consuming an already-exited child. |

## Linux Semantics Matrix

| Case | Inputs / Preconditions | Linux Semantics | Linux Evidence | Notes |
|------|------------------------|-----------------|----------------|-------|
| Normal exit-status truncation | Child exits normally with a status wider than 8 bits, e.g. `0x1234`. | Parent-visible normal exit status is truncated to the low 8 bits (`0x34`). `waitpid` exposes it via `WEXITSTATUS`; `waitid` reports `CLD_EXITED` with the same truncated `si_status`. | `/usr/include/x86_64-linux-gnu/bits/waitstatus.h`; `/usr/include/x86_64-linux-gnu/sys/wait.h`; `/tmp/exit_status_probe.c`; `/tmp/waitid_exit_probe.c`; `/usr/include/x86_64-linux-gnu/bits/siginfo-consts.h` | Strong evidence is a mix of libc headers and host-Linux probes because local man pages are unavailable in this container. |
| Raw `SYS_exit` in a multithreaded process | One thread issues raw `SYS_exit`; other threads remain runnable. | Only the calling thread exits. The process remains alive until the last thread exits, and the final wait-visible process exit status comes from the last exiting thread. | `/usr/include/x86_64-linux-gnu/asm/unistd_64.h`; `/tmp/sys_exit_thread_probe.c` | Distinct from libc `_exit`, which uses `exit_group` on this host. |
| `_exit` / `exit_group` distinction | Multithreaded process invokes libc `_exit` or raw `SYS_exit_group`. | Both are process-wide on this host and terminate the whole thread group immediately. | `/lib/x86_64-linux-gnu/libc.so.6`; `/usr/include/x86_64-linux-gnu/asm/unistd_64.h`; `/tmp/exit_group_probe.c` | Important only to avoid conflating libc `_exit` with raw `SYS_exit`. |

## Asterinas Implementation Notes

- Entry path: `/root/asterinas/kernel/src/syscall/exit.rs:9` constructs `TermStatus::Exited(exit_code as _)` and calls `do_exit(term_status)`.
- Relevant helpers: `/root/asterinas/kernel/src/process/posix_thread/exit.rs:26`, `/root/asterinas/kernel/src/process/term_status.rs:11`, `/root/asterinas/kernel/src/syscall/wait4.rs:50`, `/root/asterinas/kernel/src/syscall/waitid.rs:69`.
- Argument validation: There is no separate syscall-layer validation; the ABI argument is intentionally truncated to `u8`, which matches Linux's wait-visible 8-bit normal-exit status.
- Return / errno behavior: `sys_exit` has no error path. It returns `0` only if `do_exit` ever returns, but the intended observable behavior is immediate thread termination.
- Side effects: The shared exit path updates the process exit code unless an `exit_group` is already in progress or `execve` has taken over; it removes non-leader threads immediately but keeps the main thread in the thread table until reap.
- Explicit TODOs or unimplemented paths: No syscall-local TODO was found. The nearby `waitid` path still lacks `CLD_DUMPED`/`CLD_TRAPPED`, but that is not specific to normal `exit`.

## Comparison Matrix

| Case | Inputs / Preconditions | Linux Semantics | Asterinas Status | Class | Evidence | Test Status |
|------|------------------------|-----------------|------------------|-------|----------|-------------|
| Normal exit-status truncation | `exit_code` has bits outside the low 8 bits. | Parent sees only the low 8 bits. | Aligned in code: `sys_exit` truncates to `u8`; `TermStatus::Exited` encodes normal exit as `status << 8`; `wait4` and `waitid` decode from that representation consistently. | `bug` not confirmed | `/root/asterinas/kernel/src/syscall/exit.rs:9`; `/root/asterinas/kernel/src/process/term_status.rs:11`; `/root/asterinas/kernel/src/syscall/wait4.rs:50`; `/root/asterinas/kernel/src/syscall/waitid.rs:69` | Host-Linux probes passed. No committed Asterinas general test currently checks this exact truncation case. |
| Raw `SYS_exit` thread-local semantics | One thread exits via raw `SYS_exit` inside a multithreaded process. | Only the calling thread dies; process exit status comes from the last exiting thread. | Aligned in code: `sys_exit` routes to `do_exit`, not `do_exit_group`; `exit_internal(..., false)` exits one thread and only calls `exit_process` when the last thread leaves. | `bug` not confirmed | `/root/asterinas/kernel/src/process/posix_thread/exit.rs:26`; `/root/asterinas/kernel/src/process/posix_thread/exit.rs:40`; `/root/asterinas/kernel/src/process/task_set.rs:56` | `process/exit/exit_code` passed on Linux. Asterinas batched validation reached userspace, but the run later failed in `exit_procfs`, so this case is not fully re-confirmed in the same QEMU batch. |
| Procfs-visible thread counts during leader/non-leader exit | Parent inspects `/proc/$PPID/status` while threads exit via raw `SYS_exit`. | Main-thread exit should leave the leader visible until reap; non-leader exit should reduce `Threads:` count immediately. | Expected to align with the explicit leader-removal special case in the exit path. | `open-question` | `/root/asterinas/kernel/src/process/posix_thread/exit.rs:77`; `/root/asterinas/test/initramfs/src/apps/process/exit/exit_procfs.c:31`; `/root/asterinas/test/initramfs/src/apps/process/exit/exit_procfs.c:55` | `process/exit/exit_procfs` passed on Linux. The Asterinas batched run failed at runtime with `exit_procfs: applet not found`, so this case remains unconfirmed in-kernel. |
| `waitid(WSTOPPED|WNOHANG)` must not consume an already-exited child | Child has already exited normally; parent polls only for stopped children. | `waitid` should return success without consuming the exit, and a later `waitpid` should still reap the child. | No contradiction found in the reviewed `waitid`/wait state flow. | `open-question` | `/root/asterinas/kernel/src/process/wait.rs:58`; `/root/asterinas/kernel/src/process/wait.rs:180`; `/root/asterinas/test/initramfs/src/apps/process/exit/exit_waitid_filter.c:8` | Local untracked workspace test passed on Linux. Asterinas-only rerun was invalid because a concurrent QEMU run held `./test/initramfs/build/ext2.img`. |

## Unsupported Semantics

- None recorded yet.

## Bug Candidates

- None recorded yet.

## Verified Bugs And Tests

- None verified yet.

## Candidate Regression Tests

- Add a committed `process/exit` general test that issues `syscall(SYS_exit, 0x1234)` and asserts both `WEXITSTATUS(status) == 0x34` and `waitid(..., WEXITED)` returning `si_code == CLD_EXITED`, `si_status == 0x34`. This was not implemented in this turn because no bug was confirmed and the current Asterinas validation surface was already partially blocked by the `exit_procfs` runtime failure and one invalid overlapping QEMU run.

## Implemented Tests

- None in this turn.

## Validation Status

- `python3 /root/.codex/skills/asterinas-test/scripts/run_targets.py verify process/exit/exit_code process/exit/exit_procfs process/exit/exit_waitid_filter` passed the Linux phase for all three targets.
- The same batched command booted Asterinas successfully but failed during the general-test phase with `exit_procfs: applet not found`. Log: `/tmp/asterinas-run-test-process-exit-exit_code-process-exit-exit_procfs-process-exit-exit_waitid_filter-20260325T063240Z.log`.
- An isolated Asterinas-only rerun of `process/exit/exit_waitid_filter` was invalid because another concurrent QEMU run held `./test/initramfs/build/ext2.img`, so that result was excluded from the semantic judgment. Log: `/tmp/asterinas-run-test-process-exit-exit_waitid_filter-20260325T063542Z.log`.

## Open Questions

- Why does the existing Asterinas `process/exit/exit_procfs` general test fail with `exit_procfs: applet not found` after boot, even though the binary is built into the `process-test` Nix package? This looks like a test packaging/runtime issue rather than a reviewed `sys_exit` mismatch, but it should be resolved before treating the procfs-visible leader-exit case as fully revalidated on Asterinas.
- The current workspace contains an untracked `/root/asterinas/test/initramfs/src/apps/process/exit/exit_waitid_filter.c` and a matching invocation in `/root/asterinas/test/initramfs/src/apps/process/run_test.sh`, so part of the validation surface in this review is local to the worktree rather than a committed baseline.
