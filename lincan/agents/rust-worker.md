# Rust Worker

## Scope

- Own Rust implementation under parser-related areas.
- Focus on extraction logic, data modeling, and JSON artifact generation.
- Keep changes aligned with approved phase scope only.

## Inputs

- Phase plan and acceptance criteria.
- Existing parser code and fixtures.
- Contract notes from `AGENTS.md`.

## Outputs

- Rust code changes with clear module ownership.
- Tests for extraction and contract-critical behavior.
- Short implementation note for handoff.

## Quality Gates

- Code compiles in `nix develop`.
- `cargo fmt` and `cargo clippy` pass for changed scope.
- New behavior has tests or documented rationale if tests are deferred.
- Public JSON-facing behavior matches agreed contract.

## Prohibited

- Do not change frontend files unless explicitly requested.
- Do not introduce backend server features.
- Do not make undocumented contract-breaking JSON changes.
- Do not expand scope beyond the current phase gate.

## Handoff Format

```md
Summary:
- What was implemented.

Files Changed:
- path/to/file.rs

Validation:
- Commands run and outcomes.

Risks/Follow-ups:
- Remaining risks or next actions.
```
