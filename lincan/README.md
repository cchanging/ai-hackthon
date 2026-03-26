<h1 align="center">RUSTMAP</h1>

<p align="center">
  Static Rust structure analysis and visualization for workspace projects.
</p>

## What Is RustMap

RustMap is a local-first toolchain that turns a Rust workspace into a single JSON artifact and visualizes it in an interactive graph UI.

Current phase focuses on project-owned source code:
- Parse all workspace member crates.
- Do not parse third-party dependency source code.
- Emit dependency metadata (`name + version`) in the artifact.
- Fail fast with non-zero exit when parsing fails.

## Main Features

1. Minimal CLI: `cargo rustmap [path] [--output <file>]`.
2. Single artifact output (default: `./output/rustmap.json`).
3. Contracted JSON structure for frontend consumption:
`workspace`, `dependencies`, `crates`, `graph_index`, `warnings`.
4. Core graph relations:
`impl`, `inherit`, `call`, `contain`.
5. Local web upload with strict contract validation and explicit error messages.
6. Interactive graph exploration:
selection, highlight, basic filtering, and depth-limited call subgraph view.

## Main Methods Used

1. Workspace discovery and dependency metadata collection with `cargo_metadata`.
2. Static extraction with Rust parsing/semantic tooling (`syn`, `proc-macro2`, `ra_ap_syntax`).
3. Contract-first artifact modeling and serialization (`serde`, `serde_json`).
4. Frontend graph rendering with React Flow (`@xyflow/react`) + force layout (`d3-force`).
5. Frontend schema validation and runtime guardrails with `zod`.

## Quick Start

```bash
# 1) Generate artifact
nix develop --command cargo rustmap examples/workspace_demo -output output/rustmap.json

# 2) Run web UI
cd artifacts/web
pnpm install
pnpm dev
```

Then upload `output/rustmap.json` in the web interface.
