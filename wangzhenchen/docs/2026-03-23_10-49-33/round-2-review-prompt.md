# Code Review - Round 2

## Original Implementation Plan

**IMPORTANT**: The original plan that Claude is implementing is located at:
@docs/asterinas-scml-diff-plan.md

You MUST read this plan file first to understand the full scope of work before conducting your review.
This plan contains the complete requirements and implementation details that Claude should be following.

Based on the original plan and @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-2-prompt.md, Claude claims to have completed the work. Please conduct a thorough critical review to verify this.

---
Below is Claude's summary of the work completed:
<!-- CLAUDE's WORK SUMMARY START -->
# Round 2 Summary

## What was implemented

- Made filtered SCML preflight runs non-destructive:
  - `tools/preflight_scml_gate.py` now detects filtered/debug invocations (`--program-id`, `--limit`)
  - filtered runs write to `reports/asterinas_scml/debug-preflight/<label>/`
  - only an unfiltered full-corpus run is allowed to write:
    - `eligible_programs/asterinas_scml.jsonl`
    - `reports/asterinas_scml/scml-rejections.jsonl`
    - `reports/asterinas_scml/preflight-summary.json`
- Added target-neutral capability/gate abstraction in `orchestrator/capability.py`:
  - `CapabilitySource` protocol
  - `SequenceGate` protocol
  - `AsterinasSCMLSource`
  - `AsterinasSCMLGate`
  - moved manifest/profile projection and SCML line-classification logic behind that layer
- Rewired callers to use the new abstraction:
  - `tools/derive_scml_allowed_sequences.py` now imports `load_manifest_index` from `orchestrator.capability`
  - `tools/preflight_scml_gate.py` now uses `AsterinasSCMLSource` + `AsterinasSCMLGate`
- Fixed SCML summary aggregation so `rejected_by_scml` is now a first-class SCML result bucket in summary/signoff aggregation logic:
  - extracted `merge_scml_result_counts()` in `tools/render_summary.py`
  - added regression coverage in `tests/test_scml_reporting.py`
- Tightened `tools/reduce_case.py` semantics for `asterinas_scml`:
  - it no longer defaults to synthetic `controlled_divergence` for that workflow
  - it now requires a real campaign result with `scml_result_bucket=passed_scml_and_diverged`
  - it reruns exact-program SCML preflight for the minimized testcase before carrying forward `scml_preflight_status`
  - it refuses to write an `asterinas_scml` minimized report if `first_divergence_syscall_index` is `None`
- Removed the stale partial workflow artifacts that had been generated from debug/sample runs in formal output locations:
  - removed the clobbered `eligible_programs/asterinas_scml.jsonl`
  - removed stale `preflight-summary.json`, `scml-rejections.jsonl`, `summary/signoff`, `campaign-results`, and the invalid minimized-report artifacts under `reports/asterinas_scml/`
  - left only `reports/asterinas_scml/derivation-summary.json` plus debug-preflight outputs

## Files modified

- `orchestrator/capability.py`
- `tests/test_scml_preflight.py`
- `tests/test_scml_reporting.py`
- `tools/derive_scml_allowed_sequences.py`
- `tools/preflight_scml_gate.py`
- `tools/reduce_case.py`
- `tools/render_summary.py`

## Commit

- `f2b855a` `Protect SCML debug outputs and add capability layer`

## Tests added/passed

- Added:
  - `tests/test_scml_reporting.py`
- Expanded:
  - `tests/test_scml_preflight.py`
- Passed:
  - `python3 -m unittest tests.test_scml_manifest tests.test_scml_derivation tests.test_scml_preflight tests.test_scml_reporting`
  - `python3 -m unittest tests.test_asterinas_pipeline`
  - `python3 -m py_compile orchestrator/capability.py tools/preflight_scml_gate.py tools/reduce_case.py tools/render_summary.py tools/derive_scml_allowed_sequences.py`

## Real verification performed

- Verified filtered preflight now writes only to debug outputs:
  - `python3 tools/preflight_scml_gate.py --workflow asterinas_scml --program-id 505a3d94e57295c2f6814abae501560fe464fb972e0644d21fc0247d3669f48d`
  - confirmed:
    - `reports/asterinas_scml/debug-preflight/program-505a3d94e57295c2/eligible.jsonl`
    - `reports/asterinas_scml/debug-preflight/program-505a3d94e57295c2/scml-rejections.jsonl`
    - `reports/asterinas_scml/debug-preflight/program-505a3d94e57295c2/preflight-summary.json`
  - confirmed formal outputs were **not** recreated:
    - `eligible_programs/asterinas_scml.jsonl` does not exist
    - `reports/asterinas_scml/preflight-summary.json` does not exist
- Confirmed the formal `reports/asterinas_scml/` directory now contains only:
  - `reports/asterinas_scml/derivation-summary.json`
  - plus debug-preflight subdirectories

## Remaining items

- Full-corpus preflight has not yet been rerun after the destructive-debug-output fix, so the formal final eligible corpus and runtime SCML rejection ledger have not been regenerated.
- Because there is currently no persisted `passed_scml_and_diverged` campaign result, `tools/reduce_case.py --workflow asterinas_scml` will now correctly refuse to manufacture a misleading minimized report, but there is not yet a valid replacement report artifact.
- Campaign-scale evidence is still missing:
  - no 100-case smoke slice
  - no 500-case sign-off slice
  - no final `signoff_pass=true`

## Goal Tracker Update Request

### Requested Changes:
- Update Active Task `Implement Linux runtime SCML preflight tool without letting filtered/debug runs clobber the full eligible corpus or rejection artifacts`:
  - note that the destructive subset-output bug is fixed
  - keep the task active until a new full-corpus preflight run regenerates the formal outputs
- Update Active Task `Thread scml_preflight_status and related evidence through scheduler, summaries, and minimized reports with exact-program correctness`:
  - note that `rejected_by_scml` summary aggregation is fixed
  - note that `asterinas_scml` reduce-case now refuses invalid exact-program reports instead of emitting them
  - keep the task active until a valid exact-program minimized report from a real diverged SCML-passed case exists
- Mark original-plan AC-9 progress as started:
  - `CapabilitySource` / `SequenceGate` abstractions now exist in code and are used by derivation/preflight
- Add a Plan Evolution row:
  - filtered SCML preflight runs now write debug outputs instead of formal workflow outputs
  - invalid/stale partial workflow artifacts were removed rather than preserved as if they were trustworthy
- Add to Open Issues:
  - full-corpus preflight must be rerun to regenerate official `eligible_programs/asterinas_scml.jsonl`, `scml-rejections.jsonl`, and `preflight-summary.json`
  - a real `passed_scml_and_diverged` campaign result is still needed before `reduce_case.py --workflow asterinas_scml` can produce a valid minimized report

### Justification:
These updates keep the tracker honest: Round 2 fixed correctness bugs and added the missing abstraction layer, but it intentionally removed misleading partial artifacts instead of pretending the workflow is complete. The tracker should show that progress while keeping the campaign-scale rerun and valid SCML-passed minimized report as active remaining work.
<!-- CLAUDE's WORK SUMMARY  END  -->
---

## Part 1: Implementation Review

- Your task is to conduct a deep critical review, focusing on finding implementation issues and identifying gaps between "plan-design" and actual implementation.
- Relevant top-level guidance documents, phased implementation plans, and other important documentation and implementation references are located under @docs.
- If Claude planned to defer any tasks to future phases in its summary, DO NOT follow its lead. Instead, you should force Claude to complete ALL tasks as planned.
  - Such deferred tasks are considered incomplete work and should be flagged in your review comments, requiring Claude to address them.
  - If Claude planned to defer any tasks, please explore the codebase in-depth and draft a detailed implementation plan. This plan should be included in your review comments for Claude to follow.
  - Your review should be meticulous and skeptical. Look for any discrepancies, missing features, incomplete implementations.
- If Claude does not plan to defer any tasks, but honestly admits that some tasks are still pending (not yet completed), you should also include those pending tasks in your review.
  - Your review should elaborate on those unfinished tasks, explore the codebase, and draft an implementation plan.
  - A good engineering implementation plan should be **singular, directive, and definitive**, rather than discussing multiple possible implementation options.
  - The implementation plan should be **unambiguous**, internally consistent, and coherent from beginning to end, so that **Claude can execute the work accurately and without error**.

## Part 2: Goal Alignment Check (MANDATORY)

Read @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md and verify:

1. **Acceptance Criteria Progress**: For each AC, is progress being made? Are any ACs being ignored?
2. **Forgotten Items**: Are there tasks from the original plan that are not tracked in Active/Completed/Deferred?
3. **Deferred Items**: Are deferrals justified? Do they block any ACs?
4. **Plan Evolution**: If Claude modified the plan, is the justification valid?

Include a brief Goal Alignment Summary in your review:
```
ACs: X/Y addressed | Forgotten items: N | Unjustified deferrals: N
```

## Part 3: ## Goal Tracker Update Requests (YOUR RESPONSIBILITY)

**Important**: Claude cannot directly modify `goal-tracker.md` after Round 0. If Claude's summary contains a "Goal Tracker Update Request" section, YOU must:

1. **Evaluate the request**: Is the change justified? Does it serve the Ultimate Goal?
2. **If approved**: Update @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md yourself with the requested changes:
   - Move tasks between Active/Completed/Deferred sections as appropriate
   - Add entries to "Plan Evolution Log" with round number and justification
   - Add new issues to "Open Issues" if discovered
   - **NEVER modify the IMMUTABLE SECTION** (Ultimate Goal and Acceptance Criteria)
3. **If rejected**: Include in your review why the request was rejected

Common update requests you should handle:
- Task completion: Move from "Active Tasks" to "Completed and Verified"
- New issues: Add to "Open Issues" table
- Plan changes: Add to "Plan Evolution Log" with your assessment
- Deferrals: Only allow with strong justification; add to "Explicitly Deferred"

## Part 4: Output Requirements

- In short, your review comments can include: problems/findings/blockers; claims that don't match reality; implementation plans for deferred work (to be implemented now); implementation plans for unfinished work; goal alignment issues.
- If after your investigation the actual situation does not match what Claude claims to have completed, or there is pending work to be done, output your review comments to @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-2-review-result.md.
- **CRITICAL**: Only output "COMPLETE" as the last line if ALL tasks from the original plan are FULLY completed with no deferrals
  - DEFERRED items are considered INCOMPLETE - do NOT output COMPLETE if any task is deferred
  - UNFINISHED items are considered INCOMPLETE - do NOT output COMPLETE if any task is pending
  - The ONLY condition for COMPLETE is: all original plan tasks are done, all ACs are met, no deferrals or pending work allowed
- The word COMPLETE on the last line will stop Claude.
