---
name: xfstests-fix
description: analyze an xfstests failure, resolve the standard artifacts from the failing case name, diagnose the likely root cause in the asterinas codebase, and carry one repair round through the trellis workflow under the current session constraints. use when given a failing xfstests case such as `generic/038`, a follow-up rerun log, or a request to patch the relevant filesystem or kernel path rather than only summarize the failure. this skill must obey the active trellis workflow, session-context.md, current task scope, gate status, fixed artifact roots, and validation defaults before planning, editing, or claiming a fix.
---

# Xfstests Fix

Use this skill to handle one xfstests failure-repair cycle at a time.

Treat every code-changing repair as a Trellis task. Read `/root/asterinas/.trellis/workflow.md` before planning the round. Do not edit code until the corresponding Trellis gate is cleared.

## Current Constraints Are Mandatory

Treat the active Trellis workflow, the current session context, and the current round's task scope as hard constraints, not background guidance.

Before planning, editing, validating, or reporting status, first determine what the current constraints allow. These constraints include at least:

- the active Trellis workflow in `/root/asterinas/.trellis/workflow.md`
- the current round's `session-context.md`, when present
- the selected or already-active Trellis task
- the current gate status and whether Gate 1 or Gate 2 approval is still pending
- the fixed artifact roots and artifact resolution rules
- the current round's validation defaults and any explicit overrides already recorded for the session

If any current constraint conflicts with a generic habit of “just diagnose and patch”, follow the current constraint.

Do not treat missing gate approval, missing task ownership, or session-specific validation requirements as optional. They are blocking conditions.

## Constraint Precedence

When multiple instructions or defaults are present, apply them in this order for the current round:

1. explicit human instruction in the current conversation
2. the active Trellis workflow and current gate requirements
3. `session-context.md` for the current round
4. the currently selected or active Trellis task scope
5. explicit per-round overrides already recorded in the session
6. this skill's generic defaults

Never use a lower-priority default to bypass a higher-priority current constraint.

## Trellis Workflow

Run the repair round through the Trellis protocol, not as an ad hoc patch:

- Run `python3 ./.trellis/scripts/get_context.py` and `python3 ./.trellis/scripts/task.py list` before planning.
- Reuse the current xfstests task when it already covers the round. If `session-context.md` names a task, use that. Otherwise prefer an existing xfstests task such as `03-24-xfstests-fix-master-tracker`. If no suitable task exists, create one with `python3 ./.trellis/scripts/task.py create "<title>" --slug "<slug>"` and then start it with `python3 ./.trellis/scripts/task.py start <task>`.
- Classify the round using the Trellis decision matrix. Default to `Standard`. Escalate to `Spec-driven` when the fix touches filesystem or kernel contracts, Linux-visible semantics, persistence rules, or concurrency-sensitive code. Use `Quick Fix` only when the change is tiny, obvious, and zero-risk.
- For `Standard` or `Spec-driven` repairs, run `$brainstorm`, create `prd.md`, and stop at Gate 1 for human review before editing any code.
- For `Spec-driven` repairs, run `$spec-creator`, write the `.spec`, and stop at Gate 2 for human review before editing any code.
- Before editing, read the approved `prd.md`, the approved `.spec` when present, and the relevant implementation and guide files for the touched subsystem.
- After implementing the patch, run the Trellis review gate: the appropriate mechanical validation for the touched scope, `$code-style-review`, and `/trellis:finish-work`. Also run `$spec-linux-validator` when a spec exists, and `$ext2-concurrency-review` when the fix changes Ext2 locking, inode state transitions, or lock-release choreography.
- Stop at Gate 3 with the diff, validation outputs, report paths, and manual rerun instructions. Never run `git commit`.

## Blocking Conditions

Stop and report the blocking condition instead of continuing when any of the following is true:

- `failing_case` is missing
- the current task owner is still unresolved
- Gate 1 approval is required but not cleared
- Gate 2 approval is required but not cleared
- the standard artifacts cannot be resolved from the current constraints and no alternative artifact was explicitly provided
- the session requires a specific validation or handoff format that has not yet been established

Do not silently fall back to ad hoc repair behavior when a blocking condition is active.

## Inputs

Read `./references/session-context.md` first when it exists. Treat it as the authoritative source for the current round's constraints, including task ownership, fixed roots, validation defaults, manual validation command, and any session-specific overrides.

For the current round, prefer session-recorded constraints over generic defaults. Generic defaults apply only when the session context does not override them.

Resolve artifacts from `failing_case` instead of asking the user for per-round paths:

- error-log root: `/root/xfstests/error_log/`
- report root: `/root/asterinas/.trellis/reports/xfstests/`
- follow-up kernel log: `/root/asterinas/qemu.log`

Derive the concrete artifacts as follows:

- `error_log` = `<error_log_root>/<failing_case>.log`
- `case_slug` = replace `/` in `failing_case` with `-`
- `report` = the lexicographically last file in `report_root` whose basename contains `case_slug`

Examples:

- `generic/001` -> `/root/xfstests/error_log/generic/001.log`
- `generic/038` -> report match `/root/asterinas/.trellis/reports/xfstests/...generic-038.md`
- if both `xfstests-analysis-2026-03-19-092912-generic-011.md` and `xfstests-analysis-2026-03-19-093210-generic-011.md` exist, choose the latter because it is lexicographically later and therefore newer by the timestamped naming convention

Expect these inputs when available:

- failing xfstests case name
- optional path to extra artifacts such as kernel console logs, dmesg extracts, or reproducer notes
- optional Trellis task name or directory override
- optional manual validation command override
- optional path to the next-round log or report that appears after manual rerun

Use the concrete examples in `./references/session-context.md` for the expected field format and path-resolution rule.

Do not ask the user for `error_log` or `report` paths when `failing_case` is known. Ask only if `failing_case` itself is missing, or if the standard roots no longer contain the derived artifacts and the user wants to continue with nonstandard artifacts.

## Workflow

### 1. Establish the current constraints before any repair work

- Read `/root/asterinas/.trellis/workflow.md`.
- Read `./references/session-context.md` first when it exists.
- Run `python3 ./.trellis/scripts/get_context.py` and `python3 ./.trellis/scripts/task.py list`.
- Determine which Trellis task currently owns this round before doing any code work.
- Determine the current gate status before planning or editing.
- Determine whether the session already fixes the task, scope, validation command, artifact roots, or report path conventions.

Do not plan around generic defaults until you have checked the active session constraints.

Do not edit code unless the current round's constraints and gate status explicitly permit it.

### 2. Enter the Trellis workflow

- Resolve which task owns the repair round before code changes. Prefer the task recorded in `session-context.md`, then an existing xfstests task, then create a dedicated task if needed.
- Choose the Trellis path and satisfy its gate requirements before editing:
  - `Q&A`: answer directly when the user only wants diagnosis.
  - `Quick Fix`: only for tiny obvious zero-risk changes.
  - `Standard`: use for most repair rounds.
  - `Spec-driven`: use for contract, filesystem semantic, persistence, or concurrency-sensitive fixes.
- Do not edit code until Gate 1 is cleared, and Gate 2 as well when the round is `Spec-driven`.

### 3. Read the failure artifacts

- Resolve the concrete `error_log` first as `<error_log_root>/<failing_case>.log`.
- Resolve the concrete `report` by searching `report_root` for filenames containing the normalized case slug and picking the lexicographically last match.
- Read the resolved `error_log` and `report` before touching code.
- Extract the exact failing case, syscall or operation, observed error, expected behavior, and whether the failure happened on the test device or scratch device.
- Record the first concrete symptom rather than a broad summary, for example wrong errno, stale metadata, missing persistence, incorrect rename behavior, or lock-order fallout.

### 4. Build a root-cause hypothesis

- Map the symptom to the relevant Asterinas subsystem and code path.
- Prefer primary implementation paths over speculative global explanations.
- Read Linux's behavior under `/root/linux` and compare the observed behavior with the intended Linux-visible semantics when the test is asserting Linux compatibility.
- Call out assumptions explicitly when the log does not prove the exact root cause.

### 5. Write the Trellis planning artifacts

- For `Standard` and `Spec-driven` rounds, run `$brainstorm` and write `prd.md` with goals, scope, acceptance criteria, and whether a spec is required.
- Stop for human review at Gate 1 after `prd.md`.
- For `Spec-driven` rounds, run `$spec-creator`, write the `.spec`, and stop for human review at Gate 2.
- Once the gate is cleared, reread the approved planning artifacts before editing code.

### 6. Fix the code

- Implement the smallest change that addresses the concrete failure.
- Preserve existing behavior outside the failing path unless the logs show a wider contract violation.
- Add or adjust local comments only when they explain a subtle invariant, ordering rule, or failure guarantee.
- Do not revert unrelated work in the tree.


### 7. MUST: Dispatch a subagent to review the changes: use $kernel-architecture-audit

### 8. MUST: Fix with the subagent report in step.7

### 9. Run Trellis verification and hand off manual rerun

- Run the appropriate mechanical validation for the changed scope before handoff.
- Run `$code-style-review` and `/trellis:finish-work` for every code-changing round.
- If a `.spec` exists, run `$spec-linux-validator`.
- If the patch changes Ext2 locking, lock ordering, or lock-release windows, run `$ext2-concurrency-review`.
- Do not run the final xfstests validation command unless the user explicitly asks.
- If `session-context.md` provides `manual_validation_command`, use it.
- Otherwise derive the default validation command as `make run_kernel AUTO_TEST=xfstests RELEASE=1 MEM=12G XFSTESTS_ARGS='<failing_case>'`.
- Stop at Gate 3 with the diff, validation outputs, and what rerun result would confirm the fix.

### 10. Read follow-up artifacts

- After the user reruns validation and provides new logs or reports, read the new artifacts before drawing conclusions.
- Decide which of these states applies:
  - the original failure is fixed and no new failure replaced it
  - the original failure is fixed but another case now fails
  - the original failure still reproduces and the previous hypothesis was incomplete or wrong
- Base the conclusion on the artifacts, not on build success alone.

## Output Requirements

When reporting analysis or a fix, include:

- the failing case name
- the current-round constraints that governed this step
- the key symptom from the artifacts
- the concrete root-cause hypothesis
- the files changed and why
- what the user should rerun manually
- whether the latest artifacts show pass, partial progress, or unchanged failure

## Guardrails

- Do not bypass the Trellis gates. No code edits before Gate 1, or Gate 2 when a spec is required.
- Do not skip `prd.md` for non-trivial repair rounds.
- Do not treat a kernel or filesystem semantic repair as a casual quick fix when the Trellis matrix points to `Standard` or `Spec-driven`.
- Do not claim the issue is fixed without reading the post-rerun artifacts.
- Do not overfit to a single stack trace if the report shows a broader semantic mismatch.
- Do not skip the report file when both `error_log` and `report` are available; they often provide different evidence.
- Do not ask the user for standard artifact paths when they can be resolved from `failing_case` and the fixed roots above.
- Do not guess the per-case error-log or report path from anything other than `failing_case` plus the resolution rule documented in `session-context.md`.
- Do not broaden the patch unnecessarily when the failing contract is narrow.
- Do not run `git commit`.
- Do not mark the repair round complete without the Trellis review gate outputs and the later session recording step once the human has committed.
- Do not replace current-round constraints with generic defaults just because the defaults are easier to execute.
- Do not infer gate clearance, task ownership, or validation scope from habit; verify them from the current workflow and session context.
- Stop and report the blocking condition when the current round does not yet permit planning, editing, or handoff.

## Session Template

Use `./references/session-context.md` as the reference template for each repair round. In the normal workflow, only `failing_case` needs to change from round to round.