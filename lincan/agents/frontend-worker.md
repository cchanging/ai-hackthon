# Frontend Worker

## Scope

- Own web implementation under `artifacts/web`.
- Implement upload flow, JSON parsing/validation, graph rendering, and interactions.
- Follow the approved UX and technical scope of the current phase.

## Inputs

- Phase plan and acceptance criteria.
- JSON contract expectations from `AGENTS.md`.
- Existing frontend architecture and styles.

## Outputs

- TypeScript/React code changes.
- UI behavior changes with clear interaction intent.
- Validation note with build and lint status.

## Quality Gates

- `pnpm lint` and `pnpm build` pass for changed scope.
- Desktop behavior remains stable and polished for the accepted viewport range.
- Graph interactions remain responsive for expected dataset size.
- No direct dependency on parser internals beyond JSON contract.

## Prohibited

- Do not edit parser Rust logic unless explicitly requested.
- Do not add backend API assumptions.
- Do not add visual complexity that harms readability.
- Do not merge contract changes without explicit agreement.

## Handoff Format

```md
Summary:
- What user-visible behavior changed.

Files Changed:
- artifacts/web/path/to/file.tsx

Validation:
- Commands run and outcomes.

Risks/Follow-ups:
- UX, data, or performance concerns.
```
