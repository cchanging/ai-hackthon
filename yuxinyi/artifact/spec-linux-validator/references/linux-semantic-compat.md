# Linux Semantic Compatibility

Use Linux as the behavioral reference, not as a line-by-line implementation template.

## Allowed Differences

These differences are usually permissible if semantics still match:

* different helper decomposition,
* different internal factoring,
* different lock-transition strategy,
* different callback-safe adaptation for the local runtime model,
* different data-structure choices that preserve visible behavior,
* different local naming and control-flow shape.

Do not flag these as failures unless they change semantics or violate the spec.

## What To Compare

Compare Linux and the implementation by scenario.

### 1. Observable Behavior

Check:

* what succeeds,
* what fails,
* what is returned,
* what side effects become visible.

### 2. Error Mapping

Check:

* which conditions return which errno,
* whether invalid input, duplicate state, missing entries, capacity exhaustion, and I/O failure map correctly,
* whether local error adaptation changes visible behavior.

### 3. Mutation Intent And Ordering

Check the correctness-relevant order of effects:

* state discovery,
* reservation or growth,
* mutation,
* metadata update,
* persistence or commit points,
* cleanup on failure.

Linux-like structure is not required. Correctness-relevant ordering is.

### 4. Scenario Splits

Check scenario-sensitive branches such as:

* replacement versus no-replacement,
* same-object versus cross-object cases,
* cache hit versus cache miss,
* fast path versus slow path,
* success path versus recovery path.

### 5. State Transitions And Accounting

Check:

* link counts,
* timestamps,
* dirty or committed state,
* block and inode accounting,
* cache membership or deletion lifecycle,
* ownership or reference transitions.

### 6. Safety Properties

Check:

* consistency invariants,
* deadlock-prone paths,
* forbidden reentrancy,
* lock ordering,
* callback safety,
* no partial invalid state on failure.

## What To Flag

Flag semantic divergence when the implementation changes:

* visible success or failure conditions,
* errno behavior,
* replacement or no-replacement behavior,
* mutation intent,
* required state transitions,
* correctness-relevant ordering,
* consistency rules,
* safety properties promised by Linux semantics or the local runtime model.

## What Not To Flag

Do not flag these by themselves:

* helper extracted into a new internal method,
* branch flattened or reordered without semantic change,
* lock downgrade or upgrade pattern adapted for callback safety,
* code not looking like Linux despite equivalent behavior.

## Incomplete Linux References

If Linux references are partial:

* conclude only what the provided references support,
* note missing scenarios,
* say when full semantic comparison is impossible,
* avoid guessing based on nearby Linux code unless the inference is very strong and clearly labeled as inference.

## Comparison Method

Use this sequence:

1. Identify the Linux scenarios the spec and references care about.
2. Map each scenario to the local implementation path.
3. Compare behavior, errors, ordering, and invariants.
4. Separate semantic matches from structural differences.
5. Record uncertainty when the evidence is incomplete.

## Filesystem And Kernel Focus Points

For filesystem or kernel refactors, pay extra attention to:

* lock ordering across multiple objects,
* callback-driven I/O under locks,
* replacement versus no-replacement branches,
* metadata commit behavior,
* cleanup on partial failure,
* deadlock and reentrancy hazards,
* multi-object ordering invariants,
* local runtime constraints that justify structural divergence from Linux.
