# Web Phase 0 Preparation

## Goal

Define frontend constraints and acceptance boundaries before implementation.

## Data Source Policy

- Primary input is local JSON upload.
- One built-in sample JSON will be used for demo and regression checks.
- No backend data fetching in the current phase.

## Validation Policy

- Upload validation is strict.
- Required fields must exist and have valid types.
- Validation failures must show explicit, actionable error messages.

## Interaction Scope (v1)

- Node selection.
- Relation highlight.
- Basic filtering.

## Rendering Baseline

- Graph engine: React Flow (`@xyflow`).
- Force simulation: `d3-force`.
- Initial performance target: smooth interaction around <= 50 nodes.

## Design Input Policy

- Preferred input: hand-drawn wireframes and reference screenshots.
- Figma can be added as an enhancement source when available.

## Acceptance Gate

- Automation gate:
  - Lint passes.
  - Build passes.
- Visual and style quality is reviewed manually by user.

## Out of Scope

- Full design system implementation.
- Advanced analysis panels.
- Multi-file graph loading flows.
