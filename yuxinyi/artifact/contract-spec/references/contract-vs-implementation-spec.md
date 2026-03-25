# Contract Spec Versus Implementation Spec

Do not mix these artifacts.

## Contract Spec

A contract spec is:

* stable
* validation-oriented
* state-aware
* durable across refactors

It answers:

* what must always remain true
* what exact state relations must hold
* what success means exactly
* what partial success means exactly
* what failure must still preserve
* what invariants validators should enforce
* what semantic drift is forbidden

Typical contract content:

* state model
* global invariants
* API contracts
* scenario contracts
* cross-API invariants
* forbidden drifts

Good examples:

* If `write_at` returns `Ok(n)`, then `0 <= n <= len`.
* `InodeInner.desc.size` matches logical EOF after successful resize.
* Successful direct writes must invalidate or update overlapping stale cached pages.
* `sync_all()` success subsumes `sync_data()` obligations.

## Implementation Spec

An implementation spec is:

* change-specific
* execution-oriented
* tied to one task or refactor
* often temporary

It answers:

* what must change now
* which helpers or APIs must be introduced or removed
* what call graph must be reshaped
* what algorithm phases the implementation must follow
* what edit scope applies to this patch

Good implementation-spec examples:

* `rename` no-replacement path must call helper `x`
* downgrade write guard before page-cache I/O in this refactor
* replace helper `old_scan` with `scan_dir_for_slot`

## Quick Test

Ask:

* Would this clause still be true after a clean internal rewrite that preserves behavior and state guarantees?

If yes, it belongs in the contract spec.

If no, it probably belongs in the implementation spec.

## Common Confusion

These belong in the contract spec:

* exact logical postconditions
* exact in-memory success state
* exact on-disk success state
* exact failure postconditions
* exact durability scope

These usually do not:

* which helper is called first
* which temporary lock transition one refactor uses
* which file the helper lives in
