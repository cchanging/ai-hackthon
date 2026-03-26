# Contract Spec Principles

Use these principles to write strong validation-oriented contract specs.

## Core Goal

A contract spec freezes stable semantic and state constraints that must survive future refactors.

It is not a temporary plan for one patch.

## What Strong Contracts Capture

Strong contracts capture:

* the stable contract boundary
* the local state model when known
* exact preconditions
* observable effect boundaries
* in-memory state postconditions on success
* on-disk state postconditions on success when relevant
* logical postconditions on success
* failure postconditions
* preserved invariants
* cross-API invariants
* scenario semantics
* forbidden drift
* allowed internal flexibility

## What Strong Contracts Avoid

Do not encode:

* helper extraction plans
* temporary migration steps
* one refactor's edit order
* current helper naming
* one exact lock choreography
* one exact control-flow shape

Those belong to implementation work unless they are permanent semantic requirements.

## State-Oriented Writing

When the local model is known, name exact state directly.

Prefer:

* `InodeInner.desc`
* `InodeInner.page_cache`
* block mapping state
* dirty flags
* inode mirrors
* on-disk inode fields
* allocation metadata

Avoid replacing known local state with vague phrases like:

* inode metadata
* cache state
* internal structures

unless the local model is genuinely unknown.

## Five Required Distinctions

Keep these separate:

1. logical postconditions
2. in-memory state postconditions
3. on-disk state postconditions
4. durability guarantees
5. allowed internal flexibility

If these are mixed together, the contract becomes weak and hard to validate.

## Strong Success Clauses

For important APIs, success clauses should answer:

* what exactly became visible
* what exact range or object was affected
* what exact state fields changed
* what exact state fields must still agree
* whether success may be partial
* what the returned value means exactly

Examples:

* If the API returns `Ok(n)`, then `0 <= n <= len`.
* Logical bytes in `[offset, offset+n)` equal the first `n` bytes of the caller buffer.
* `InodeInner.desc.size` reflects the new logical EOF.
* `InodeInner.page_cache` contains or coherently invalidates overlapping state.

## Observable Effects Must Constrain, Not Narrate

`[OBSERVABLE EFFECTS]` should define the allowed visible-effect boundary.

It should answer:

* what kinds of visible effects are allowed
* how far those effects may extend
* what visible effects are forbidden

Prefer:

* The only logical bytes this API may newly expose are bytes in `[offset, offset+n)`.
* The API may change allocation state only as needed to support the committed range.
* The API must not create visible effects outside the requested object or range.

Avoid weak narrative-only bullets like:

* may modify metadata
* may update cache
* may allocate blocks

unless they are immediately narrowed into validator-checkable boundaries.

## Strong Failure Clauses

Failure clauses should answer:

* what may already be committed
* what must not be over-reported as success
* what state must remain unchanged
* what state must remain coherent
* what structural validity must still hold

Prefer:

* Failure may preserve a committed prefix already made visible before error, but must not over-report it.
* Failure must not leave `desc.size` inconsistent with visible logical EOF.
* Failure must not leave block mapping state ext2-invalid.

Avoid:

* must not leave invalid state
* must remain consistent

unless followed immediately by exact, checkable obligations.

## Cross-API Invariants Matter

Add cross-API invariants when one API contract is not enough.

Typical examples:

* `seek_end()` agrees with `metadata().size`
* successful `write_at` is reflected by later successful `read_at`
* `sync_all()` subsumes `sync_data()` obligations
* rename results are reflected by later lookup and readdir behavior

## Split Mode-Specific Contracts When Externally Relevant

Split one API into mode-specific subcontracts when the difference changes visible or validator-relevant semantics.

Good candidates:

* buffered write versus direct write
* data sync versus full sync
* hole read versus mapped read if returned bytes differ semantically

Do not split on internal branches that are externally invisible.

## Boundary Test

Ask:

* Would this clause still be true after a clean internal rewrite that preserves behavior?

If yes, it belongs in the contract spec.

If no, it probably belongs in an implementation spec.
