[TITLE]
Ext2 inode data I/O contract example: `write_at`

[SCOPE]
- Boundary: API-level
- Covered APIs or scenarios:
  - `write_at`
  - successful buffered write
  - successful direct write
- Out of scope:
  - one exact helper sequence
  - one exact lock choreography
- Allowed internal flexibility:
  - buffered and direct paths may use different helpers

[STATE MODEL]
Relevant in-memory state:
- `Inode::type_`
- `Inode::inner`
- `InodeInner.desc`
- `InodeInner.page_cache`
- `InodeInner.backend`

Relevant on-disk state:
- file data blocks reachable from the inode block mapping
- on-disk inode fields mirrored by `InodeInner.desc`
- allocation metadata needed to make committed data reachable

[GLOBAL INVARIANTS]
- `InodeInner.desc` is the authoritative in-memory mirror of inode persistence state.
- `InodeInner.page_cache` must not expose stale bytes that contradict the committed logical file view.
- Successful writes preserve ext2-valid inode and block-mapping state.

[API CONTRACTS]

## API: `write_at`
[PURPOSE]
Write file data starting at byte offset `offset`.

[DOMAIN]
- Defined for inode types that support regular file-data writes.
- For inode types outside this domain, the API must fail rather than fabricate regular-file write semantics.

[PRECONDITIONS]
- `offset` is a valid VFS write position.
- `len` is the requested byte count.
- The inode is live and writable.

[OBSERVABLE EFFECTS]
- May modify logical file contents in a committed prefix of the requested range.
- May extend logical file size.
- May dirty page-cache state and inode metadata.

[IN-MEMORY STATE POSTCONDITIONS: SUCCESS]
- If the API returns `Ok(n)`, then `0 <= n <= len`.
- `InodeInner.desc.size` equals the new logical EOF if `offset + n` exceeds old EOF.
- Successful content-changing writes update `mtime`.
- Successful content-changing writes update `ctime` when inode-change semantics require it.
- Buffered success leaves `InodeInner.page_cache` serving the committed bytes for `[offset, offset+n)`.
- Successful direct write must invalidate or update overlapping cached stale pages.

[ON-DISK STATE POSTCONDITIONS: SUCCESS]
- Buffered success does not by itself imply data durability at return.
- Direct-write success means backing storage for the committed range has received the new bytes as the primary data path.
- Required inode and allocation metadata may still be dirty but not yet fully durable unless a stronger sync contract applies.

[LOGICAL POSTCONDITIONS: SUCCESS]
- Logical bytes in `[offset, offset+n)` equal the first `n` bytes of the caller buffer.
- No byte outside `[offset, offset+n)` is implied by the return value to have been written.
- A later successful `read_at(offset, n)` returns those bytes unless a later successful operation changed that range.

[FAILURE POSTCONDITIONS]
- Failure must not over-report written bytes.
- Failure may preserve a committed prefix already made visible before error, but must not claim more than that prefix.
- Failure must not leave `InodeInner.desc.size` inconsistent with visible logical EOF.
- Failure must not leave ext2-invalid block-mapping state.

[PRESERVED INVARIANTS]
- `InodeInner.desc` remains coherent with visible logical file state.
- `InodeInner.page_cache` must not serve stale overlapping bytes after a range already reported successful.

[ALLOWED INTERNAL FLEXIBILITY]
- The implementation may choose different allocation, rollback, and cache-coherence helpers.
- The contract does not freeze one exact BIO pipeline or one exact locking strategy.

[LINUX SEMANTIC NOTES]
- Match Linux committed-prefix accounting and direct-I/O-visible cache-coherence semantics.

[CROSS-API INVARIANTS]
- A successful `write_at(offset, buf)` returning `Ok(n)` is reflected by later successful `read_at(offset, n)` unless a later successful operation changed the range.

[FORBIDDEN DRIFTS]
- Do not let successful direct writes leave stale overlapping cache data externally observable.
- Do not let success overstate committed bytes.
- Do not let `InodeInner.desc.size` drift away from visible logical EOF.
