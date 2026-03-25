---
name: contract-spec
description: use when chatgpt needs to create a strong validation-oriented contract spec for an api, module, or subsystem, especially to freeze stable semantic constraints, state postconditions, failure guarantees, and forbidden drift across future refactors.
---

# Contract Spec

Write strong contract specs, not implementation plans.

This skill produces validator-ready contract specs. The output must be strong enough
that a later reviewer or validator can point to a violated clause, not just say the
implementation "feels different".

## Use This Skill For

* API-level contract baselines
* module-level semantic baselines
* scenario contracts such as rename, unlink, sync, or read/write flows
* validator baselines for future refactors

Typical prompts:

* write a strong contract spec for `read_at` and `write_at`
* freeze the API-level and state-level contract for `sync_data` and `sync_all`
* define a validator-ready contract for inode data I/O
* capture pre/post/in-memory/on-disk invariants for this filesystem API family

## Do Not Use This Skill For

* patch-specific helper extraction plans
* one refactor's edit sequence
* lock choreography plans for one change
* required call-graph rewrites
* temporary migration instructions

Use an implementation-spec skill such as `$spec-creator` for those tasks.

## Core Workflow

1. Pick a stable contract boundary.
   API-level, module-level, or scenario-level only. If the boundary feels patch-specific, it is wrong.
2. Build a local state model first.
   Name exact structs and fields when known, such as `InodeInner.desc`, `InodeInner.page_cache`,
   dirty flags, block mapping state, inode mirrors, and on-disk objects.
3. Separate five things explicitly:
   logical postconditions, in-memory state postconditions, on-disk state postconditions,
   durability guarantees, and allowed internal flexibility.
4. Write exact state obligations.
   Prefer clauses a validator can check over high-level intent language.
5. Add cross-API invariants when single-API contracts are too weak.
6. Use Linux as a semantic baseline only for stable visible behavior.
   Do not freeze Linux helper shape, control flow, or locking unless that is itself a permanent contract rule.

## Required Output Shape

Use the strong template in [contract-spec-template.md](./references/contract-spec-template.md).

At minimum, include:

* `TITLE`
* `SCOPE`
* `STATE MODEL`
* `GLOBAL INVARIANTS`
* `API CONTRACTS`
* optional `CROSS-API INVARIANTS`
* optional scenario contracts
* `FORBIDDEN DRIFTS`

For each important API, prefer this section set:

* `PURPOSE`
* `DOMAIN` or `APPLICABILITY`
* `PRECONDITIONS`
* `OBSERVABLE EFFECTS`
* `IN-MEMORY STATE POSTCONDITIONS: SUCCESS`
* `ON-DISK STATE POSTCONDITIONS: SUCCESS`
* `LOGICAL POSTCONDITIONS: SUCCESS`
* `FAILURE POSTCONDITIONS`
* `PRESERVED INVARIANTS`
* `ALLOWED INTERNAL FLEXIBILITY`
* optional `LINUX SEMANTIC NOTES`

Tiny getter/setter families may be grouped when splitting them would reduce clarity.

## Strength Rules

Strong contracts prefer:

* exact state obligations over vague intent
* committed-prefix semantics over generic success language
* explicit failure postconditions over "must not be invalid"
* exact structure names when local model is known
* explicit visibility-versus-durability distinctions
* effect-boundary language over vague "may do X" summaries

Weak wording to avoid unless immediately made precise:

* must remain valid
* must remain semantically compatible
* should preserve behavior

Good wording examples:

* If the API returns `Ok(n)`, then `0 <= n <= len`.
* Logical bytes in range `[offset, offset+n)` equal the caller buffer prefix.
* `InodeInner.desc` must reflect successful size growth.
* `InodeInner.page_cache` must not retain stale overlapping data after successful direct write.
* Failure must not leave `desc.size` inconsistent with logical EOF.
* The only logical bytes this API may newly expose are bytes in `[offset, offset+n)`.
* The API must not create visible effects outside the requested write domain.

## Filesystem / Kernel Bias

When local model is known, contracts should name concrete state directly:

* inode mirrors such as `desc`
* page cache state
* dirty state
* cache invalidation or cache coherence requirements
* block-mapping validity
* file size update rules
* timestamp update rules such as `ctime`, `mtime`, `atime`
* direct I/O versus buffered I/O semantic differences
* sync and durability domains
* read/write committed range semantics
* partial-completion rules

## Observable Effects Rule

`[OBSERVABLE EFFECTS]` is a constraint section, not a loose summary section.

Write it as an effect boundary:

* what categories of externally visible effects are allowed
* what the maximum allowed effect domain is
* what kinds of visible effects are explicitly forbidden

Bad pattern:

* "May modify metadata"
* "May update cache"
* "May allocate blocks"

Better pattern:

* "The only logical bytes this API may newly expose are bytes in `[offset, offset+n)`."
* "The API may change allocation state only as needed to make the committed range reachable."
* "The API must not create visible effects outside the write domain, including namespace changes or stronger durability claims."

If `[OBSERVABLE EFFECTS]` does not help a validator rule out forbidden effects, it is too weak.

## Resources

Read only what you need:

* [contract-spec-principles.md](./references/contract-spec-principles.md)
* [contract-spec-template.md](./references/contract-spec-template.md)
* [contract-vs-implementation-spec.md](./references/contract-vs-implementation-spec.md)
* [linux-semantic-notes.md](./references/linux-semantic-notes.md)
* [example.md](./references/example.md)
