# Code Review - Round 6

## Original Implementation Plan

**IMPORTANT**: The original plan that Claude is implementing is located at:
@docs/asterinas-scml-diff-plan.md

You MUST read this plan file first to understand the full scope of work before conducting your review.
This plan contains the complete requirements and implementation details that Claude should be following.

Based on the original plan and @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-6-prompt.md, Claude claims to have completed the work. Please conduct a thorough critical review to verify this.

---
Below is Claude's summary of the work completed:
<!-- CLAUDE's WORK SUMMARY START -->
## Round 6 Summary

- Fixed exact package provenance for SCML campaign rows.
  - `orchestrator/vm_runner.py` now writes `workflow` into each packaged initramfs manifest.
  - `orchestrator/scheduler.py` now records exact `candidate_package_dir`, `candidate_package_slot`, and `candidate_package_workflow` on each SCML campaign result.
  - `tools/reduce_case.py` now prefers campaign-row package provenance directly and only falls back to manifest lookup when provenance is absent. Fallback lookup is now workflow-scoped instead of scanning every package and guessing by size/newness.
  - Added regression coverage in `tests/test_scml_reduce_case.py` for:
    - using campaign-row package provenance without fallback lookup
    - workflow-scoped fallback package selection
- Restored a reproducible official baseline-driven derivation path.
  - `configs/asterinas_scml_rules.json` now points the formal derivation source back to `eligible_programs/baseline.jsonl`.
  - The generated-candidate path remains available via:
    - `eligible_programs/asterinas_scml.targets.jsonl`
    - `eligible_programs/asterinas_scml.generated.jsonl`
    - `reports/asterinas_scml/generation-summary.json`
    - `reports/asterinas_scml/generation-gaps.jsonl`
  - The official reverse-filtering chain again rebuilds:
    - `eligible_programs/asterinas_scml.static.jsonl` with `1298` rows
    - `eligible_programs/asterinas_scml.jsonl` with `601` rows
    - `reports/asterinas_scml/preflight-summary.json` with `eligible=601`, `rejected=697`
- Prevented generation-summary contamination of formal sign-off reports.
  - `tools/render_summary.py` now only imports generation metrics when the configured official derivation source is the generated corpus file.
  - This removes the stale `profile_enabled_total=1`, `targets_with_candidates=1`, `generation_candidate_count=4` pollution from official `summary.json` / `signoff.md` in the baseline-driven path.
  - Added regression coverage in `tests/test_scml_reporting.py` to prove baseline-driven reports ignore generation metrics.
- Kept the new generation pipeline usable under partial failures.
  - `tools/generate_scml_candidates.py` now preserves `generated_file`, `generation-summary.json`, and `generation-gaps.jsonl` even when some targets fail `syzabi_generate` or `syzabi_inspect`.
  - Added `tests/test_scml_generation.py` integration coverage for non-fatal `generator_failed` handling.
- Revalidated the fresh exact AC-11 rerun after provenance fixes.
  - Re-ran:
    - `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63`
  - The command completed successfully with the current workspace and reused the exact SCML campaign package provenance instead of guessing from cross-workflow manifests.
  - The current minimized report remains fresh and valid:
    - `reports/asterinas_scml/minimized-report.json`
    - `program_id=19c14bc2a915872c2c76b23f67e34669ce46ba2bb34cb14bbe59d466ed4b310a`
    - `run_command=python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63`
    - `first_divergence_event_index=4`
    - `first_divergence_syscall_index=1`
    - `scml_preflight_status=passed`
- Re-ran the official artifact chain required by the review.
  - `python3 tools/export_scml_targets.py --workflow asterinas_scml`
  - `python3 tools/generate_scml_candidates.py --workflow asterinas_scml`
  - `python3 tools/derive_scml_allowed_sequences.py --workflow asterinas_scml`
  - `python3 tools/preflight_scml_gate.py --workflow asterinas_scml --jobs 32`
  - `python3 orchestrator/scheduler.py --workflow asterinas_scml --campaign full --limit 500 --jobs 32`
  - `python3 tools/prog2c_wrap.py --workflow asterinas_scml --eligible-file eligible_programs/asterinas_scml.jsonl --jobs 8`
  - `python3 tools/render_summary.py --workflow asterinas_scml --campaign full`
- Final official report state:
  - `reports/asterinas_scml/summary.json`
    - `signoff_pass=true`
    - `build_success_rate=1.0`
    - `dual_execution_completion_rate=0.996`
    - `trace_generation_success_rate=1.0`
    - `canonicalization_success_rate=1.0`
    - `baseline_invalid_rate=0.004`
    - `passed_scml_and_diverged=100`
    - `passed_scml_and_no_diff=398`
    - `passed_scml_but_candidate_failed=2`
  - `reports/asterinas_scml/signoff.md`
    - `signoff_pass: True`
- Verification:
  - `python3 -m unittest tests.test_scml_generation tests.test_asterinas_pipeline tests.test_scml_reduce_case tests.test_scml_reporting tests.test_prog2c_wrap tests.test_scml_preflight tests.test_baseline_pipeline`
  - All targeted suites passed.

## Goal Tracker Update Request

### Requested Changes:
- Mark the remaining active task `Make tools/reduce_case.py --workflow asterinas_scml --program-id <real-diverged-id> rerunnable in the current workspace-write environment` as completed.
- Add a Round 6 plan-evolution entry:
  - exact candidate package provenance is now recorded in campaign rows and consumed by `reduce_case.py`
  - the official derivation path is restored to the baseline-driven source
  - formal sign-off rendering now ignores generation metrics unless generated candidates are the configured official derivation input
- Remove both remaining open issues:
  - cross-workflow package mis-selection in reducer replay
  - generated-corpus/generation-summary contamination of formal sign-off reports

### Justification:
The last outstanding review blocker is now closed in fresh execution, not only in code. The exact command `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63` completes successfully after package provenance is threaded from campaign results into reducer replay. The official baseline-driven derivation/preflight/sign-off chain is again reproducible without a manual source override, and the formal `summary.json` / `signoff.md` no longer import stale generation metrics. With fresh rerun success plus passing official sign-off artifacts in place, the final active task in the tracker should move to Completed and Verified.
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
- If after your investigation the actual situation does not match what Claude claims to have completed, or there is pending work to be done, output your review comments to @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-6-review-result.md.
- **CRITICAL**: Only output "COMPLETE" as the last line if ALL tasks from the original plan are FULLY completed with no deferrals
  - DEFERRED items are considered INCOMPLETE - do NOT output COMPLETE if any task is deferred
  - UNFINISHED items are considered INCOMPLETE - do NOT output COMPLETE if any task is pending
  - The ONLY condition for COMPLETE is: all original plan tasks are done, all ACs are met, no deferrals or pending work allowed
- The word COMPLETE on the last line will stop Claude.
