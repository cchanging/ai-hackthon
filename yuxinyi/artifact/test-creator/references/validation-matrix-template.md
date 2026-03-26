# Validation Matrix Template

Use a matrix before or alongside the full case inventory.

## Recommended Columns

* `Case ID`
* `Scenario`
* `Coverage dimensions`
* `Level`
* `Validation mode`
* `Primary oracle`
* `Suggested implementation target`
* `Spec traceability`
* `Notes`

## Example

```md
| Case ID | Scenario | Coverage dimensions | Level | Validation mode | Primary oracle | Suggested implementation target | Spec traceability | Notes |
|---------|----------|---------------------|-------|-----------------|----------------|---------------------------------|-------------------|-------|
| T1 | `read_at` zero length | zero-length, no-op, metadata stability | MUST | integration test, static review | returns `Ok(0)` | userspace integration test | `read_at` success rules, forbidden drift on fabricated bytes | low infra |
| T2 | `write_at` extends EOF | EOF growth, read-after-write, metadata coherence | MUST | integration test | full-length success and later read visibility | userspace integration test | `write_at` size-growth clauses | high value |
| T3 | direct write stale-cache overlap | direct-vs-buffered split, cache coherence | MUST | integration test, fault injection | no stale bytes after direct write | integration test plus hook-based review | direct-write forbidden drift | may need cache control hooks |
```

## Matrix Rules

* Every `MUST` case should have a primary oracle.
* If a case requires special infrastructure, say so in `Notes`.
* Do not collapse unrelated semantics into one giant matrix row.
* If a single scenario has distinct modes, split it into separate rows.
* When the repository is known, fill `Suggested implementation target` with the likely harness and path family rather than leaving it implicit.
