# Traceability Template

Use this when mapping test obligations back to source specs.

## Goal

A validator should be able to answer:

* which clause forces this case?
* which cases defend this clause?
* which important clauses still lack runtime or review coverage?

## Traceability Summary Format

```md
## Traceability Summary

### Contract clauses to cases

- `<spec path>:<section or clause>`
  - `T1` checks zero-length success semantics.
  - `T4` checks failure does not over-claim committed bytes.

### Forbidden drifts to cases

- `<forbidden drift>`
  - `T2`, `T3`

### Linux semantic notes to cases

- `<linux note>`
  - `T6`

### Review-only obligations

- `<obligation>`
  - static review only: `R1`
```

## Case-Level Traceability Format

Each case should cite the narrowest relevant sources:

* contract clauses
* invariants
* scenario contracts
* forbidden drifts
* Linux semantic notes when they justify visible behavior

## Gaps

If some clauses are not yet covered, say so explicitly.

Example:

* durability distinction after `sync_all` is only partially covered; runtime persistence harness still needed.
