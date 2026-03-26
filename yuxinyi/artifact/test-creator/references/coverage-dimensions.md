# Coverage Dimensions

Start from coverage axes before inventing cases.

Pick the smallest set that explains the scenario. Do not include irrelevant axes.

## Common Dimensions For File And Filesystem APIs

### Input boundaries

* zero length
* minimum valid offset
* exact EOF
* one byte or one block before EOF
* one byte or one block after EOF
* maximum representable range or overflow-adjacent inputs

### Mode splits

* buffered vs direct
* no-sync vs data-sync vs full-sync
* replacement vs no-replacement
* same-directory vs cross-directory

### Completion semantics

* full success
* committed prefix
* fail-before-commit
* fail-after-partial-side-effects

### State domains

* logical file contents
* logical EOF
* in-memory metadata
* on-disk reachability
* persistence durability
* page-cache visibility

### Metadata updates

* `atime`
* `mtime`
* `ctime`
* link count
* allocation accounting

### Failure and cleanup

* invalid arguments
* backing-store failure
* ENOSPC-style allocation failure
* writeback failure
* cleanup after failed EOF extension

### Cross-API consistency

* read-after-write
* resize vs metadata size
* sync effects after successful mutation
* rename and readdir / lookup agreement

### Concurrency-sensitive corners

* racing readers vs extending writes
* stale-cache exposure
* lock-drop revalidation points
* atomicity expectations at API boundaries

## Selection Heuristic

For each axis, ask:

* can this change visible behavior?
* can this change the expected oracle?
* can this fail differently?
* can this expose stale or contradictory state?

If yes, it deserves at least one case in the matrix.
