# `.test` File Template

Use this template when the main requested deliverable is a standalone `.test` file.

```md
# <feature or api> test plan

## Title

<short title>

## Scope

- Feature or API:
- Focus restrictions:
- Out of scope:

## Source Specs

- Contract spec:
- Implementation spec:
- Linux semantic references:
- Local code or API references:

## Coverage Dimensions

- <dimension 1>
- <dimension 2>
- <dimension 3>

## Validation Matrix

| Case ID | Scenario | Level | Validation Mode | Primary Oracle | Suggested Implementation Target | Traceability |
|---------|----------|-------|-----------------|----------------|---------------------------------|--------------|
| T1 | ... | MUST | integration test | ... | initramfs C test in `test/initramfs/...` | ... |

## Case Inventory

## Case: <descriptive scenario title>

Covers:
- <coverage axes>

Level:
- MUST

Validation mode:
- integration test
- static review

Why it matters:
- <why this case exists>

Prestate:
- <initial state>

Action:
- <operation to perform>

Expected result:
- <return value / errno / visible result>

Expected in-memory effects:
- <state that must change or remain unchanged>

Expected on-disk or persistence effects:
- <durability / reachability / flush expectations>

Failure oracle:
- <what would prove drift or regression>

Traceability:
- <contract clauses / invariants / forbidden drifts / Linux notes>

Suggested implementation target:
- <unit test / integration test / static review / deferred>
- <target module or path when known>

## Traceability Summary

- <cluster important clauses under the cases that cover them>

## Test Generation Notes

- <what a later agent should implement as unit, integration, or fault-injection tests>
- <what runtime infrastructure is required>
- <what can be validated statically instead>
- <which cases remain deferred to heavier frameworks such as xfstests when applicable>
```

## Notes

* Keep the file standalone.
* Do not paste the whole spec into the `.test`.
* Prefer concrete cases over long narrative prose.
* Split cases by mode when semantics differ.
* If the repository or harness is known, include a suggested implementation target for each important case.

## Compact Example Titles

* `write_at` extends EOF
* direct write invalidates stale overlapping cache
* `rename` no-replacement same directory
* `sync_data` preserves data visibility without claiming full metadata durability
