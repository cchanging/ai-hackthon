# Parser Phase 0 Preparation

## Goal

Define implementation boundaries before parser coding starts.

## Input Boundary

- Parse all workspace member crates.
- Analyze project-owned source only.
- Do not parse third-party dependency source code.

## Dependency Metadata Rule

- Collect dependency metadata from manifest information.
- Output dependency list as:
  - `name`
  - `version`

## Extraction Scope (v1)

- Declarations:
  - modules
  - structs
  - enums
  - traits
  - functions
- Relations:
  - `impl`
  - `use`
  - `call`
  - `inherit`

## Failure Policy

- Fail-fast: stop on parse failure.
- Return non-zero process exit code.
- Emit actionable error text with file/module context when available.

## CLI Contract (v1)

- Command shape: `cargo rustmap [path] [--output]`
- Default output path: `./output/rustmap.json`
- `--output` overrides default output path.

## JSON Contract Policy

- No `schemaVersion` field in Phase 0.
- Keep contract changes additive whenever possible.
- Document any breaking change in `AGENTS.md` before implementation.

## Out of Scope

- Backend APIs.
- Runtime graph query services.
- Advanced semantic edge types not listed in this document.
