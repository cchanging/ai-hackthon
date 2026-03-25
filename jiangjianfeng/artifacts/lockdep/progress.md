# Lockdep Progress

This document records the implementation progress of the static lockdep tool
against the current `plan.md`.

## Scope

Current codebase status:

- Implemented code lives under [tools/lockdep](/root/asterinas-codex/tools/lockdep).
- Main implementation commits so far:
  - `52732bd35` `Add lockdep tool through phase 2`
  - `68c138252` `Add phase 3 IRQ context detection`
  - `e70004fcb` `Add phase 4 lock summary propagation`
  - `a9c86bdfa` `Add phase 5 global cycle reporting`
  - `4bf42e19c` `Add phase 6 graph export reporting`
  - `735bfe9cc` `Improve lockdep lock identity and lock modes`
  - `932ddb9b1` `Normalize ordered double-lock edges`
  - `37ce88e2c` `Refine deadlock cycle filtering`
  - `91e1db433` `Add minimal IRQ conflict reporting`
  - `860ed6d95` `Propagate IRQ contexts across direct calls`
  - `879a49a73` `Refine IRQ context reporting`
  - `c93c961dc` `Add lockdep config loading prototype`
  - `950f06707` `Add explicit lockdep config selection`
  - `3c756f31c` `Add lockdep fixture tests and ignore filtering`
  - `f529b51ec` `Add AA deadlock detection`
  - `06cd78e4e` `Add cargo osdk lockdep subcommand`
  - return-with-lock modeling and wrapper-config removal
  - `5a714d6a7` `Refactor lockdep IRQ execution state tracking`
  - `7d80799ed` `Propagate IRQ state to entry-lock summaries`
  - `e80f70980` `Propagate IRQ callsite contexts across calls`
  - `18797515e` `Resolve local callable aliases in lockdep`
  - `8cf6e12ff` `Propagate contexts through callback wrappers`
  - `d2435a9a0` `Rebind callback wrapper argument locks`
  - `5b32c8e92` `Propagate returned guards through callbacks`
  - `697e708a5` `Add wrapper callback regression coverage`
  - `e6f42f220` `Add callback return-guard regression coverage`
  - `a684be569` `Add aliased callback wrapper coverage`
  - `d8ffe1c9e` `Add callback return alias regression coverage`
  - `766c8f35d` `Propagate callback summary locks into wrappers`
  - `14b9d4224` `Propagate nested callback wrapper summaries`
  - `13b56ee44` `Track callable argument bindings precisely`
  - `cad887750` `Refactor callsite callee resolution`
  - `3a6121685` `Fix lockdep review regressions`
  - `363932f21` `Refactor callable summary expansion`

Important note:

- Commit titles after Phase 4 do not exactly match the phase numbering in `plan.md`.
- In terms of `plan.md`, the implementation is currently:
  - Phase 1: done
  - Phase 1.5: minimally done
  - Phase 2: done for MVP subset
  - Phase 3: partially done, but materially stronger than the original IRQ-label prototype
  - Phase 4: partially done, with significantly expanded local wrapper/callback summaries
  - Phase 5: partially done
  - Phase 5.5: minimally done; reverse-edge heuristic removed, annotation-driven support still missing
  - Phase 5.8: partially done
  - Phase 6: minimally done
  - Phase 7: not started

## Phase 0

Planned:

- Real lock-pattern sample collection
- Expected-result table
- Synthetic regression crates

Current status:

- Partially done informally.
- We already used real repository samples to drive implementation:
  - `disable_irq().lock()`
  - `RwLock<WriteIrqDisabled>`
  - `IrqLine::on_active`
  - `register_bottom_half_handler_l1`
  - `register_bottom_half_handler_l2`
  - ordered double-lock helpers such as futex/ext2
- We also produced a false-positive review document:
  - [lockdep_cycle_review.md](/root/asterinas-codex/tools/lockdep/lockdep_cycle_review.md)
- We now also have a checked-in fixture crate and integration test for supported cases:
  - [fixture_cases.rs](/root/asterinas-codex/tools/lockdep/tests/fixture_cases.rs)
  - [fixture crate](/root/asterinas-codex/tools/lockdep/tests/fixtures/ostd-lockdep-cases/src/lib.rs)

Gap vs plan:

- No dedicated “sample catalog” document yet.
- Synthetic test coverage now exists, but only for a single focused fixture crate.
- No broader expected-result matrix document yet.

## Phase 1

Planned:

- Independent tool skeleton
- `main.rs + driver.rs + analysis/`
- `rustc_driver` integration
- crate artifact export and front-end aggregation

Current status:

- Done.
- Implemented files:
  - [tools/lockdep/Cargo.toml](/root/asterinas-codex/tools/lockdep/Cargo.toml)
  - [tools/lockdep/src/main.rs](/root/asterinas-codex/tools/lockdep/src/main.rs)
  - [tools/lockdep/src/driver.rs](/root/asterinas-codex/tools/lockdep/src/driver.rs)
  - [tools/lockdep/analysis/src/lib.rs](/root/asterinas-codex/tools/lockdep/analysis/src/lib.rs)
- The tool runs as:
  - `cargo run --manifest-path tools/lockdep/Cargo.toml -- ...`
- It wraps Cargo with `RUSTC_WORKSPACE_WRAPPER`, exports per-crate JSON artifacts, and aggregates them in the front-end.

Gap vs plan:

- None at skeleton level.

## Phase 1.5

Planned:

- `--target`
- feature forwarding
- target-aware compilation for `ostd/kernel`

Current status:

- Minimally done.
- Implemented CLI forwarding:
  - `--target`
  - `--features`
  - `--no-default-features`
- Confirmed analysis works for supported target triples such as `x86_64-unknown-none`.

Gap vs plan:

- No dedicated `RUSTFLAGS` forwarding interface yet.
- No OSDK-aware environment preparation yet.
- Still driven by plain `cargo check`, not by `cargo osdk`.

## Phase 2

Planned:

- Standard lock API recognition
- intra-function lock acquire/release recovery
- lock dependency edges inside one function

Current status:

- Done for the current MVP subset.
- Implemented in:
  - [tools/lockdep/analysis/src/collect.rs](/root/asterinas-codex/tools/lockdep/analysis/src/collect.rs)
  - [tools/lockdep/analysis/src/model.rs](/root/asterinas-codex/tools/lockdep/analysis/src/model.rs)
- Supported primitives/methods:
  - `SpinLock::{lock, try_lock}`
  - `RwLock::{read, write, try_read, try_write}`
  - `Mutex::{lock, try_lock}`
  - `RwMutex::{read, write, try_read, try_write}`
  - `disable_irq()`
- Implemented:
  - guard local tracking
  - `StorageDead`
  - `Drop`
  - explicit `drop(...)`
  - function-local lock events
  - function-local lock edges
  - regression coverage through an `ostd`-API fixture crate

Gap vs plan:

- Still conservative.
- No precise alias analysis.
- No support for upgrade paths such as `upread -> upgrade`.
- No fully general return-with-lock summary model yet; the current implementation focuses on
  guard-returning helper functions and local callback-wrapper paths that stabilize through iterative local summaries.

## Phase 3

Planned:

- IRQ entry detection
- context state propagation
- `disable_irq` and IRQ-open/close tracking
- IRQ safety property tracking

Current status:

- Partially done.
- Implemented:
  - `IrqLine::on_active` entry recognition
  - `register_bottom_half_handler_l1` entry recognition
  - `register_bottom_half_handler_l2` entry recognition
  - initial function contexts in JSON output
  - L1 bottom-half entry `DisabledLocalIrqGuard` tracking
  - structured IRQ execution state tracking in MIR
  - propagation of IRQ entry contexts through crate-local direct calls
  - propagation through local callable aliases and callback wrappers
  - mode-aware IRQ conflict refinement for `RwLock` / `RwMutex`
  - single-lock IRQ safety violations
  - IRQ dependency violations
  - symmetric IRQ-state merge at CFG joins
  - per-event/per-edge context labels such as:
    - `Task`
    - `TaskIrqDisabled`
    - `HardIrqTopHalf`
    - `BottomHalfL1`
    - `BottomHalfL1IrqDisabled`
    - `BottomHalfL2`

Gap vs plan:

- No dedicated Linux lockdep-style derived IRQ state machine yet.
- Interprocedural context propagation is still limited to crate-local summaries.
- No richer context join rules for recursion/SCCs beyond set propagation.
- No cross-crate IRQ summary propagation yet.

## Phase 4

Planned:

- function summaries
- call-site summary propagation
- global graph construction
- SCC/cycle extraction

Current status:

- Partially done.
- Implemented:
  - direct-call collection
  - summary of “entry locks” per function
  - summary of returned locks
  - propagation from caller held locks to callee entry locks
  - propagation through local callable aliases / callback wrappers / nested callback wrappers
  - global graph aggregation
  - SCC/cycle extraction
- The tool now reports global cycles in the aggregated graph.

Gap vs plan:

- Summary model is still minimal.
- It does not yet model:
  - a fully general `returns_with_locks` lattice beyond the current guard-returning helper support
  - recursive SCC summaries in a principled way
  - unknown side effects
  - full context-sensitive interprocedural summaries
- Cross-crate propagation is still effectively artifact aggregation, not a richer summary engine.

## Phase 5

Planned:

- better reporting
- `lockdep.toml`
- ignore rules
- wrapper configuration
- uncertainty reporting

Current status:

- Partially done.
- Implemented:
  - human-readable terminal summary
  - JSON export
  - DOT export of the global lock graph
  - cycle witness output with origin function/context/location
  - stable lock-class identity that no longer directly merges raw MIR locals such as `(*_4)` across functions
  - basic lock-mode filtering for `RwLock` / `RwMutex` shared-read edges in the global graph
  - cycle-level blocking filter so non-blocking mixed-mode cycles are not reported as deadlocks
  - single-lock IRQ and IRQ dependency summaries in terminal and JSON output
  - compatibility `irq_conflict` output
- automatic loading of a workspace-root `lockdep.toml` prototype
- explicit `--config <path>` support
- `ignore` prefixes now affect the summarized result set
- `irq_entries` now affect IRQ entry recognition
- return-with-lock modeling now recovers acquisitions across guard-returning helper functions
- config summary reporting:
    - ignore count
    - IRQ entry count
    - ordered-helper count
- dedicated format documentation:
  - [lockdep-toml.md](/root/asterinas-codex/tools/lockdep/lockdep-toml.md)
- The current review document for false positives is:
  - [lockdep_cycle_review.md](/root/asterinas-codex/tools/lockdep/lockdep_cycle_review.md)

Gap vs plan:

- `ignore` and `irq_entries` now affect analysis.
- `ordered_helpers` is still parsed but inert.
- No structured uncertainty classification in output yet.

## Phase 5.5

Planned:

- annotation/config support for ordered multi-lock helpers
- normalization of ordered double-lock patterns
- elimination of known false positives

Current status:

- Partially done.
- The design has been added to [plan.md](/root/asterinas-codex/tools/lockdep/plan.md).
- The earlier heuristic normalization for reverse edges has been removed because it was unsound.
- Real branch-dependent reverse lock orders are now preserved and can produce true cycles.

Gap vs plan:

- No parser for:
  - `#[lockdep::ordered_by(...)]`
  - `#[lockdep::acquire_many(...)]`
  - equivalent `lockdep.toml` entries
- No annotation-driven normalization exists yet.
- It does not yet understand:
  - the actual ordering key (`ino`, bucket index, etc.)
  - batch-lock helpers with more than two locks
  - per-helper semantics from config or attributes

## Phase 6

Planned:

- integrate into `cargo osdk lockdep`

Current status:

- Minimally done.
- Implemented:
  - `cargo osdk lockdep`
  - OSDK command plumbing in:
    - `/root/asterinas-codex/osdk/src/cli.rs`
    - `/root/asterinas-codex/osdk/src/commands/lockdep.rs`
  - shell-out from the OSDK command into the standalone `tools/lockdep` crate

Gap vs plan:

- Integration is still shallow.
- The OSDK command is an orchestrating wrapper, not a deeper in-process integration.

## Phase 7

Planned:

- CI integration
- regression suite

Current status:

- Not implemented.

Gap vs plan:

- No CI job.
- No checked-in regression suite for the tool itself.
- No non-blocking or blocking pipeline integration.

## Current Strengths

- Works on real `ostd` and `aster-kernel` builds for `x86_64-unknown-none`.
- Can be invoked either directly or via `cargo osdk lockdep`.
- Can recover real lock events and many real lock edges.
- Can produce a global graph and report candidate cycles.
- Can export DOT for visualization.
- Can already suppress several high-noise false positives from local MIR ordering artifacts.
- Can recover lock edges hidden behind guard-returning helper functions without explicit wrapper config.

## Current Main Limitations

- Lock-class identity is improved, but still not fully instance-sensitive.
- `RwLock` / `RwMutex` mode compatibility is only partially modeled.
- Ordered multi-lock helper handling is still largely missing; the earlier reverse-edge heuristic was removed because it was unsound.
- IRQ conflict checking is not yet implemented in lockdep style.
- Annotation/config-based precision mechanisms are only partially implemented.

## Recommended Next Work

The most valuable next steps are:

1. Stabilize lock-class identity.
2. Add lock-mode compatibility filtering for `read`/`write`.
3. Implement ordered multi-lock annotation/config support.
4. Add real IRQ safety conflict checks.

These four steps are the shortest path from “useful prototype” to “actionable kernel deadlock detector”.

## Current Triage Snapshot

Recent `aster-kernel` results after the latest precision fixes:

- lock events: `2439`
- function-level/global lock edges: `637`
- candidate cycles: `1`
- AA/self-loop reports: `1`

The remaining reported cycle is currently the exfat self-deadlock path in
`ExfatInode::reclaim_space`.

Recent `ostd` results with the current implementation:

- lock events: `122`
- function-level lock edges: `3`
- candidate cycles: `0`
- IRQ conflicts: `0`

Recent `aster-uart` results with the current implementation:

- top-half entry context propagates from `arch::init::{closure#1}` into `console::UartConsole::<U>::trigger_input_callbacks`
- acquire events inside `trigger_input_callbacks` are now labeled `HardIrqTopHalf`

Recent `aster-kernel` results with the current implementation:

- lock events: `2439`
- function-level lock edges: `637`
- candidate cycles: `1`
- IRQ conflicts: `0`

Recent fixture test results with the current implementation:

- `cargo test --manifest-path tools/lockdep/Cargo.toml --test fixture_cases`
- status: passing
- covered cases:
  - ordinary lock-order cycle
  - branch-dependent reverse cycle
  - AA/self-loop deadlock
  - IRQ reentry AA risk
  - top-half IRQ context
  - L1 bottom half context
  - L2 bottom half context
  - `disable_irq().lock()`
  - IRQ-state merge after conditional control flow
  - `RwLock<WriteIrqDisabled>` read/write paths
  - local callable aliases
  - callback wrappers / nested callback wrappers
  - returned guards through callback wrappers
  - single-lock IRQ reporting
  - IRQ dependency reporting

## Phase 5.8

Planned:

- detect `A -> A` self-loops
- detect interrupt-assisted `A -> A` risks
- expose AA risks separately from ordinary cycles

Current status:

- Partially done.
- Implemented:
  - self-edges are emitted for deadlock-relevant same-lock acquisitions
  - AA reports are split out as:
    - `self_lock`
    - `irq_reentry`
  - the fixture crate asserts:
    - ordinary cycle detection
    - IRQ conflict detection
    - AA deadlock reporting

Gap vs plan:

- AA detection still shares the current lock-class precision limits.
- IRQ-assisted AA detection is still based on heuristic interrupt/interruptible context classes.
- AA reporting is present, but not yet configurable through `lockdep.toml`.
