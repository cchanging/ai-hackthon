# RustMap Project Guide

## Overview

RustMap is a static Rust structure analysis and visualization project.
The current delivery chain is:

`cargo rustmap` -> single JSON artifact -> local upload in web UI -> interactive graph exploration.

Current phase constraints:
- No backend service.
- No dependency source parsing.
- Focus on project-owned code structures and relations.

## Locked Decisions (Phase 0)

### Scope
- Parse all workspace member crates.
- Do not parse third-party dependency source code.
- List used dependencies as `name + version` in output metadata.
- Fail-fast on parse errors with non-zero exit.

### CLI and Artifact
- v1 CLI surface is minimal: `cargo rustmap [path] [--output]`.
- Default output path is `./output/rustmap.json`.
- `--output` can override the default path.
- `output/` must stay ignored by git.

### JSON and UI
- No `schemaVersion` field in Phase 0.
- v1 relations include `impl`, `inherit`, `call`, and `contain`.
- Frontend JSON upload validation is strict with explicit error messages.
- v1 frontend interactions include selection, highlight, basic filtering, and secondary call-subgraph exploration (depth-limited).
- Initial performance target is small graphs (about <= 50 nodes).
- Current frontend acceptance is desktop-first; mobile is not a release gate in this phase.

### Design and Quality Workflow
- Design input priority: hand-drawn wireframes and reference screenshots; Figma is optional enhancement.
- Style enforcement is tool-first:
  - Rust: `rustfmt` + `clippy`
  - Web: ESLint
- Comments should be minimal and only explain intent or boundaries when needed.
- Acceptance gate is automation-first: tests and compilation must pass; visual quality is manually reviewed by user.

## Architecture

### `artifacts/parser`

Owns:
- Workspace discovery and source collection.
- rust-analyzer based semantic extraction.
- JSON artifact assembly for frontend consumption.
- Error reporting in fail-fast mode.

Implementation location:
- Parser Rust code lives under `artifacts/parser/src`.
- Root `src` is reserved for the binary entry (`src/main.rs`) only.

### `artifacts/web`

Owns:
- Local JSON upload.
- Strict contract validation and actionable error feedback.
- Graph rendering with React Flow (`@xyflow`) + `d3-force`.
- Core interactions: selection, highlight, basic filtering, and secondary call-subgraph exploration.

Non-goals in current phase:
- Backend API.
- Online graph persistence.
- Advanced analytics panels.
- Full mobile parity with desktop interactions.

## Contract Policy

- Frontend consumes only JSON artifact contracts.
- Parser internals are not consumed by frontend directly.
- Contract changes should be additive whenever possible.
- Any breaking contract change must be documented in `AGENTS.md` in the same iteration.

Current JSON shape (Phase 1):
- Top-level fields: `workspace`, `dependencies`, `crates`, `graph_index`, `warnings`.
- Nested `crates/modules/items` is the human-readable source-oriented view.
- `graph_index` is the frontend-optimized view with minimal node payload (`id`, `kind`, `label`) plus `edges`, `by_kind`, `by_container`, `by_edge_kind`.
- `graph_index.edges` must include `source_context` for UI styling and legend grouping.
- `use` imports are not emitted as graph edges.
- Type-reference relations are emitted as `contain` edges.
- `graph_index` must include `method` nodes for impl methods.
- Enum items must include `enum_variants`.
- Struct items must include `struct_shape` (`named|tuple|unit`) and `struct_fields` (`name?`, `index?`, `visibility`, `type_expr`, `type_id?`).
- Struct items with trait implementations must include method source mapping (`source=inherent|trait` and `trait_id` when trait-based).
- Struct method entries must include `method_id` that points to a graph `method` node.
- Trait items must include `trait_methods` with `name`, optional `receiver`, and `fn_signature` (params/return with resolved `type_id` when available).
- Free function items must include `fn_signature` (params and return type with resolved `type_id` when available).
- Method items must include `owner_id`, `owner_kind`, `source`, `trait_id?`, and `fn_signature`.
- `warnings` entries must include `severity` (`warn` in current phase).

## Development Workflow Gates

Every phase follows this sequence:
1. Confirm scope, contract, and acceptance criteria with the user.
2. Resolve open technical questions.
3. Implement only after explicit phase approval.
4. If blocked, stop and ask immediately.
5. Before any `git commit`, ask for explicit user approval.

Reference phases:
- Phase A: parser extraction baseline.
- Phase B: JSON fixture and contract hardening.
- Phase C: frontend upload and graph MVP.
- Phase D: interaction refinement and review hardening.

## Milestone Commit Policy

- Create one commit per milestone phase.
- Include all intended changes of that phase in one concise commit.
- Commit only after explicit user confirmation for that specific commit action.

Commit message format:

```text
<PhaseLabel>: <short summary>
1. <change item>
2. <change item>
```

Example:

```text
Initial: phase-0 baseline
1. lock project workflow and contracts
2. add preparation docs and fixture workspace
```

## Command Baseline

Use `nix develop` for all local development commands.

Common checks:
- Environment:
  - `nix develop --command cargo --version`
  - `nix develop --command pnpm --version`
- Rust:
  - `nix develop --command cargo fmt --all -- --check`
  - `nix develop --command cargo clippy --all-targets --all-features -- -D warnings`
  - `nix develop --command cargo test`
- Web:
  - `cd artifacts/web && pnpm lint`
  - `cd artifacts/web && pnpm build`

## Subagent Templates

Subagent templates are stored in `agents/*.md`.

Roles:
- Rust worker
- Frontend worker
- Rust code reviewer
- Frontend code reviewer
- Frontend designer

Each template must include:
- Scope boundaries.
- Input and output definitions.
- Quality gates.
- Prohibited actions.
- Handoff summary format.

## Language and Collaboration

- Communication with user: Chinese.
- Code, docs, and comments in repository: English.
- Keep communication concise and actionable.
- Ask immediately when risk or ambiguity can affect correctness.

## Documentation Sync Policy

- `AGENTS.md` is the living architecture and collaboration contract.
- When user direction changes, update this file in the same iteration.
- Keep updates concise, explicit, and implementation-oriented.
- Remove or rewrite outdated statements immediately.

## Engineering Principles

Primary goal: elegant, minimal, readable code.

### Simplicity
- Prefer direct data flow over clever abstraction.
- Minimize indirection.
- Remove dead paths quickly.

### Readability
- Keep modules small with clear ownership.
- Use predictable names and explicit boundary types.
- Write comments for intent, not obvious mechanics.

### Maintainability
- Keep parser/frontend contract stable.
- Add tests for parser extraction and contract-critical frontend behavior.
- Avoid scope creep outside the approved phase.

### Performance (Pragmatic)
- Optimize after measurement or obvious pain.
- Avoid premature optimization during schema and MVP stage.
- Keep graph operations responsive for target node scale.

## References

- [rust-analyzer](https://github.com/rust-analyzer/rust-analyzer)
- [React Flow](https://reactflow.dev/)
- [d3-force](https://github.com/d3/d3-force)
- [Nix](https://nixos.org/)
