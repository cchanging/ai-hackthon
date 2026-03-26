---
name: asterinas-verify
description: Verify an Asterinas change, syscall, or module through one entry skill. Use when you need to split a change into review units, choose the right verification path, stress user-space input corner cases, generate a report with a Findings section, add general regression tests when appropriate, and use targeted tests to confirm uncertain behavior on Linux or Asterinas. Prefer subagents aggressively so the main agent stays focused on orchestration and final judgment.
---

# Asterinas Verify

Use this as the main entry point for Asterinas verification work.

Supported forms:

- `change <git-range>`
- `patch <patch-file>`
- `files <path...>`
- `syscall <name-or-path>`
- `module <path...>`

Entry rules:

- `change` / `patch` / `files` → change profile; use when the scope may span multiple areas and needs unit splitting first.
- `syscall` → syscall profile; use only when the target is one Linux-facing syscall entry file.
- `module` → module profile; use when the scope is already one subsystem/feature surface (trait, hook, or implementation area).

## Profile selection

Choose exactly one verification profile:

- `change`: the input is a diff, patch, changed file list, or broader change goal
- `syscall`: the target is one Linux-facing syscall entry under the repo root’s `kernel/src/syscall/`
- `module`: the target is a subsystem, implementation area, trait, callback, or feature interface

If the input mixes several areas, start in `change` mode, split it into review units, then route each unit to the right downstream profile.

## Workflow

1. Normalize the input.
If the request is change-oriented, classify review units with `./scripts/classify_review_unit.py`.
If the request is already one syscall or one module, skip directly to the chosen profile.

Or run `./scripts/prepare_review.py` to generate a compact JSON manifest (repo root, changed paths, review units, default report paths, and the structured finding schema) that subagents can consume directly without free-form prompting.
The classifier now splits module-sized units by subdomain/subdir heuristics to avoid giant mixed units.

2. Initialize the report.
Use `./scripts/init_report.py` with `change`, `module`, or `syscall`.
The report must include a `Findings` section near the top-level summary.
This section should list the final confirmed items that matter to the user first, ordered by severity, with class and file references.
Use the card-style layout: short claim, anchors, evidence IDs, class, confidence, and test intent.

3. Delegate early.
Prefer spawning subagents as soon as you know the review units.
Keep the main agent responsible for:

- choosing the profile
- deciding the validation plan
- integrating subagent outputs into the report
- making the final classification and user-facing conclusion

Push token-heavy work down to subagents whenever the platform supports them.
In particular, do not let the main agent spend tokens on repeated large diff reads, line-number harvesting, or table drafting when those can be scoped to subagents.

4. Build the evidence ladder.
Use these references:

- `./references/evidence-ladder.md`
- `./references/behavioral-spec-patterns.md`
- `./references/report-schema.md`

5. Expand the user-space input surface before final judgment.
For correctness review, explicitly enumerate the user-controlled inputs and the corner cases they create.
At minimum, consider the cases that fit the target surface:

- null, empty, or zero-length user inputs
- maximum-length, truncated, or boundary-sized buffers and paths
- invalid, partially valid, or misaligned user pointers
- flag combinations, reserved bits, and mutually inconsistent arguments
- repeated calls, partial progress, retry paths, and state transitions that cross success/error boundaries
- interactions between user input validation and downstream helper assumptions

Record the relevant corner cases in the report even when they do not become findings.

6. Plan validation from one mapping layer.
Use `./scripts/select_test_family.py`.
Use its `validation_feasible` and `candidate_subdir` outputs to decide early whether a unit should stay `report-only`.

7. Run validation only through `../asterinas-test/SKILL.md`.
Do not start execution as soon as you find the first bug.
Finish the current review pass first, decide which findings are real bugs, add all required regression tests, and then run `asterinas-test` once on the full target set whenever feasible.
When a confirmed bug has a crisp user-visible oracle and no concrete execution blocker, implement the regression test in the current turn instead of leaving it only under `Candidate Regression Tests`.
If a suspected correctness issue is still materially uncertain after code and spec review, and a targeted test can provide a crisp oracle, generate that test early and use the result as additional evidence before the final classification.
Treat such runs as confirmation work, not as a replacement for the evidence ladder, and batch multiple confirmation tests together whenever feasible instead of running one-off probes.

8. Draft the report quickly.
If findings are already structured JSON, use `./scripts/render_findings.py --findings <file>` to emit the findings cards and evidence appendix markdown, then paste into the report to avoid manual formatting.

9. Do not try to fix the found bugs.

## Subagent strategy

Default to subagents unless the task is already tiny.

Use subagents for:

- one review unit at a time when a change splits into several units
- Linux semantics research for a syscall unit
- external or upstream evidence collection for a module unit
- exact evidence extraction for one review unit, including file/line references
- user-space corner-case enumeration for one review unit when the reachable input surface is large
- report drafting for populated sections such as `Changed Paths`, `Review Units`, `Findings`, and `Behavior Matrix`
- test implementation for one general test family
- independent validation or log reading that would otherwise flood the main context

Keep ownership narrow:

- one research subagent per syscall or module unit
- one evidence-citation or report-drafting subagent when formatting the report would otherwise require substantial local reading
- one test subagent per top-level general test family
- one aggregation pass on the main agent

Do not give one subagent a mixed bag of unrelated units, research, test writing, and final classification.

When a task has several review units, the expected shape is:

1. main agent classifies units and opens the report
2. research subagents gather evidence for each unit in parallel
3. each unit subagent returns structured findings with exact file/line citations, candidate regression-test ideas, and the most relevant user-space corner cases it checked or wants covered by tests
4. optionally use one report-drafting subagent to convert those structured findings into report-ready rows and a concise `Findings` section
5. main agent consolidates the findings and determines which items are real bugs versus unsupported or open questions
6. test subagents implement concrete regression tests for all confirmed bugs and targeted confirmation tests for uncertain corner cases with crisp oracles
7. main agent batches the runnable targets and calls `asterinas-test` once for execution whenever feasible

## Report requirements

Every final review document must contain a `Findings` section.

The `Findings` section should:

- appear before long evidence tables when feasible
- list only the final user-relevant findings, not every intermediate suspicion
- order items by severity first
- include the finding class (`bug`, `unsupported`, `contract-risk`, `regression-risk`, or `open-question`)
- include anchor file references for each item
- mention the relevant user-space input corner cases when they materially affect the claim
- state whether the attached test intent is for `regression` or `confirmation`

Keep the detailed evidence and comparison tables elsewhere in the report.

## Token control

When the input is a non-trivial change, the main agent should avoid these token-heavy steps unless no delegation path is available:

- reading the full multi-file diff locally after review units are already known
- re-reading unit-local diffs just to collect exact line references
- manually drafting large report tables from subagent findings
- doing a second local pass whose only purpose is formatting evidence that a subagent could already return in structured form

Preferred shape:

1. main agent classifies units and initializes the report
2. subagents inspect scoped paths and return structured findings with citations following the documented schema
3. one optional subagent drafts report-ready bullets or table rows from structured findings
4. main agent performs only final judgment, validation planning, and final report integration

## Profile rules

### Change

- Split mixed diffs into review units before semantic judgment.
- Keep the top-level report as the aggregation artifact.
- Route `syscall` units through the syscall profile and `module` or `feature-interface` units through the module profile.
- Prefer one subagent per review unit once the units are known.
- Prefer one additional report-drafting subagent once the unit findings are stable if the report body will otherwise require substantial manual synthesis.
- Do not have the main agent open large per-unit diffs just to restate findings already available from subagents; ask unit subagents to provide exact citations and concise report-ready findings.
- Do not run validation per unit as soon as the first bug appears; batch execution after the change-level bug set and tests are ready.
- Aggregate user-space input corner cases across units so the top-level report shows which externally reachable edges were actually checked.

### Syscall

- Prefer Linux man-pages and Linux source as the governing semantics.
- Keep a Linux semantics matrix separate from the Asterinas comparison matrix.
- Usually emphasize `bug`, `unsupported`, and `open-question`, but the common five-class taxonomy still applies.
- Prefer a dedicated research subagent for Linux semantics and a separate test subagent if regression tests are needed.
- Confirm the full bug set for the syscall before starting execution, then run the generated test targets together when feasible.
- Treat syscall arguments as the primary user-space input surface; explicitly review pointer validity, buffer length boundaries, empty inputs, flag combinations, and partial-success/error transitions.

### Module

- Prefer behavior-oriented contracts over file-level commentary.
- State clearly when the contract is formal versus implementation-derived.
- Use `contract-risk` or `regression-risk` when adoption or evidence is incomplete.
- Prefer separate subagents for upstream evidence search and for test work when both are needed.
- Determine the concrete bug set before running tests, then batch the generated targets into one final `asterinas-test` invocation whenever feasible.
- When a module is reachable from syscalls or other user-controlled interfaces, trace how user input is normalized before it hits the module and check boundary assumptions at that handoff.

## Constraints

- Treat this skill as the canonical verification entry point.
- Reuse the skill scripts and references instead of copying local workflow text.
- Repo detection: scripts default to the current git toplevel (or cwd) as repo root and place reports under `<repo>/change-review` or `<repo>/syscall-review` unless `--output` or `--repo` is provided.
- Report layout: executive summary + findings cards (with evidence IDs) + evidence appendix + validation log. Keep wide matrices only where they add value (syscall semantics and comparison).
- Keep validation on `general test + verify` or `report-only`.
- For correctness review, explicitly examine user-controlled boundary, error, and state-transition cases before concluding that no bug exists.
- Implement tests when the behavior is concrete, user-visible, and has a crisp oracle.
- If such a test is feasible in the current environment, implement it in the same turn instead of leaving it only as a candidate.
- If a corner-case suspicion remains uncertain, use a targeted confirmation test when it can materially reduce uncertainty with a crisp oracle; record that the test was used for confirmation.
- Leave an item under `Candidate Regression Tests` only when the oracle is not crisp, the environment is blocked, or adding coverage would still be speculative; state that reason in the report.
- Optimize for low main-agent token use by delegating unit-local exploration, evidence collection, and test work whenever feasible.
- Determine the bug set before execution.
- Generate all regression tests for confirmed bugs before calling `asterinas-test`.
- Prefer one final batched `asterinas-test` run over repeated incremental runs unless isolation is required to make progress.

## Resources

- Verification toolkit: `./scripts/` and `./references/`
- Shared test entry: `../asterinas-test/SKILL.md`
