---
name: ext2-concurrency-review
description: review a specified ext2 feature or method for deadlocks, unsafe lock-release windows, and other synchronization hazards. use when an agent needs to audit the full concurrency behavior of a path such as write_at, rename, unlink, truncate, reclaim, or page-cache callbacks, especially when lock ordering, release/reacquire choreography, vfs-to-ext2 atomicity, or linux-visible behavior under races must be validated across the complete call graph.
---

# Ext2 Concurrency Review

Review Ext2 changes as a concurrency specialist.

Inspect a specified Ext2 feature or method across its full reachable call graph, prove or disprove concurrency safety, and save a markdown report under `.trellis/reports/concurrency/`.

## Current Lock Baseline

Start from the current ext2 lock model in the codebase, not from Linux locking intuition.
If the reviewed code introduces a new nesting pattern outside this baseline, treat it as suspicious until proven safe.

### Lock-bearing structures and what they protect

* `Inode.inner: RwMutex<InodeInner>`
  Protects the in-memory inode mirror `desc`, the per-inode `PageCache`, and the `Arc<InodeBackend>` handle stored in `InodeInner`.
* `InodeBackend.block_map: RwMutex<InodeBlockMap>`
  Serializes page-cache backend traversal against foreground block-map mutation. This is the main lock used by pager callbacks.
* `InodeBlockMap.indirect_blocks: Mutex<IndirectBlockManager>`
  Protects cached indirect-block buffers and on-demand indirect-block traversal/update state inside the block-map domain.
* `Inode.xattr: Option<RwMutex<Xattr>>`
  Protects xattr block state independently from `InodeInner`.
  Current code avoids co-holding `xattr` with `inner`; any new nested `xattr` plus `inner` acquisition needs explicit review.
* `Ext2.super_block: RwMutex<Dirty<SuperBlock>>`
  Protects filesystem-wide superblock metadata.
* `BlockGroup.desc: RwMutex<Dirty<GroupDesc>>`
  Protects one block-group descriptor.
* `BlockGroup.block_bitmap: RwMutex<Dirty<IdBitmap>>`
  Protects the block-allocation bitmap for one group.
* `BlockGroup.inode_bitmap: RwMutex<Dirty<IdBitmap>>`
  Protects the inode-allocation bitmap for one group.
* `BlockGroup.inode_cache: RwMutex<BTreeMap<u32, Arc<Inode>>>`
  Protects the per-group live inode cache.

### Implicit locks in page-cache and VMO callback paths

These locks are outside ext2 proper, but they are part of the real callback lock stack and must be reviewed as such:

* `PageCacheManager.pages: Mutex<LruCache<usize, CachePage>>`
  Held across page-cache lookup, eviction, discard, and some backend callback entry points.
* `PageCacheManager.ra_state: Mutex<ReadaheadState>`
  In `ondemand_readahead`, acquired after `pages` and held while calling into the backend.
* `Vmo.pages`
  This is the VMO page-store lock, but pager callbacks are not entered while it is held on commit and are entered only after it is dropped on decommit.
  Reviewers should therefore treat the page-cache manager locks, not `Vmo.pages`, as the main implicit callback locks.

### Proven lock ordering rules

Use these as the default expected order:

* Cross-inode operations: acquire `Inode.inner` locks in ascending inode-number order.
* Single-inode foreground mutation: `inner -> block_map -> indirect_blocks`.
* Page-cache callback path: `PageCacheManager.pages -> PageCacheManager.ra_state -> block_map.read -> indirect_blocks` when block lookup descends into indirect blocks.
* Block-group bitmap sync path: read `BlockGroup.desc`-derived bitmap addresses before taking bitmap locks.

### Review triggers that should be treated as suspicious

Flag these immediately and require explicit proof:

* starting any `PageCache` or `Vmo` operation while still holding `block_map.write()` or `indirect_blocks`,
* acquiring `Inode.inner` from a page-cache backend callback path,
* acquiring `xattr` and `inner` together in a new nesting pattern,
* introducing a new global order between `super_block`, block-group locks, and inode locks without an explicit invariant and call-graph audit.

## Workflow

### 1. Define the review target

Start from a named feature or method, for example:

* `write_at`
* `rename`
* `unlink`
* `truncate`
* a page-cache callback path
* a specific Ext2 helper when the user provides it

Then determine the full review scope from that target:

* trace the primary Ext2 entry point,
* include `kernel/src/fs/fs_impls/ext2/impl_for_vfs/` when the feature is reachable from VFS,
* include adjacent kernel helpers when the call path crosses module boundaries,
* use Linux ext2 under `/root/linux/fs/ext2/` as the semantic reference,
* use Asterinas Ext2 specs as the locking and protocol reference.

### 2. Build context before judging

Collect:

* the target feature name and concrete entry methods,
* which locks from the baseline above are on the path,
* reachable helpers and lock helpers,
* nearby helpers and lock helpers,
* relevant VFS entry points,
* relevant Linux reference paths,
* relevant Asterinas spec or protocol notes.

Use codebase retrieval first for semantic understanding when helpful, then read the full reachable code paths directly.
Do not skip the callback side of the path when `PageCache`, `Vmo`, direct I/O, truncate, reclaim, or directory block mutation is involved.

### 3. Apply the three analysis layers

Always run all three:

* lock graph and ordering analysis,
* lock-release window analysis,
* semantic race analysis.

Use [review-rubric.md](./references/review-rubric.md) for the required questions, hotspot list, and verdict rules.
Start lock-graph analysis from the baseline order above and explain any deviation explicitly.

### 4. Classify findings conservatively

Only three finding categories exist:

* `Deadlock`
* `Lock-Release Window`
* `Other`

If safety cannot be proven, do not silently pass. Use `SUSPICIOUS` when the protocol is too unclear to prove safe from the available evidence.

When a finding may affect visible behavior, state whether the divergence is from:

* `Asterinas spec`,
* `Linux semantics`,
* `both`,
* or `unclear`.

### 5. Keep Linux semantics and local locking distinct

Linux is a behavioral reference, not a 1:1 locking template.

Do:

* compare visible behavior under races,
* compare error-path and revalidation semantics,
* compare mutation ordering where correctness depends on it.

Do not:

* flag every structural locking difference from Linux,
* assume local callback-safe choreography is wrong just because Linux looks different.

Use [cross-check-guidance.md](./references/cross-check-guidance.md) for attribution rules and VFS-boundary review expectations.

### 6. Produce the required report

Always emit:

* verdict header,
* findings first,
* lock-release window inventory,
* VFS boundary review,
* saved report path.

Use the exact structure in [report-template.md](./references/report-template.md).
Before sending the final answer, save the full markdown report under `.trellis/reports/concurrency/`
with:

```bash
python3 .trellis/scripts/save_report.py --kind concurrency-check --subdir concurrency --slug <target-slug>
```

Use a short slug derived from the reviewed feature or method.

## Decision Rules

Use:

* `PASS` only when no concrete deadlock, unsafe window, or other material concurrency hazard is found and lock-drop windows are explicitly revalidated.
* `FAIL` when a concrete unsafe path, stale-state window, or concurrency bug is evidenced.
* `SUSPICIOUS` when the pattern may be safe but cannot be proven from the full reachable code and surrounding invariants.

Confidence must be:

* `HIGH` when the evidence is direct,
* `MEDIUM` when conclusions depend on a small amount of unchanged helper behavior,
* `LOW` when major safety claims depend on undocumented or unproven assumptions.

## Evidence Standard

Prefer exact evidence:

* file path and line,
* function and helper names,
* lock acquisition and release points,
* branch-specific behavior,
* state carried across release windows,
* Linux reference function,
* Asterinas spec or protocol note.

Separate proven bugs from plausible-but-unproven suspicion.

## Resources

Read these references as needed:

* [review-rubric.md](./references/review-rubric.md) for the review checklist, hotspot list, and evidence grading.
* [cross-check-guidance.md](./references/cross-check-guidance.md) for Linux-vs-spec attribution rules and VFS-boundary review guidance.
* [report-template.md](./references/report-template.md) for the exact output structure.
* `/root/asterinas/.trellis/scripts/save_report.py` for persisting the final markdown report.
