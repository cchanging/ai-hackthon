# `lockdep.toml` Format

This document explains the workspace-level `lockdep.toml` file used by
`tools/lockdep`.

Current status note:

- `ignore` and `irq_entries` are active.
- guard-returning helper functions and local callback-wrapper propagation no longer require a
  separate `wrappers` config concept.
- `ordered_helpers` is still parsed but remains inert.

## Where It Is Loaded

By default, `cargo-lockdep` looks for `lockdep.toml` at the workspace root.

Examples:

```bash
cargo run --manifest-path tools/lockdep/Cargo.toml -- \
  -p aster-kernel --target x86_64-unknown-none -- --quiet
```

This automatically loads:

```text
<workspace-root>/lockdep.toml
```

You may also pass an explicit path:

```bash
cargo run --manifest-path tools/lockdep/Cargo.toml -- \
  --config lockdep.toml \
  -p aster-kernel --target x86_64-unknown-none -- --quiet
```

If the file does not exist, lockdep runs with an empty configuration.

## Top-Level Structure

The current parser accepts these top-level keys:

```toml
ignore = [
  "some::def_path::prefix",
]

[[irq_entries]]
function = "SomeType::some_entry"
context = "BottomHalfL1"
callback_arg_index = 1

[[ordered_helpers]]
function = "some::ordered::helper"
order = "ascending"
```

## `ignore`

`ignore` is a list of def-path prefixes.

Meaning:

- After crate artifacts are collected, any function whose `def_path` starts
  with one of these prefixes is removed from the summarized result set.
- This affects reporting output.
- It does not currently change how MIR is analyzed before summarization.

Example:

```toml
ignore = [
  "my_crate::debug_only",
  "my_crate::tests::",
]
```

## `irq_entries`

`irq_entries` declares that a specific function should be treated as an IRQ
entry registration helper.

Fields:

- `function`
  The callee def-path to match.
- `context`
  The execution context assigned to the registered callback.
- `callback_arg_index`
  Zero-based index into the call arguments of the registration function.

Current supported context strings are:

- `Task`
- `TaskIrqDisabled`
- `HardIrqTopHalf`
- `BottomHalfL1`
- `BottomHalfL1IrqDisabled`
- `BottomHalfL2`

Example:

```toml
[[irq_entries]]
function = "SoftIrqLine::enable"
context = "BottomHalfL1"
callback_arg_index = 1
```

This means:

- when lockdep sees a call to `SoftIrqLine::enable(...)`,
- it treats argument `1` as the callback,
- and marks that callback as running in `BottomHalfL1`.

## `ordered_helpers`

`ordered_helpers` is reserved for helpers that acquire multiple locks in a known
order.

Fields:

- `function`
  The helper def-path to match.
- `order`
  A textual description of the ordering policy.

Example:

```toml
[[ordered_helpers]]
function = "some::helper::lock_two"
order = "ascending"
```

Current status:

- This section is parsed.
- It is counted in terminal/JSON config summaries.
- It is not yet applied to the analysis.

## Current Status of Each Section

As of the current implementation:

- `ignore`: active.
- `irq_entries`: active.
- return-with-lock modeling for guard-returning helper functions: active.
- local callback-wrapper propagation without extra wrapper config: active.
- `ordered_helpers`: parsed, but not yet active.

## Current Workspace Example

The current workspace `lockdep.toml` looks like this:

```toml
[[irq_entries]]
function = "SoftIrqLine::enable"
context = "BottomHalfL1"
callback_arg_index = 1
```

## Practical Notes

- `function` must match the analyzed Rust def-path, not a file path.
- `callback_arg_index` is zero-based.
- guard-returning helper functions and local callback-wrapper propagation no longer require
  explicit `wrappers` config.
- If a config entry is spelled correctly but has no effect, the usual cause is
  that the real callee def-path does not exactly match the configured string.
