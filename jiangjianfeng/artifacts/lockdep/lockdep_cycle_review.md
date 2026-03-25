# Lockdep Cycle Review

Historical note:

- This file is a point-in-time review of an older cycle report.
- Several conclusions here are no longer current implementation status:
  - the old heuristic reverse-edge normalization has been removed
  - real branch-dependent reverse lock orders are now preserved
  - lock identity and local wrapper/callback propagation are materially stronger than when this review was written

This document reviews the 5 cycles currently reported by the static lockdep prototype
against `aster-kernel` on `x86_64-unknown-none`.

## Summary

Current conclusion:

- Cycle 1 (`futex::lock_bucket_pairs`): false positive.
- Cycle 2 (`exfat::rename` / `ExfatFs::evict_inode`): false positive.
- Cycle 3 (`VsockSpace::poll`): false positive.
- Cycle 4 (`ext2::read_lock_two_inodes`): false positive.
- Cycle 5 (`ext2::write_lock_two_inodes`): false positive.

The reported 5 cycles are useful because they expose real precision problems in the analyzer,
but they do not currently look like actionable deadlocks in the kernel.

The false positives fall into three buckets:

1. Missing path sensitivity for ordered double-lock helpers.
2. Missing lock-mode compatibility for `RwLock` / `RwMutex`.
3. Unstable lock-class identity for local MIR places such as `(*_4)`.

## Cycle 1: `process::posix_thread::futex::lock_bucket_pairs`

Relevant code:

- [futex.rs](/root/asterinas-codex/kernel/src/process/posix_thread/futex.rs#L355)
- [futex.rs](/root/asterinas-codex/kernel/src/process/posix_thread/futex.rs#L376)
- [futex.rs](/root/asterinas-codex/kernel/src/process/posix_thread/futex.rs#L381)

Reported pattern:

- bucket A -> bucket B
- bucket B -> bucket A

Why this is not a real deadlock:

- The helper explicitly sorts by bucket index before taking the second lock.
- The two reported edges come from mutually exclusive branches of the same `match index_1.cmp(&index_2)`.
- At runtime, only one order is possible for a given pair.

Verdict:

- False positive.

Why the analyzer reports it:

- It merges effects from both branches into one function summary.
- It does not preserve the path predicate `index_1 < index_2` vs `index_1 > index_2`.

How to eliminate this false positive:

- Add path-sensitive branch handling for ordered double-lock idioms.
- Detect comparator-ordered helpers as a special pattern:
  - compare two keys or indices;
  - acquire `lhs` then `rhs` in one branch;
  - acquire `rhs` then `lhs` in the opposite branch;
  - normalize both branches into one canonical order edge.
- A simpler first step is to suppress reversed edges when both branches are proven to be selected by the same total-order comparison.

## Cycle 2: `exfat::rename` and `ExfatFs::evict_inode`

Relevant code:

- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/exfat/inode.rs#L1649)
- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/exfat/inode.rs#L1660)
- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/exfat/inode.rs#L1670)
- [fs.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/exfat/fs.rs#L141)

Reported pattern:

- inode-inner `read` -> fs inode map `read`
- fs inode map `read` -> inode-inner `read`
- another inode-inner `read` closes the cycle

Why this is not a real deadlock:

- All reported acquisitions in this cycle are `RwMutex(read)`.
- Two read locks on the same `RwMutex` do not block each other.
- A cycle made entirely of shared-read acquisitions is not a deadlock.

Verdict:

- False positive.

Why the analyzer reports it:

- It currently treats `read` and `write` as equally blocking lock acquisitions when building the graph.
- It lacks a lock-mode compatibility matrix.

How to eliminate this false positive:

- Model lock modes explicitly in the dependency graph.
- At minimum:
  - do not report `read -> read` cycles for `RwLock` / `RwMutex`;
  - distinguish `read`, `write`, and later `upgrade`.
- Better:
  - keep a compatibility table per primitive and mode;
  - only create deadlock-relevant edges for conflicting mode pairs.

## Cycle 3: `net::socket::vsock::common::VsockSpace::poll`

Relevant code:

- [common.rs](/root/asterinas-codex/kernel/src/net/socket/vsock/common.rs#L218)
- [common.rs](/root/asterinas-codex/kernel/src/net/socket/vsock/common.rs#L267)
- [connected.rs](/root/asterinas-codex/kernel/src/net/socket/vsock/stream/connected.rs#L113)
- [connected.rs](/root/asterinas-codex/kernel/src/net/socket/vsock/stream/connected.rs#L118)

Reported pattern:

- `RwLock(read)` on `connected_sockets` -> `SpinLock(lock)` on some `(*_4)`
- `SpinLock(lock)` on some `(*_4)` -> `RwLock(read)` on `connected_sockets`

Why this is not a real deadlock:

- The `SpinLock` node is identified only as `(*_4)`, which is a raw MIR-local place string.
- That identifier is not stable across functions and can accidentally merge unrelated locks.
- In the source, `VsockSpace::poll` clearly takes `connected_sockets.read()` and then calls methods like `connected.update_info()` / `connected.get_info()`, which acquire `Connected::connection`.
- There is no evidence here of the reverse order, i.e. `Connected::connection` being held and then `connected_sockets.read()` being acquired on the same path.

Verdict:

- False positive, caused by unstable lock-class identity.

Why the analyzer reports it:

- For locals and dereferenced temporaries, it currently uses `format!("{receiver:?}")` as the lock-class key.
- Names like `(*_4)` are only meaningful inside one MIR body, but the global graph merges them across functions.

How to eliminate this false positive:

- Stop using raw MIR place strings as global lock-class identity.
- Replace them with a stable lock-class scheme:
  - static/global lock: `DefId`;
  - field lock: receiver root + projection path + field id + lock type;
  - local allocation lock: function `DefId` + allocation site / local origin;
  - fallback: include the enclosing function `DefId` in the lock-class key so `(*_4)` from different functions cannot collide.
- This is the highest-value precision fix in the current prototype.

## Cycle 4: `ext2::read_lock_two_inodes`

Relevant code:

- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/ext2/inode.rs#L920)
- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/ext2/inode.rs#L927)
- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/ext2/inode.rs#L931)
- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/ext2/inode.rs#L935)

Why this is not a real deadlock:

- The helper orders the two inode locks by `ino`.
- The reported two directions come from the two mutually exclusive branches.
- The mode is `RwMutex(read)`, so even the apparent cycle is read/read only.

Verdict:

- False positive.

Why the analyzer reports it:

- Same two issues as above:
  - missing path sensitivity for ordered double-lock helpers;
  - missing read/read compatibility handling.

How to eliminate this false positive:

- Apply the same fixes as Cycle 1 and Cycle 2.

## Cycle 5: `ext2::write_lock_two_inodes`

Relevant code:

- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/ext2/inode.rs#L938)
- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/ext2/inode.rs#L945)
- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/ext2/inode.rs#L949)
- [inode.rs](/root/asterinas-codex/kernel/src/fs/fs_impls/ext2/inode.rs#L953)

Why this is not a real deadlock:

- This helper also orders by `ino` before taking the second write lock.
- So although this is a blocking mode (`write`/`write`), the runtime order is still canonical.

Verdict:

- False positive.

Why the analyzer reports it:

- Missing path sensitivity for ordered double-lock helpers.

How to eliminate this false positive:

- Same as Cycle 1:
  - track branch predicates, or
  - recognize order-normalizing helpers and canonicalize the resulting edges.

## Recommended Fix Order

The most effective order to reduce false positives is:

1. Stable lock-class identity.
   - This removes bogus cross-function merges like `(*_4)`.
2. Lock-mode compatibility.
   - This removes read/read-only cycles for `RwLock` and `RwMutex`.
3. Ordered double-lock recognition.
   - This removes `lock_bucket_pairs`, `read_lock_two_inodes`, `write_lock_two_inodes`, and similar helpers.

## Practical Next Step

Before adding more deadlock rules, the analyzer should implement these two concrete changes:

- Change `LockClass` identity so local places are keyed by at least:
  - enclosing function `DefId`,
  - local/allocation origin,
  - projection path,
  - lock primitive and mode.
- Add a compatibility filter so graph cycles are reported only when every edge in the witness cycle can participate in a blocking cycle.

Without these two changes, the current cycle report is useful for finding modeling gaps,
but not yet reliable enough for kernel triage.
