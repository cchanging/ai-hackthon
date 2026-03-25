# Lockdep Summary

This file summarizes the current state of the static lockdep prototype in this repository.

Status note:

- The current prototype already supports local callable aliases, callback wrappers, nested
  callback wrappers, and returned-guard propagation through local wrappers.
- The recent review fixes are already included:
  - release-build `lockdep-driver` resolution
  - preservation of real branch-dependent reverse lock-order edges
  - symmetric IRQ-state merge at CFG joins

## Current State

The current implementation lives under [tools/lockdep](/root/asterinas-codex/tools/lockdep).

The prototype can already:

- build as an independent tool via `cargo run --manifest-path tools/lockdep/Cargo.toml -- ...`
- analyze real `ostd` and `aster-kernel` crates on `x86_64-unknown-none`
- export per-crate artifacts and aggregate them into a global report
- emit:
  - human-readable terminal output
  - JSON output
  - DOT graph output

Supporting documents:

- plan: [plan.md](/root/asterinas-codex/tools/lockdep/plan.md)
- progress tracker: [progress.md](/root/asterinas-codex/tools/lockdep/progress.md)
- `lockdep.toml` format: [lockdep-toml.md](/root/asterinas-codex/tools/lockdep/lockdep-toml.md)
- false-positive review: [lockdep_cycle_review.md](/root/asterinas-codex/tools/lockdep/lockdep_cycle_review.md)

## Supported Deadlock Detection

The prototype currently supports these categories.

### 1. Ordinary Lock-Order Cycles

It can detect ordinary lock-order cycles of the form:

- `A -> B -> A`

This is based on:

- intra-function lock event extraction
- direct-call summary propagation
- global lock graph construction
- SCC/cycle extraction

### 2. IRQ-Related Risk Detection

It can detect a minimal form of IRQ-related lock risk by tracking whether a lock appears in:

- hardirq/softirq usage bits
- hardirq-enabled / hardirq-disabled usage bits
- softirq-enabled / softirq-disabled usage bits

This is currently reported as:

- `single_lock_irq_violation`
- `irq_dependency_violation`
- compatibility `irq_conflict`

### 3. AA / Self-Loop Deadlock Detection

It now distinguishes AA-style deadlock risks from ordinary lock-order cycles.

Currently reported AA categories:

- `self_lock`
  - repeated acquisition of the same lock class on the same path
- `irq_reentry`
  - the same lock class appears in an interrupt context and an interruptible context

This means the prototype no longer only looks for `A -> B -> A`; it can also report `A -> A`.

## IRQ Context Support

The prototype currently understands these entry contexts:

- `IrqLine::on_active` -> `HardIrqTopHalf`
- `register_bottom_half_handler_l1` -> `BottomHalfL1`
- `register_bottom_half_handler_l2` -> `BottomHalfL2`

It also handles:

- `disable_irq().lock()`
- L1 bottom-half `DisabledLocalIrqGuard`
- direct crate-local propagation of IRQ entry contexts through normal calls
- local callable aliases
- callback wrappers and nested callback wrappers
- returned guards through local helper/callback wrappers

## Precision Improvements Already Landed

Several important false-positive reductions are already implemented.

### Stable Lock-Class Identity

The analyzer no longer directly uses raw MIR locals like `(*_4)` as global lock identity.

It now uses a more stable key derived from:

- enclosing function
- argument/local root
- projection path

This removed several bogus cross-function merges.

### Lock-Mode Filtering

The global graph now filters out clearly non-deadlocking shared-read pairs such as:

- `RwLock(read) -> RwLock(read)`
- `RwMutex(read) -> RwMutex(read)`

### Review Fixes

The recent review-driven fixes include:

- keeping both lock-order directions for real branch-dependent cycles
- symmetric IRQ enablement merge at CFG joins
- building `lockdep-driver` in the same profile as the frontend binary

### Blocking-Cycle Filtering

Even when a graph cycle exists, the tool now tries to reject cycles that do not correspond to a real blocking wait cycle.

This removed the earlier exfat false positive from the top-level report.

## Tests Added

There is now a fixture crate based on real `ostd` APIs:

- fixture manifest:
  [tools/lockdep/tests/fixtures/ostd-lockdep-cases/Cargo.toml](/root/asterinas-codex/tools/lockdep/tests/fixtures/ostd-lockdep-cases/Cargo.toml)
- fixture code:
  [tools/lockdep/tests/fixtures/ostd-lockdep-cases/src/lib.rs](/root/asterinas-codex/tools/lockdep/tests/fixtures/ostd-lockdep-cases/src/lib.rs)
- integration test:
  [tools/lockdep/tests/fixture_cases.rs](/root/asterinas-codex/tools/lockdep/tests/fixture_cases.rs)

The current fixture covers:

- ordinary lock-order cycle
- branch-dependent reverse lock order
- top-half IRQ context
- L1 bottom half
- L2 bottom half
- `disable_irq().lock()`
- IRQ-state merge after conditional control flow
- `RwLock<WriteIrqDisabled>` read/write mode distinction
- callable aliases
- callback wrappers
- nested callback wrappers
- returned guards through callback wrappers
- single-lock IRQ safety reporting
- IRQ dependency reporting
- AA/self-loop reporting
- IRQ reentry AA reporting

Test command:

```bash
cargo test --manifest-path tools/lockdep/Cargo.toml --test fixture_cases
```

## Configuration Support

The prototype has an early `lockdep.toml` configuration pipeline.

Currently supported:

- automatic loading from workspace root
- explicit path selection with `--config <path>`
- reporting config counts in terminal/JSON output
- `ignore` prefixes already affect summarized results
- `irq_entries`
- return-with-lock modeling for guard-returning helper functions
- local callback-wrapper propagation without extra `wrappers` config

Currently parsed but still inert:

- `ordered_helpers`

## What Is Still Missing

The prototype is useful, but it is not yet a finished lockdep tool.

Major missing pieces:

### 1. Full IRQ Safety Analysis

Still missing:

- full lockdep-style hardirq/softirq state classification
- derived safety state propagation
- stronger interrupt nesting reasoning

### 2. Better Interprocedural Precision

Still missing:

- richer function summaries
- recursive SCC-aware summary solving
- more general cross-crate return-with-lock / callback summary propagation
- better unknown-effect handling

### 3. Annotation / Config-Driven Precision

Still missing:

- `ordered_helpers` affecting analysis
- attribute-based equivalents such as:
  - `#[lockdep::ordered_by(...)]`
  - `#[lockdep::acquire_many(...)]`

### 4. More Complete AA Analysis

AA detection is now present, but still limited by:

- current lock-class precision
- heuristic IRQ reentry classification
- lack of dedicated suppression/config support

### 5. Deeper OSDK Integration

Current status:

- `cargo osdk lockdep` already exists
- the current OSDK entry shells out to the standalone `tools/lockdep` crate

Still missing:

- tighter OSDK-native environment preparation
- deeper CLI integration beyond shell-out orchestration

### 6. CI Integration

Still missing:

- CI job
- broader regression matrix
- non-blocking / blocking integration policy

## Practical Current Interpretation

Today, the prototype should be viewed as:

- already useful for exploring lock relationships
- already capable of detecting several real lock-pattern classes
- already much better than raw text scanning
- but still not final enough to treat every reported issue as a true deadlock

The most trustworthy current outputs are:

- lock events and contexts
- ordinary lock-order edges
- AA/self-loop signals in small targeted fixtures

The least mature parts are:

- full Linux-style IRQ safety inference
- configuration-driven precision
- cross-crate interprocedural accuracy in the full kernel

## Recommended Next Steps

If development resumes, the best next priorities are:

1. Add cross-crate summary propagation.
2. Make `ordered_helpers` affect analysis.
3. Strengthen Linux-style IRQ safety/state inference.
4. Expand the fixture suite into multiple focused crates.
