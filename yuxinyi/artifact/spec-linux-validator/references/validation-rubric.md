# Validation Rubric

Use this rubric to normalize the spec into machine-checkable buckets before reviewing code.

## Core Principle

Do not validate against the spec as one undifferentiated block of prose.
Extract explicit obligations first, then compare code against each bucket.

## Bucket Model

For every applicable item in the spec, create a checklist entry under one of these buckets.

### 1. Required APIs

Check:

* required functions and methods exist,
* required signatures and return types exist,
* required preserved APIs were not removed or weakened,
* required guard-returning APIs still satisfy their workflow contract.

Classify as:

* `implemented exactly`
* `implemented partially`
* `missing`
* `contradicted`
* `unclear`

### 2. Required Call Graph

Check:

* required callers actually call the mandated helper,
* forbidden old paths are no longer used on the runtime path,
* canonical shared paths were actually converged,
* refactor touch points were all updated.

Look for:

* branch-specific call sites,
* replacement versus no-replacement branches,
* call-site divergence that leaves old code reachable.

### 3. Preconditions

Check:

* argument validation,
* type/state guards,
* filesystem range checks,
* required lock/guard state at entry,
* assumptions the method must enforce before proceeding.

Failure mode:

* method proceeds without enforcing a stated precondition,
* method weakens the precondition in a way that violates the spec.

### 4. Postconditions

Check:

* success-state guarantees,
* failure-state guarantees,
* allowed error set,
* returned guard state,
* cleanup and non-persistence guarantees on failure.

For each postcondition, state whether it is:

* directly proven by the code,
* only partially evidenced,
* not statically provable from the available code.

### 5. Algorithm Phases

Check ordered phases, not just ingredients.

Examples:

* validate before mutation,
* scan before growth,
* downgrade before callback-driven I/O,
* commit metadata after data mutation,
* reacquire required guard state before return.

If phases exist but the ordering is wrong, treat that as a violation.

### 6. Global Invariants

Check invariants that must hold across the whole change:

* lock ordering,
* canonical runtime path,
* unchanged user-visible semantics,
* forbidden lock states,
* filesystem consistency rules.

### 7. Method Invariants

For each required API, check what remains true:

* before the call,
* after success,
* after failure.

Pay attention to:

* no invalid on-disk state,
* no partially committed visible state,
* preserved caller workflow assumptions,
* preserved ownership or guard relationships.

### 8. Forbidden Actions

Treat these as hard constraints.

Examples:

* must not call page cache while holding a write lock,
* must not introduce a second mutation algorithm,
* must not use `unsafe`,
* must not change replacement semantics.

Any proven violation here is at least a spec-conformance failure.

### 9. Edit Scope Constraints

Check whether the implementation stayed within the permitted refactor scope.

Look for:

* unrelated renames,
* broad rewrites outside the touched runtime path,
* removal of preserved helper structure without necessity,
* accidental changes outside the required area.

Scope violations are usually cleanup-level unless they also alter semantics or safety.

### 10. Acceptance Checklist

Treat each acceptance item as one of:

* statically satisfied,
* partially evidenced,
* not statically provable,
* contradicted.

Do not mark an item as satisfied if it really needs runtime evidence not present in the code.

## Status Rules

### PASS

Use only when all material requirements are either:

* directly satisfied, or
* explicitly marked as not statically provable but with no contrary evidence and only for runtime-only checks.

### PARTIAL

Use when:

* some requirements are unclear,
* some acceptance items are not statically provable,
* evidence exists for only part of the intended behavior,
* the implementation appears plausible but not fully demonstrated.

### FAIL

Use when:

* a required API is missing,
* a required call path is absent,
* an invariant is broken,
* a forbidden action is present,
* required phase ordering is violated,
* the code clearly contradicts the spec.

## Evidence Rules

Always attach evidence to claims:

* spec clause or section,
* implementation file and function,
* exact branch or condition when relevant,
* call path or state transition when relevant.

If evidence is indirect, say so.

## Refactor-Specific Review Traps

When validating refactors, explicitly look for these traps:

* old and new paths both remain reachable,
* helper was extracted but one caller still bypasses it,
* branch-specific behavior changed accidentally,
* lock transitions moved but callback safety was not preserved,
* mutation ordering changed while final output looks superficially correct.
