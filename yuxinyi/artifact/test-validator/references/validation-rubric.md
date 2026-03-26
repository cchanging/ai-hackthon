# Validation Rubric

Normalize the review into four separate coverage layers.

## Core Principle

Do not collapse these into one verdict:

* `spec` -> `.test`
* `.test` -> existing runtime tests
* `.test` -> current code logic
* missing or weak cases inside `.test`

One layer passing does not imply the others pass.

## Bucket Model

### 1. Spec Obligations

Normalize the spec into:

* preconditions
* success postconditions
* failure postconditions
* preserved invariants
* cross-API invariants
* forbidden drifts
* Linux semantic notes, if the spec relies on them

### 2. `.test` Cases

Normalize the `.test` file into:

* case IDs or case headings
* coverage dimensions
* level (`MUST`, `SHOULD`, `OPTIONAL`)
* validation mode
* oracle
* traceability targets

### 3. Existing Runtime Tests

Normalize existing tests into:

* unit tests
* integration tests
* fault-injection tests
* concurrency tests

Record whether each test fully covers, partially covers, or misses a `.test` case.

### 4. Code Logic Obligations

For each important `.test` case, inspect:

* entry checks
* branching conditions
* success path
* failure path
* cleanup path
* metadata updates
* cross-API relationships

## Status Rules

### PASS

Use only when all material requirements in that layer are either directly evidenced or only blocked by runtime-only proof gaps.

### PARTIAL

Use when some requirements are covered but key items are unclear, only partially covered, or not fully provable.

### FAIL

Use when a material obligation is missing, contradicted, or left uncovered in that layer.

## Evidence Rules

Always attach evidence:

* spec clause
* `.test` case
* test function name
* code path
* exact missing gap

## Four Required Questions

Every run must answer:

1. Does the `.test` cover the spec?
2. Which `.test` cases are covered by existing unit and integration tests?
3. Does the code logically satisfy the `.test` requirements?
4. What important cases are missing or weak in the `.test`?
