# Delivery Template

Use this template when the requested outcome is actual test code rather than only a plan.

```md
## Semantic Target

- Feature or API: `<name>`
- Source of truth: `<spec / .test / failure analysis / Linux semantic note>`
- Main obligations:
  - `<obligation 1>`
  - `<obligation 2>`

## Test Placement

- Unit tests (`#[ktest]`):
  - `<absolute kernel path>`
- Integration tests:
  - `/root/asterinas/test/initramfs/src/apps/fs/ext2/<file>.c`

## Coverage Mapping

- `#[ktest]` coverage:
  - `<local invariant / branch / helper semantic>`
- Integration coverage:
  - `<syscall-visible semantic>`
- Deferred to xfstests or another harness:
  - `<e2e, stress, persistence matrix, or none>`

## Patch Shape

- Added files:
  - `<path>`
- Updated files:
  - `<path>`

## Notes For Review

- Why the unit test is the right level:
  - `<reason>`
- Why the integration test is the right level:
  - `<reason>`
- Remaining semantic gap:
  - `<gap or none>`
```

## Rules

* Keep obligations concrete and semantic.
* Do not claim xfstests coverage for cases that were not implemented there.
* Do not hide deferred coverage.
* Prefer absolute paths in the placement and patch sections.
