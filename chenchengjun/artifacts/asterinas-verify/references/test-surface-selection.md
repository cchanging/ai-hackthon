# Validation Plan Selection

Use only these outcomes in this workflow.

## `general test + verify`

Use when:

- the behavior is externally visible
- the module under `<repo>/test/initramfs/src/apps` is clear
- Linux and Asterinas parity is a meaningful oracle
- `./scripts/select_test_family.py` returns `validation_feasible=true`

## `report-only`

Use when:

- the behavior is internal or adoption-only
- the oracle is not crisp enough for a user-visible general test
- no obvious top-level test module owns the regression
- `./scripts/select_test_family.py` reports no general test module

## Rules

- Prefer `general test + verify` for user-visible behavior.
- Keep the finding `report-only` when the best result is a candidate regression scenario rather than an executable general test.
- If uncertainty remains in a user-visible corner case, a targeted confirmation test may be authored early, but still prefer batching those confirmation runs instead of one-off execution loops.
