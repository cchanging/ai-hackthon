---
name: ext2-test-writer
description: write concrete Asterinas Ext2 tests from a spec, `.test` artifact, semantic bug report, or Linux-visible behavior requirement. use when codex needs to turn semantic obligations into actual `#[ktest]` unit tests and initramfs integration tests under `/root/asterinas/test/initramfs/src/apps/fs/ext2/`, and validate that the written tests cover the spec semantics. do not use this skill to manually author xfstests cases; xfstests remains the end-to-end framework and only serves as semantic reference here.
---

# Ext2 Test Writer

Write actual tests, not only a test plan.

Default output is a code patch that adds or updates Asterinas tests. If the user only wants a standalone `.test` artifact or validation matrix, use `$test-creator` instead.

## Inputs

Start from one or more of:

* a `.spec`
* a `.test` artifact produced by `$test-creator`
* a failure analysis or xfstests diagnosis
* Linux semantic notes
* an existing implementation plus explicit semantic requirements from the user

If multiple sources disagree, prefer the spec contract first, then Linux-visible semantics, then current implementation.

## Workflow

### 1. Lock the semantic target

Extract the concrete obligations that must be checked:

* success results and returned values
* errno and failure conditions
* namespace visibility
* metadata updates
* persistence and remount behavior
* forbidden drifts after refactor

Do not start from “where tests usually go.” Start from what semantics must be proven.

### 2. Choose the test level

Read [test-level-selection.md](./references/test-level-selection.md).

Split obligations by level instead of forcing everything into one harness:

* use `#[ktest]` for local kernel semantics, helper logic, internal state transitions, narrow invariants, and error propagation that do not require a userspace syscall flow
* use initramfs integration tests for syscall-visible behavior, pathname operations, cross-fd behavior, mount/remount behavior, process-visible results, or persistence checks
* use both when one feature has an internal invariant that should be pinned by unit tests and an external semantic contract that should be pinned by integration tests

Do not manually author xfstests tests in this skill. Xfstests is the later end-to-end oracle, not the immediate output.

### 3. Place the tests in the right location

For `#[ktest]` unit tests:

* place tests near the closest module that owns the behavior
* reuse an existing `test.rs` or local `#[cfg(ktest)]` section when present
* keep unit tests narrowly scoped and deterministic

For integration tests:

* place new C tests under `/root/asterinas/test/initramfs/src/apps/fs/ext2/`
* use the existing `../../common/test.h` harness
* prefer one semantic topic per file
* the local `Makefile` already includes `../../common/Makefile`, which builds all `*.c` files in that directory, so a new C file usually needs no extra Makefile change

### 4. Implement the tests

Read only the template you need:

* [unit-test-template.md](./references/unit-test-template.md)
* [integration-test-template.md](./references/integration-test-template.md)
* [delivery-template.md](./references/delivery-template.md)

Write the narrowest credible tests first. One test should prove one semantic point.

Prefer:

* explicit setup and cleanup
* concrete errno assertions
* visible postconditions over incidental internal details
* comments only when the semantic reason is not obvious from the code

Avoid:

* duplicating the spec as comments
* mixing unrelated semantic obligations into one large test
* asserting implementation accidents that the spec does not require

### 5. Use the standard delivery format

When the task is “write the tests” rather than “only plan the tests,” structure the result using
[delivery-template.md](./references/delivery-template.md).

The final deliverable should make it obvious:

* which semantic obligations were implemented as `#[ktest]`
* which semantic obligations were implemented as initramfs integration tests
* which obligations were intentionally deferred to xfstests or another harness
* which files were added or changed

### 6. Validate semantic traceability

Read [semantic-traceability-checklist.md](./references/semantic-traceability-checklist.md) before finishing.

Every written test should be traceable to a semantic obligation. In the final answer, briefly state:

* which obligations were covered by unit tests
* which obligations were covered by integration tests
* which obligations remain for xfstests or another harness

## Output Rules

When this skill is used to write tests:

* default to editing or creating actual test files
* keep `#[ktest]` tests focused on one invariant or one local semantic branch
* keep integration tests focused on one syscall-visible behavior family
* use Linux-visible semantics as the oracle when the spec says to match Linux
* preserve existing repo style and helpers
* say explicitly when some semantic obligation cannot be covered well at unit or integration level

## Resources

Read only what is needed:

* [test-level-selection.md](./references/test-level-selection.md)
* [unit-test-template.md](./references/unit-test-template.md)
* [integration-test-template.md](./references/integration-test-template.md)
* [delivery-template.md](./references/delivery-template.md)
* [semantic-traceability-checklist.md](./references/semantic-traceability-checklist.md)
