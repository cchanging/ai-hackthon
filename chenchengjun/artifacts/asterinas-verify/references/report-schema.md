# Asterinas Review Report Schema

All review flows share one reporting model.

## Common fields

Every report should make these items easy to locate:

- review scope
- findings
- user-space input corner cases that were checked
- evidence sources
- behavior or semantics matrix
- finding classes
- candidate or implemented regression tests
- confirmation tests used to resolve uncertainty
- validation status
- open questions

## Structured finding schema (for subagent outputs)

- `id`: stable identifier for the finding.
- `class`: `bug | unsupported | contract-risk | regression-risk | open-question`.
- `claim`: one-sentence summary of the user-relevant issue.
- `anchors`: list of `<repo-relative-path>:<line?>` that pin the finding.
- `input_cases`: optional list of user-space corner cases or state transitions checked for this finding.
- `evidence`: list of items, each with:
  - `id`: short label (e.g., `E1`).
  - `source`: `spec | linux-derived | asterinas-contract | diff-intent`.
  - `path`: `<repo-relative-path>:<line?>`.
  - `text`: short paraphrase or snippet.
- `test`: optional object with `kind (general|report-only)`, `module`, `goal (regression|confirmation)`, `idea`, and `oracle`.
- `confidence`: `low | med | high`.

## Report layout (rendered by init_report templates)

- Executive Summary: slug, date, reviewer, scope, validation plan, status.
- Findings cards: severity-ordered bullets; each card references evidence IDs and anchors, and should include `input_cases` and `test.goal` when relevant.
- User-space Input Corner Cases: concise list of the externally reachable edges the reviewer checked, even when they produced no finding.
- Evidence Appendix: table with `ID | Source | Path | Note | Supports (Finding IDs)`.
- Validation Log: planned targets, confirmation tests used to resolve uncertainty, commands run, outcomes (pass/fail + note) to make auditing easy.
- Optional matrices (syscall only): Linux semantics and comparison matrices should reference evidence IDs instead of repeating long text.

## Finding classes

Use this common finding taxonomy across change, module, and syscall review:

- `bug`
- `unsupported`
- `contract-risk`
- `regression-risk`
- `open-question`

Profiles may emphasize a subset, but change-level aggregation should not need class translation.

## Profile notes

### Change profile

The top-level report is the aggregation layer. It should include a `Findings` section near the summary, then list review units, routed profiles, and merged findings.

### Module profile

The module report should include a concise `Findings` section before the detailed matrices, then focus on behavior surfaces, governing semantics, contract boundaries, and the user-controlled edges that reach the module.

### Syscall profile

The syscall report should include a concise `Findings` section before the detailed matrices, explicitly enumerate the user-space input surface, and keep a Linux semantics matrix separate from the Asterinas comparison matrix.
