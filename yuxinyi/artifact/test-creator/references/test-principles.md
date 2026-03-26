# Test Principles

Use these rules when converting specs into testing artifacts.

## What This Skill Produces

The goal is not “more tests.”
The goal is a validation artifact that makes drift visible.

A strong testing artifact should tell a later reviewer or test-generation agent:

* what must be tested before merge
* what can be checked statically
* what needs runtime infrastructure
* what failure oracles matter
* which cases protect against future refactors
* which harness and target location best fit each important case when the repo context is known

## Separation Of Responsibilities

Keep these roles distinct:

* contract spec:
  defines what must remain true
* implementation spec:
  defines how a change intends to achieve it
* `.test` artifact:
  defines how to validate those obligations

Do not rewrite the spec while building the `.test`.

## Test Artifact Quality Bar

Good cases are:

* explicit
* traceable
* narrow enough to implement
* strong enough to catch semantic drift
* organized by validation mode

Weak cases to avoid:

* “test normal behavior”
* “test error handling”
* “test concurrency”
* “ensure Linux compatibility”

Those are coverage headings, not implementable obligations.

Another weak pattern to avoid:

* producing a good case inventory with no indication of how a later agent should implement it in the target repository

## Obligation Levels

Use exactly these levels:

* `MUST`: required before merge or required to defend core semantics
* `SHOULD`: strong coverage that materially reduces drift risk
* `OPTIONAL`: useful but lower-value or infra-heavy coverage

## Validation Modes

Use one or more of:

* `static review`
* `unit test`
* `integration test`
* `fault injection`
* `concurrency review`

Keep validator-oriented review obligations separate from runtime cases.
When the repository is known, separate runtime cases by likely harness rather than leaving that decision fully implicit.

## Oracles

Prefer precise oracles:

* returned bytes or errno
* logical EOF
* metadata timestamps or link count
* page-cache visibility / stale-data absence
* committed-prefix behavior
* post-failure surviving state
* persistence distinction after `sync_data` vs `sync_all`

When possible, state both:

* what must happen
* what must not happen

## Recommended Output Order

1. Title
2. Scope
3. Source specs
4. Coverage dimensions
5. Validation matrix
6. Case inventory
7. Traceability summary
8. Test generation notes

## Compact Filesystem Examples

### Example: `read_at` zero length

Why it matters:
guards the no-op success path and protects refactors from accidental metadata or data changes.

Validation modes:
`integration test`, `static review`

Expected result:
returns `Ok(0)`.

Expected in-memory effects:
no logical file-content change; no EOF change.

Expected persistence effects:
none required.

### Example: `write_at` extends EOF

Why it matters:
guards size growth, later read visibility, and metadata coherence.

Validation modes:
`integration test`, optionally `fault injection`

Expected result:
successful write returns full length.

Expected in-memory effects:
logical EOF grows; later read of written range matches caller bytes.

Expected persistence effects:
non-sync visibility may exist without full durability; later sync strengthens persistence only.
