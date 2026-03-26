---
name: test-validator
description: use when chatgpt needs to validate consistency across a spec, a standalone `.test` artifact, existing unit or integration tests, and the current code, especially to prove coverage, find missing cases, and check whether code logic satisfies the `.test` requirements.
---

# Test Validator

Validate four relationships separately:

1. whether the `.test` artifact covers the spec,
2. whether existing unit and integration tests cover the `.test` cases,
3. whether current code logic satisfies the `.test` requirements,
4. which important cases are still missing or weak in the `.test`.

Keep these judgments separate. Existing unit tests do not excuse a missing `.test` case. A well-written `.test` file does not prove the code follows it. Code that looks plausible does not excuse missing traceability back to the spec.

## Workflow

### 1. Gather the four inputs

Collect and label:

* the spec,
* the `.test` artifact,
* the current code or diff,
* the existing unit and integration tests.

If one input is missing, continue conservatively and say what cannot be concluded.

### 2. Normalize the spec and the `.test`

Do not review against raw prose.

First reduce:

* the spec into obligations,
* the `.test` into concrete validation cases,
* the existing tests into implemented coverage evidence.

Use [validation-rubric.md](./references/validation-rubric.md) for the exact buckets and status rules.

### 3. Validate `spec` -> `.test` coverage

For each material spec obligation, classify the `.test` coverage as:

* covered explicitly,
* covered partially,
* missing,
* contradicted,
* unclear.

Check:

* preconditions,
* success postconditions,
* failure postconditions,
* preserved invariants,
* cross-API invariants,
* forbidden drifts,
* Linux semantic notes when the spec uses them.

### 4. Validate `.test` -> existing test implementation coverage

For each `.test` case, ask whether current tests already cover it.

Classify each case as:

* covered by unit test,
* covered by integration test,
* covered partially,
* not covered,
* unclear.

Use [existing-test-coverage.md](./references/existing-test-coverage.md) for mapping rules and acceptable evidence.

### 5. Validate `.test` -> code logic conformance

Treat the `.test` cases as validator obligations for the current code.

For each important case, classify code evidence as:

* logically satisfied,
* partially evidenced,
* contradicted,
* not statically provable,
* unclear.

Use [logic-check-guidance.md](./references/logic-check-guidance.md) for code-inspection priorities and evidence standards.

### 6. Detect missing or weak cases in the `.test`

Do not stop at traceability already written in the `.test`.
Actively look for missing coverage dimensions, weak oracles, mode splits that should be separate, and cases that are too broad to implement reliably.

Use [gap-detection.md](./references/gap-detection.md) for the required gap-finding pass.

### 7. Report in the strict output format

Always produce the final answer using the report structure in [report-template.md](./references/report-template.md).
Before sending the final answer, save the full markdown report under `.trellis/reports/test/`
with:

```bash
python3 .trellis/scripts/save_report.py --kind test-validate --subdir test --slug <target-slug>
```

Use a short slug derived from the feature or `.test` artifact being verified.
In the final answer, include the saved report path.

The report must include:

* `PASS`, `PARTIAL`, or `FAIL` for `spec` -> `.test` coverage,
* `PASS`, `PARTIAL`, or `FAIL` for existing unit/integration test coverage of the `.test`,
* `PASS`, `PARTIAL`, or `FAIL` for code logic conformance to the `.test`,
* explicit missing or weak `.test` cases,
* must-fix items and risk level,
* the saved report path.

## Decision Rules

Use these rules consistently:

* Treat missing evidence as `PARTIAL` or `unclear`, not as `PASS`.
* Treat a spec obligation with no corresponding `.test` case as a `spec` -> `.test` failure.
* Treat a `.test` case with no existing runtime test as a coverage gap, not as proof of code failure.
* Treat a `.test` case contradicted by code as a code-logic failure even if no runtime test exists yet.
* Treat weak or non-checkable `.test` cases as `.test` quality gaps.
* Treat pure naming or helper-structure differences as irrelevant unless they block traceability or semantics.

## Evidence Standard

Cite concrete evidence wherever possible:

* exact spec clauses,
* exact `.test` case IDs or headings,
* exact unit or integration test names,
* exact code functions, branches, guards, and return paths,
* exact traceability links,
* exact missing dimensions when reporting `.test` gaps.

Separate proven failures from plausible concerns.

## Resources

Read these references as needed:

* [validation-rubric.md](./references/validation-rubric.md)
* [existing-test-coverage.md](./references/existing-test-coverage.md)
* [logic-check-guidance.md](./references/logic-check-guidance.md)
* [gap-detection.md](./references/gap-detection.md)
* [report-template.md](./references/report-template.md)
* `/root/asterinas/.trellis/scripts/save_report.py`
