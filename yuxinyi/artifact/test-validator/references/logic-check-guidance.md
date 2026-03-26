# Logic Check Guidance

Use the `.test` file as a validator contract for code inspection.

## Goal

For each material `.test` case, ask:

* does the code path enforce the prestate assumptions?
* does the success path satisfy the expected result?
* does the failure path preserve the required invariants?
* does cleanup occur when the case requires it?

## What To Inspect

### Entry validation

Check:

* type guards
* argument validation
* alignment checks
* overflow checks

### Success path

Check:

* returned value
* visible state update
* metadata update
* read-after-write or other cross-API guarantees

### Failure path

Check:

* rollback
* non-overclaiming error return
* preserved old state
* no contradictory cache or metadata state

### Mode-specific behavior

Check separate branches for:

* buffered vs direct
* sync-related differences
* allocation vs no-allocation
* replacement vs no-replacement

## Status Labels

For code logic, classify each material case as:

* `logically satisfied`
* `partially evidenced`
* `contradicted`
* `not statically provable`
* `unclear`

## Important Distinction

`not covered by runtime test` is not the same as `code failure`.

Keep these separate:

* implementation gap
* missing automated test
* missing `.test` obligation
