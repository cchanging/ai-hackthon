# cargo-lockdep

`cargo-lockdep` is a prototype static lock dependency analyzer for Asterinas.

Status note:

- This document reflects the implementation state after the recent callback/wrapper summary work
  and the follow-up review fixes.
- The standalone frontend now works from both debug and release builds.
- The analyzer now propagates local callable aliases, callback wrappers, nested callback wrappers,
  and returned guards through local interprocedural summaries.

It is currently implemented as an independent tool crate under
[tools/lockdep](/root/asterinas-codex/tools/lockdep).

The tool can be invoked directly, and it is also wired into OSDK as
`cargo osdk lockdep`.

Direct entrypoint:

```bash
cargo run --manifest-path tools/lockdep/Cargo.toml -- --help
```

OSDK entrypoint:

```bash
cargo osdk lockdep --help
```

## What It Does

The tool uses `rustc_driver` to analyze MIR and extracts:

- lock acquire/release events
- function-local lock-order edges
- crate-local direct-call summary propagation
- local callable/callback-wrapper summary propagation
- a global lock dependency graph
- global cycle reports
- single-lock IRQ safety violations
- IRQ dependency violations
- compatibility `irq conflict` reports
- AA/self-loop deadlock reports

## Supported Lock Primitives

The current MVP focuses on `ostd` synchronization APIs:

- `SpinLock`
- `RwLock`
- `Mutex`
- `RwMutex`
- `disable_irq()`

## Supported Contexts

The current prototype recognizes these execution contexts:

- `Task`
- `TaskIrqDisabled`
- `HardIrqTopHalf`
- `BottomHalfL1`
- `BottomHalfL1IrqDisabled`
- `BottomHalfL2`

Entry detection currently supports:

- `IrqLine::on_active`
- `register_bottom_half_handler_l1`
- `register_bottom_half_handler_l2`

## Usage

Analyze a package directly:

```bash
cargo run --manifest-path tools/lockdep/Cargo.toml -- \
  -p ostd --target x86_64-unknown-none -- --quiet
```

Analyze the kernel package directly:

```bash
cargo run --manifest-path tools/lockdep/Cargo.toml -- \
  -p aster-kernel --target x86_64-unknown-none -- --quiet
```

Export JSON:

```bash
cargo run --manifest-path tools/lockdep/Cargo.toml -- \
  -p aster-kernel --target x86_64-unknown-none \
  --emit-json lockdep.json -- --quiet
```

Export DOT:

```bash
cargo run --manifest-path tools/lockdep/Cargo.toml -- \
  -p aster-kernel --target x86_64-unknown-none \
  --emit-dot lockdep.dot -- --quiet
```

Use an explicit config file:

```bash
cargo run --manifest-path tools/lockdep/Cargo.toml -- \
  --config lockdep.toml \
  -p aster-kernel --target x86_64-unknown-none -- --quiet
```

Use the OSDK subcommand:

```bash
cargo osdk lockdep \
  -p aster-kernel --target-arch x86_64 -- --quiet
```

## Current Output

The terminal summary currently reports:

- number of analyzed crates/functions
- lock event count
- lock edge count
- global cycle count
- single-lock IRQ violation count
- IRQ dependency violation count
- IRQ conflict count
- AA/self-loop count

The JSON report additionally includes:

- per-function contexts
- per-function lock events
- per-function lock edges
- per-function lock usage states
- global cycle reports
- single-lock IRQ safety reports
- IRQ dependency reports
- IRQ conflict reports
- AA deadlock reports

## Configuration

The current `lockdep.toml` support is intentionally minimal.

Supported today:

- automatic loading from workspace root
- explicit `--config <path>`
- config summary counts in output
- `ignore` prefixes affecting which functions are retained in the summarized result set
- `irq_entries` affecting IRQ entry-context detection
- return-with-lock modeling for guard-returning helper functions

Parsed but not yet applied to analysis:

- `ordered_helpers`

See also:

- [lockdep-toml.md](/root/asterinas-codex/tools/lockdep/lockdep-toml.md)

## Tests

The current fixture-based integration test is:

```bash
cargo test --manifest-path tools/lockdep/Cargo.toml --test fixture_cases
```

It uses the fixture crate at:

- [tools/lockdep/tests/fixtures/ostd-lockdep-cases/src/lib.rs](/root/asterinas-codex/tools/lockdep/tests/fixtures/ostd-lockdep-cases/src/lib.rs)

Covered scenarios include:

- ordinary lock-order cycles
- branch-dependent reverse lock orders that must remain visible as real cycles
- IRQ top half / bottom half contexts
- `disable_irq().lock()`
- IRQ state merge after CFG joins
- `RwLock<WriteIrqDisabled>`
- callable aliases
- callback wrappers
- nested callback wrappers
- returned guards through helper/callback wrappers
- single-lock IRQ safety reporting
- IRQ dependency reporting
- AA/self-loop reporting

## Current Limitations

This is still a prototype.

Known limitations:

- lock-class identity is improved but not fully instance-sensitive
- IRQ safety checking is stronger than the original heuristic prototype, but still not Linux lockdep-style complete
- `lockdep.toml` is only partially effective; `ignore` and `irq_entries` are active, but `ordered_helpers` is still inert
- ordered multi-lock annotations/config are not implemented
- cross-crate summary propagation is still not implemented
- `cargo osdk lockdep` currently shells out to the standalone tool crate

For a fuller project status view, see:

- [summary.md](/root/asterinas-codex/tools/lockdep/summary.md)
- [progress.md](/root/asterinas-codex/tools/lockdep/progress.md)
- [lockdep-toml.md](/root/asterinas-codex/tools/lockdep/lockdep-toml.md)
