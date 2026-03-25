# Spec To Test Mapping

Use this mapping workflow to turn a spec into concrete cases.

## Step 1: Classify The Clause

Each clause usually falls into one of these buckets:

* precondition
* success postcondition
* failure postcondition
* preserved invariant
* cross-API invariant
* forbidden drift
* Linux-visible semantic note

## Step 2: Choose The Validation Form

Map each bucket to one or more validation forms:

* precondition:
  `integration test`, sometimes `unit test`
* success postcondition:
  `integration test`
* failure postcondition:
  `integration test`, `fault injection`
* preserved invariant:
  `integration test`, `static review`
* cross-API invariant:
  `integration test`, sometimes `concurrency review`
* forbidden drift:
  whichever mode is strongest at catching it
* Linux-visible semantic note:
  `integration test` or `static review`, depending on observability

If the repository context is known, also choose a likely implementation target:

* local helper or local invariant:
  usually `unit test`
* syscall-visible or namespace-visible semantic:
  usually `integration test`
* persistence matrix, stress, or broad compatibility surface:
  may be deferred to a heavier harness

## Step 3: Materialize The Case

For each important clause, specify:

* prestate
* action
* expected result
* expected in-memory effects
* expected persistence effects
* failure oracle

## Step 4: Split By Mode When Needed

Do not hide materially different semantics inside one case.

Split when the spec distinguishes:

* buffered vs direct
* sync domain differences
* different namespace scenarios
* different cleanup paths

## Step 5: Assign Priority

Use:

* `MUST` for core semantic or regression-defending clauses
* `SHOULD` for important but secondary coverage
* `OPTIONAL` for expensive or niche cases

## Step 6: Add Validator Notes

Mark cases that are better handled by:

* static review
* code inspection
* fault injection
* concurrency review

When a later writing agent will consume the artifact, also add:

* suggested harness
* likely target module or directory
* any case that should stay deferred rather than being forced into a weak local test

## Compact Mapping Examples

### `read_at` clause: zero-length success

Clause type:
success postcondition

Likely case:

* prestate: file may contain arbitrary data
* action: call `read_at(offset, len=0, ...)`
* expected result: `Ok(0)`
* expected in-memory effects: no logical content or EOF change
* expected persistence effects: none

### `write_at` clause: later read must observe written bytes

Clause type:
cross-API invariant

Likely case:

* prestate: writable regular file
* action: successful `write_at(offset, payload)` then `read_at(offset, len(payload))`
* expected result: read returns written payload unless a later successful operation changed that range
* expected persistence effects: visibility does not require sync, durability may still depend on later sync
