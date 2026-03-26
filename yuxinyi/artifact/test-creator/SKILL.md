---
name: test-creator
description: use when chatgpt needs to turn an existing spec into standalone `.test` files, validation matrices, corner-case inventories, or traceability maps that drive validation and later test generation. when repo or harness context is known, include test-level recommendations, suggested target files, and handoff notes so another agent can turn the artifact into actual tests with minimal semantic ambiguity.
---

# Test Creator

Turn an existing spec into test obligations, not into another spec.

This skill is for deriving validator-ready and test-generation-ready artifacts from:

* a contract spec,
* an implementation spec,
* or both together.

The primary output is a **new standalone `.test` file**. Do not append test content to the `.spec`.

## Use This Skill For

* generating a `.test` artifact from an existing `.spec`
* building a validation matrix for a feature or API
* extracting MUST / SHOULD / OPTIONAL test obligations
* mapping spec clauses to concrete runtime or review cases
* preparing guidance for a later test-generation agent
* producing corner-case inventories for kernel or filesystem APIs
* preparing repo-aware handoff artifacts for a later test-writing agent

Typical prompts:

* derive a `.test` file from this `write_at.spec`
* turn these contract clauses into a validation matrix
* generate test obligations for `rename` no-replacement
* produce a traceability map for `sync_data` and `sync_all`
* create test-generation-ready cases for direct I/O only

## Do Not Use This Skill For

* writing or rewriting the spec itself
* merging test obligations back into the contract spec
* generating an implementation patch plan
* prescribing one exact test framework unless the user asks
* replacing a deeper semantic or concurrency review

Use `$contract-spec` for stable semantic contracts and `$spec-creator` for implementation specs.
Use `$ext2-test-writer` when the task is to turn the resulting artifact into actual Asterinas Ext2 `#[ktest]` or initramfs integration test code.

## Core Workflow

1. Identify the source material.
   Separate contract-spec obligations from implementation-spec intentions. If both exist, keep them distinct.
2. Lock the review scope.
   Note the API family, scenario, Linux semantic references, repo-specific constraints, and any focus restriction such as direct I/O only.
3. Extract testable obligations first.
   Convert spec clauses into things a validator or runtime test can actually check: prestates, actions, observable results, in-memory effects, persistence effects, failure oracles, and forbidden drifts.
4. Build coverage dimensions before enumerating cases.
   Use [coverage-dimensions.md](./references/coverage-dimensions.md) to decide which axes matter.
5. Split obligations into validation modes.
   Explicitly distinguish:
   * validator-oriented checks
   * static review obligations
   * runtime test obligations
   * dynamic test obligations
   * fault-injection cases
   * concurrency review cases
6. Produce a validation matrix.
   Mark every case as `MUST`, `SHOULD`, or `OPTIONAL`, and state whether it belongs to static review, unit test, integration test, fault injection, or concurrency review.
   When the repo context is known, also record the recommended harness and likely target location.
7. Write the standalone `.test` artifact.
   Use [test-file-template.md](./references/test-file-template.md). Keep it separate from the `.spec`.
8. Add traceability.
   Every important case should point back to contract clauses, invariants, forbidden drifts, scenario contracts, and Linux semantic notes when relevant.
9. Add writer handoff notes when appropriate.
   If the artifact is meant to feed later code generation, say which cases are best implemented as unit tests, which as integration tests, and which remain deferred to heavier harnesses such as xfstests.

## Output Rules

When generating testing artifacts:

* prefer a new `.test` file over inline prose
* keep `.test` separate from `.spec`
* use readable markdown headings and labels
* do **not** use bracketed formal markers like `[PRESTATE]` or `[COVERS]`
* keep cases explicit, checkable, and narrow enough to implement
* do not overfit to one repo layout or test harness unless the task provides it
* when the task does provide repo context, include enough harness and target-path guidance that another agent can implement the cases directly

Allowed outputs, depending on the prompt:

* standalone `.test` file
* validation matrix
* test obligations document
* corner-case checklist
* test case inventory
* traceability map
* test-generation-ready template

## Required Case Content

Each strong obligation entry should answer:

* what it covers
* why it matters
* what prestate is required
* what action is taken
* what result is expected
* what in-memory state should change
* what on-disk or persistence state should change
* what must not happen
* which spec clauses justify the case
* which validation mode and implementation target fit best when the repo context is known

When the API has distinct modes, split cases instead of hiding the difference:

* buffered vs direct I/O
* `sync_data` vs `sync_all`
* same-dir vs cross-dir rename
* replacement vs no-replacement

## Filesystem And Kernel Bias

Pay special attention to:

* zero-length operations
* boundaries and overflow-adjacent inputs
* EOF behavior
* partial completion
* metadata update rules
* cache coherence
* durability semantics
* failure cleanup
* cross-API consistency
* concurrency-sensitive corner cases
* forbidden drifts under future refactors

For APIs like `read_at`, `write_at`, `resize`, `fallocate`, `sync_data`, `sync_all`, `rename`, `unlink`, and `link`, prefer concrete cases over broad “test basic behavior” bullets.

## Resources

Read only what you need:

* [test-principles.md](./references/test-principles.md)
* [test-file-template.md](./references/test-file-template.md)
* [validation-matrix-template.md](./references/validation-matrix-template.md)
* [traceability-template.md](./references/traceability-template.md)
* [coverage-dimensions.md](./references/coverage-dimensions.md)
* [spec-to-test-mapping.md](./references/spec-to-test-mapping.md)
