# Experiments Log

This file records each conversation round as a compact engineering log.

## Recording Template

```md
## Epoch N - YYYY-MM-DD HH:MM TZ
Question:
- ...

Answer Summary:
- ...

Operations:
- ...
```

## Epoch 1 - 2026-03-24 14:57 CST
Question:
- Should RustMap keep a backend GraphQL server, and how should the workflow and collaboration model be structured before implementation starts?

Answer Summary:
- No backend server in the current phase.
- Use `cargo-rustmap` to generate a JSON artifact, then upload it locally in the web frontend for visualization.
- Adopt phase gates: discuss and confirm scope/contract/acceptance before coding each phase.
- Keep reusable subagent templates as standalone files.

Operations:
- Rewrote `AGENTS.md` to remove backend GraphQL as the active path and define the JSON-artifact architecture.
- Added phase-gate collaboration rules and technical baseline (Rust 2024, rust-analyzer crates, React Flow + d3-force, `nix develop`).
- Added five subagent templates under `agents/`.
- Replaced `experiments.md` with Epoch-based recording format and initialized Epoch 1.

## Epoch 2 - 2026-03-24 15:21 CST
Question:
- What preparation decisions should be finalized before parser and frontend implementation starts?

Answer Summary:
- Lock parser boundary to workspace members only, with fail-fast behavior.
- Do not parse dependency source code, but include dependency metadata (`name + version`).
- Keep CLI minimal (`cargo rustmap [path] [--output]`) with default output at `./output/rustmap.json`.
- Keep frontend scope to local upload, strict validation, and basic interactions (selection, highlight, filtering).
- Use hand-drawn sketches and reference screenshots as primary design input.

Operations:
- Updated `AGENTS.md` with Phase 0 locked decisions, CLI/output contract, and command baseline.
- Added `output/` ignore rule to root `.gitignore`.
- Added parser preparation contract document at `artifacts/parser/PHASE0_PREP.md`.
- Added web preparation contract document at `artifacts/web/PHASE0_PREP.md`.
- Added fixture workspace at `examples/workspace_demo` for parser/frontend regression usage.

## Epoch 3 - 2026-03-24 15:26 CST
Question:
- How should milestone commits be managed across phases?

Answer Summary:
- Use one commit per milestone phase with concise standardized message format.
- Ask for explicit user approval every time before running `git commit`.
- Keep commit message compact and structured for consistent history.

Operations:
- Updated `AGENTS.md` with a milestone commit policy and commit-message template.
- Added explicit gate that commit action requires per-commit user confirmation.
- Recorded this workflow decision in `experiments.md`.

## Epoch 4 - 2026-03-24 16:12 CST
Question:
- Implement Phase 1 parser MVP with layered single-crate structure, mixed AST/semantic extraction, and `B + graph_index` JSON output.

Answer Summary:
- Implemented `cargo-rustmap` binary entry and parser pipeline in `src/{cli,workspace,extract,model,emit,error}.rs`.
- Added strict JSON contract with top-level `workspace/dependencies/crates/graph_index/warnings`.
- Kept visual node kinds focused on `struct/enum/trait/fn/type/const/static`, with `impl` represented as edge plus node property.
- Skipped `call` edges in Phase 1 and emitted warnings for unresolved `use/inherit`.

Operations:
- Added parser dependencies and binary target in `Cargo.toml`.
- Implemented workspace discovery, extraction, relation resolution, and JSON emission.
- Added tests for fixture extraction and output behavior under `tests/phase1_parser.rs`.
- Validated with `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, and a real output run to `/tmp/rustmap_phase1.json`.

## Epoch 5 - 2026-03-24 16:16 CST
Question:
- Migrate parser implementation out of root `src` into `artifacts/parser`, while keeping only a single binary entry in root `src/main.rs`.

Answer Summary:
- Parser modules were relocated to `artifacts/parser/src`.
- Cargo library target now points to `artifacts/parser/src/lib.rs`.
- Root `src` keeps only `main.rs` as the binary entry.

Operations:
- Moved `cli/emit/error/extract/lib/model/workspace` modules from `src/` to `artifacts/parser/src/`.
- Updated `Cargo.toml` with `[lib] path = "artifacts/parser/src/lib.rs"`.
- Updated `AGENTS.md` to document parser code location and root entrypoint policy.

## Epoch 6 - 2026-03-24 16:36 CST
Question:
- Refine parser JSON payload for enum internals, trait method ownership mapping, compact graph index fields, and free-function signature/type references.

Answer Summary:
- Added enum variant output with field metadata.
- Changed struct method output to include ownership source (`inherent` vs `trait`) and linked `trait_id`.
- Reduced redundant graph node fields; kept compact `id/kind/label` in `graph_index.nodes`.
- Added `fn_signature` for free functions, including params/return type and resolved `type_id` when possible.

Operations:
- Updated parser data model in `artifacts/parser/src/model.rs`.
- Updated extraction logic in `artifacts/parser/src/extract.rs` for enum/method/signature enrichment and compact serialization.
- Added and adjusted tests in `tests/phase1_parser.rs` for the new JSON behavior.
- Updated `AGENTS.md` contract section to reflect current Phase 1 JSON contract and deferred `call` edges.

## Epoch 7 - 2026-03-24 16:49 CST
Question:
- Add three frontend-oriented JSON enhancements before UI implementation: edge source context, edge-kind grouped index, and warning severity.

Answer Summary:
- Added `source_context` on each graph edge.
- Added `graph_index.by_edge_kind` for direct edge-kind lookup.
- Added `warnings[*].severity` and set current warnings to `warn`.

Operations:
- Extended schema in `artifacts/parser/src/model.rs` with `GraphEdgeRef`, `GraphEdge.source_context`, `GraphIndex.by_edge_kind`, and `WarningItem.severity`.
- Updated extraction and graph-index assembly in `artifacts/parser/src/extract.rs`.
- Updated parser tests in `tests/phase1_parser.rs` to verify the new fields.
- Regenerated `output/example.json` with the new JSON shape.

## Epoch 8 - 2026-03-25 18:42 CST
Question:
- Complete the frontend graph phase with robust interaction behavior, crate-level primary filtering, and smoother drag experience for multi-crate scale.

Answer Summary:
- Implemented a desktop-first RustMap shell with strict artifact upload validation, graph canvas rendering, left-rail filters, and inspector relation navigation.
- Added crate-first filtering pipeline (`crate -> kind`) and exposed crate filter controls above kind filter.
- Improved graph interaction stability by hardening d3-force/React Flow state handoff and replacing hard boundary snaps with velocity-based soft rebound.
- Kept contract consumption strictly additive on existing artifact fields (`crates`, `graph_index.by_container`, `graph_index.edges`).

Operations:
- Built frontend app architecture under `artifacts/web/src/app`, `components`, `lib`, and `styles` with React Flow + d3-force runtime.
- Added crate filter state/actions and two-stage graph filtering flow in app state and graph helpers.
- Updated relation navigation to auto-enable hidden target crate/kind before focus jump.
- Tuned drag lifecycle to avoid position race conditions and moved bounds handling to a continuous soft force model.
- Ran frontend verification with `nix develop --command pnpm --dir artifacts/web lint` and `nix develop --command pnpm --dir artifacts/web build`.
