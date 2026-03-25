---
name: spec-creator
description: Create or rewrite implementation specs into coding-agent-friendly task contracts for Asterinas and similar Rust systems code. Use when drafting a new `.spec`, refactoring a human-oriented design note, or converting `writing-spec` style material into explicit task boundaries, hard constraints, required APIs, required call graphs, phased algorithms, forbidden actions, and machine-checkable acceptance checklists.
---

# Spec Creator

Write specs for coding agents as executable task contracts, not as design essays.

Use this skill to turn requirements, Linux references, and local code structure into a spec that makes four things obvious:

1. What must change.
2. What must not change.
3. What code shape must exist when done.
4. How completion will be judged.

## Workflow

### 1. Read before writing

Before drafting the spec:

* Identify whether the task is a new implementation, a refactor, or a bug fix on existing code.
* Find the current runtime paths, helper functions, and call sites that matter.
* Find the Linux semantic references if Linux behavior is relevant.
* Separate user-visible behavior requirements from locking / architecture constraints.

If the task is a refactor, state that explicitly. Default assumption: it is **not** greenfield.

### 2. Separate hard constraints from background

Do not mix explanatory context with hard requirements.

Use:

* hard sections for things the agent must obey,
* background sections for semantic references, rationale, and local model notes.

Good background sections:

* `SEMANTIC REFERENCES`
* `BACKGROUND`
* `RATIONALE`
* `LOCAL MODEL`
* `LOCKING FACTS`

Good hard sections:

* `TASK`
* `REQUIRED APIS`
* `REQUIRED CALL GRAPH`
* `ALGORITHM`
* `GLOBAL INVARIANTS`
* `METHOD INVARIANTS`
* `FORBIDDEN`
* `EDIT SCOPE`
* `ACCEPTANCE CHECKLIST`

Background may explain. Hard sections must constrain.

### 3. Prefer machine-checkable wording

Coding agents follow explicit constraints better than prose.

Prefer statements like:

* `MUST add function X`
* `MUST use helper Y on path Z`
* `MUST NOT call page cache while holding self.inner.write()`
* `MUST preserve existing errno behavior`
* `MUST return a live write guard`

Avoid vague wording like:

* "clean this up"
* "follow the spirit of Linux"
* "share as much logic as possible"

When you need "same algorithm" semantics, define exactly what may differ and what may not differ.

### 4. Make code shape explicit

Distinguish these clearly:

* `REQUIRED APIS`: functions, methods, signatures, return types, preserved interfaces
* `REQUIRED CALL GRAPH`: which paths must call which helpers
* `GLOBAL INVARIANTS`: system-wide truths that must remain true across all touched paths
* `METHOD INVARIANTS`: what each required API must preserve before and after the call

If call-site convergence matters, say so directly:

* There must be one canonical runtime insertion algorithm.
* Path A and path B must both call helper Z.
* The old mutable-inner path must not remain on the runtime path.

### 5. Write algorithms as phases

For lock-sensitive or multi-step logic, write the contract as ordered phases.

Example pattern:

* `PHASE 1`: validate arguments
* `PHASE 2`: acquire guard / locate state
* `PHASE 3`: perform growth / upgrade / downgrade transitions
* `PHASE 4`: write data
* `PHASE 5`: commit metadata
* `PHASE 6`: return

This is better than a dense paragraph when guard transitions or ordering are critical.

### 6. Constrain refactor scope

Large specs often trigger unnecessary rewrites. Prevent that explicitly.

Add an `EDIT SCOPE` section that says things like:

* Make the smallest safe refactor that satisfies this spec.
* Preserve unrelated helpers and public behavior.
* Do not rename unrelated symbols.
* Do not rewrite parsing or layout logic unless required by the shared runtime path.

### 7. End with acceptance assertions

Turn tests into short completion checks, not long test-plan prose.

Prefer:

* `duplicate name returns EEXIST`
* `same-dir rename no-replacement uses shared helper path`
* `cache-miss scan/write path completes without deadlock`

Avoid long narrative test descriptions unless the setup is non-obvious.

## Required Output Shape

Use this shape unless the task is so small that one or two sections are unnecessary:

1. `TASK`
2. `SEMANTIC REFERENCES`
3. `LOCAL MODEL`
4. `LOCKING FACTS` if concurrency or callbacks matter
5. `REQUIRED APIS`
6. `REQUIRED CALL GRAPH`
7. One or more `ALGORITHM` sections
8. `GLOBAL INVARIANTS`
9. `METHOD INVARIANTS`
10. `FORBIDDEN`
11. `EDIT SCOPE`
12. `ACCEPTANCE CHECKLIST`

Optional sections:

* `BACKGROUND`
* `RATIONALE`
* `TOUCH POINTS`
* `OPEN QUESTIONS`
* `DIFF` when behavior intentionally diverges from Linux

Read [template.md](./references/template.md) and follow it closely.

## Linux And Local Architecture

When Linux code is the semantic reference:

* Match Linux user-visible semantics, mutation intent, and errno behavior where applicable.
* Do **not** mechanically copy Linux locking structure.
* State explicitly when Linux is the semantic reference but not the locking blueprint.

Recommended wording:

* Match Linux user-visible semantics and mutation ordering where applicable.
* Linux source is the semantic reference for filesystem behavior, not a 1:1 locking blueprint.
* Adapt locking to Asterinas callback and safety constraints.

If local architecture imposes a stronger rule than Linux, write that rule as a hard constraint.

## Refactor-Specific Rules

When the task modifies existing code:

* State: `This is a refactor of existing code, not a fresh implementation.`
* Tell the agent to identify current runtime paths before editing.
* List likely touch points when multiple call sites must be updated together.
* Say which old path must stop being used if the task eliminates duplication.

Recommended wording:

* Before editing, identify the current runtime path and all call sites that must converge on the canonical helper.
* Runtime path `A` must stop using helper `B`.
* Preserve behavior outside the touched runtime path.

## Invariant Rules

Every non-trivial spec should define invariants at two levels:

* one global invariant block for the whole change,
* one per-method invariant block for each required API or algorithm entry point.

Use `GLOBAL INVARIANTS` for truths that must hold across the entire feature or refactor, such as:

* lock ordering remains ascending by inode number,
* there is one canonical runtime insertion path,
* user-visible errno behavior remains unchanged,
* no callback-driven I/O occurs while holding a write lock.

Use `METHOD INVARIANTS` to state what each method preserves across entry and exit. These should answer:

* what must already be true when the method is called,
* what state relationships remain true after success,
* what state relationships remain true after failure,
* what the method must not invalidate even if it mutates internal state.

Good examples:

* `add_entry`: directory layout remains ext2-valid before and after the call.
* `add_entry`: failure must not persist a partially committed invalid dirent.
* `rename_inner`: inode lock ordering and replacement semantics remain unchanged.
* `add_entry_from_write_guard`: returned guard still protects the same inode inner state and caller workflow can continue.

## Asterinas Bias

For Asterinas kernel work:

* Keep `kernel/` safe Rust only.
* Prefer `Result<T>` with explicit errno behavior.
* Preserve lock ordering across inodes and subsystems.
* Call out callback-sensitive operations such as page cache I/O.
* State forbidden lock states explicitly when needed.

If the task is in `kernel/`, forbid `unsafe` unless the task is explicitly in `ostd/`.

## Common Failure Modes To Prevent

Add constraints to stop these mistakes:

* Over-copying Linux locking behavior.
* Reimplementing nearly identical algorithms instead of converging on one canonical path.
* Updating the core helper but missing the runtime call sites.
* Leaving old mutation code reachable on the runtime path.
* Holding a write lock across callback-driven I/O.
* Expanding the refactor into unrelated renames or structural cleanup.

## Writing Rules

* Use short imperative statements.
* Prefer bullets and short paragraphs over dense prose.
* Use `MUST`, `MUST NOT`, `ONLY`, `EXACTLY`, and `UNCHANGED` when the condition is strict.
* Name concrete functions, methods, structs, guards, and paths whenever possible.
* If a section is optional and irrelevant, omit it instead of filling it with filler text.
* If you mention a semantic difference from Linux, explain whether it affects behavior, locking, or implementation structure.

## Resources

### Template

Use [template.md](./references/template.md) as the default skeleton for a new spec or a rewrite of an older `writing-spec` style document.
