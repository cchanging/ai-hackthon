# Review Rubric

Use this checklist for every feature review.

## Inputs

Start with:

* the target feature or method name,
* optional seed paths or specific entry methods from the user,
* relevant Linux references if already known.

Examples:

* `write_at`
* `rename`
* `unlink`
* `Inode::add_entry`

If the target is ambiguous, identify the candidate entry points first and say which ones are included in the review.

## Baseline Sources

Start from these Asterinas references when relevant:

* the reachable implementation under `kernel/src/fs/fs_impls/ext2/`,
* VFS entry adapters under `kernel/src/fs/fs_impls/ext2/impl_for_vfs/`,
* `.trellis/spec/kernel/quality-guidelines.md`,
* `.trellis/spec/kernel/error-handling.md`,
* feature-specific Ext2 contract or task artifacts under `.trellis/spec/kernel/` or `.trellis/tasks/` when they define locking or protocol constraints.

Also inspect:

* the full reachable Ext2 call path for the target feature,
* nearby unchanged helpers,
* VFS wrapper entry points,
* Linux ext2 behavior under `/root/linux/fs/ext2/`.

## Finding Categories

Only use these categories:

### Deadlock

Examples:

* lock ordering inversion across lock classes,
* multi-inode locking that does not enforce ascending inode order,
* self-deadlock through recursive helper acquisition,
* synchronous re-entry into Ext2 while holding an incompatible lock,
* upgrade or downgrade choreography blocked by current call-chain ordering,
* cross-layer lock inversion involving VFS, page cache, reclaim, superblock, block group, inode, or xattr paths.

### Lock-Release Window

Examples:

* lock dropped and reacquired without revalidation,
* stale snapshot carried across a lock drop,
* split-phase protocol that assumes old observations remain true,
* dropped lock to avoid deadlock but no post-reacquire safety proof,
* upgrade or downgrade exposing an unsafe intermediate state.

### Other

Examples:

* too-weak lock mode for a mutation,
* inconsistent multi-object snapshot without a valid contract,
* lost-update or duplicate-entry risk,
* lifetime race involving `Arc`, `Weak`, reclaim, or deletion-pending state,
* dirty-state or persist-ordering mismatch under concurrency,
* thin-wrapper violation that creates a race window across the VFS boundary.

## Analysis Layers

### 1. Lock Graph And Ordering

For each reachable path in the reviewed feature:

1. list every lock acquire, release, upgrade, downgrade, and callback,
2. derive the effective lock order,
3. compare it to local rules and nearby unchanged paths,
4. flag any inversion, ambiguity, or re-entry risk.

### 2. Window Analysis

For each lock drop or lock replacement in the reviewed feature:

1. record facts learned before the release,
2. list concurrent operations that can invalidate those facts,
3. check for explicit revalidation after reacquisition,
4. if revalidation is missing or incomplete, flag it.

### 3. Semantic Race Analysis

Check whether the feature's concurrency behavior can alter:

* VFS-to-Ext2 dispatch atomicity,
* inode lifetime,
* directory entry validity,
* link-count correctness,
* block-allocation ownership,
* metadata or page-cache consistency,
* error propagation under races.

## Must-Review Hotspots

Always scrutinize these when they are part of the reviewed feature:

* `rename`
* `unlink`
* `rmdir`
* `truncate`
* `reclaim`
* page-cache backend callbacks
* metadata persistence
* multi-inode operations
* allocation or free paths

## Evidence Grades

Use these internally when deciding confidence:

* `direct`: exact lock behavior or race window is visible in changed or adjacent code.
* `inferred`: the conclusion depends on unchanged helper behavior that appears stable but was not fully expanded.
* `unknown`: safety depends on behavior the review could not inspect or prove.

Base confidence on the weakest important link in the reasoning chain.

## Verdict Mapping

### PASS

Only when:

* no concrete deadlock risk is found,
* no unsafe release/reacquire window is found,
* no other material concurrency hazard is found,
* lock ordering remains explainable,
* any lock-drop window is explicitly revalidated.

### FAIL

Use when:

* a concrete deadlock path exists,
* a concrete stale-state or missing-revalidation window exists,
* a concrete race can cause corruption, wrong errno, wrong visibility, or lifetime bugs,
* the reviewed feature weakens synchronization in an observably unsafe way.

### SUSPICIOUS

Use when:

* the protocol is too complex to prove safe from the available code,
* the code relies on undocumented invariants,
* safety depends on behavior outside the inspected reachable paths,
* the revalidation or lock-order story is incomplete.
