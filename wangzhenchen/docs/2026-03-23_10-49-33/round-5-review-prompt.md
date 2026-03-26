# Code Review - Round 5

## Original Implementation Plan

**IMPORTANT**: The original plan that Claude is implementing is located at:
@docs/asterinas-scml-diff-plan.md

You MUST read this plan file first to understand the full scope of work before conducting your review.
This plan contains the complete requirements and implementation details that Claude should be following.

Based on the original plan and @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-5-prompt.md, Claude claims to have completed the work. Please conduct a thorough critical review to verify this.

---
Below is Claude's summary of the work completed:
<!-- CLAUDE's WORK SUMMARY START -->
## Round 5 Summary

- Fixed the Round 4 portability blocker in the Asterinas runner temp path.
  - `tools/run_asterinas.py` no longer hardcodes `$HOME/tmp/fuzzasterinas`; `local_tmp_dir()` now resolves through `orchestrator.common.temp_dir()` / `TMPDIR`.
  - Added regression coverage in `tests/test_asterinas_pipeline.py` to verify `materialize_guest_env_file()` uses the runtime temp root.
- Confirmed fresh SCML reducer reruns now work in the current workspace-write environment.
  - Fresh reruns no longer fail at the old unwritable temp-root step.
  - Re-ran `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63`.
  - Produced a fresh minimized report in:
    - `reports/asterinas_scml/minimized-report.json`
    - `reports/asterinas_scml/minimized-report.md`
    - `reports/asterinas_scml/19c14bc2a915872c2c76b23f67e34669ce46ba2bb34cb14bbe59d466ed4b310a-minimized.syz`
  - Verified the new minimized report carries:
    - `run_command=python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63`
    - `first_divergence_event_index=4`
    - `first_divergence_syscall_index=1`
    - `scml_preflight_status=passed`
- Hardened the packaged guest selector fallback for the new worktree.
  - `tools/run_asterinas_shared.py` now retries raw-selector loading during the same mount-discovery loop instead of waiting until mount retries are exhausted.
  - This fixes the new-worktree `candidate0`/`candidate-triage*` `infra_error` mode where guest boot reached userspace but failed with:
    - `failed to mount ext2 package disk on /ext2; block devices: /dev/vda /dev/vdb`
- Made the new SCML generation pipeline resilient to partial generator failures.
  - Added new tools from the current worktree:
    - `tools/export_scml_targets.py`
    - `tools/generate_scml_candidates.py`
  - `tools/generate_scml_candidates.py` now keeps writing `generated_file`, `generation-summary.json`, and `generation-gaps.jsonl` even when some targets hit `generator_failed`; it no longer aborts the whole pipeline after outputs are already materialized.
  - Also downgraded invalid generated programs (`syzabi_inspect` failures) to per-target `inspect_failed` gaps instead of crashing the whole run.
  - Added regression coverage in `tests/test_scml_generation.py` for both `inspect_failed` handling and non-fatal `generator_failed` runs.
- Reconstructed the official SCML artifact chain after the new worktree had cleared it.
  - Restored official preflight corpus:
    - `python3 tools/derive_scml_allowed_sequences.py --workflow asterinas_scml --source-eligible-file eligible_programs/baseline.jsonl`
    - `python3 tools/preflight_scml_gate.py --workflow asterinas_scml --jobs 32`
  - Result:
    - `eligible_programs/asterinas_scml.static.jsonl`: `1298`
    - `eligible_programs/asterinas_scml.jsonl`: `601`
    - `reports/asterinas_scml/scml-rejections.jsonl`: `697`
    - `reports/asterinas_scml/preflight-summary.json`: `source_total=1298`, `eligible=601`, `rejected=697`
  - Rebuilt official campaign/signoff outputs with the current worktree:
    - `python3 orchestrator/scheduler.py --workflow asterinas_scml --campaign full --limit 500 --jobs 32`
    - `python3 tools/prog2c_wrap.py --workflow asterinas_scml --eligible-file eligible_programs/asterinas_scml.jsonl --jobs 8`
    - `python3 tools/render_summary.py --workflow asterinas_scml --campaign full`
  - Final official results in `reports/asterinas_scml/summary.json`:
    - `signoff_pass=true`
    - `build_success_rate=1.0`
    - `dual_execution_completion_rate=0.996`
    - `trace_generation_success_rate=1.0`
    - `canonicalization_success_rate=1.0`
    - `baseline_invalid_rate=0.004`
    - `scml_result_counts={passed_scml_and_diverged: 100, passed_scml_and_no_diff: 398, passed_scml_but_candidate_failed: 2, rejected_by_scml: 697}`
- Verification:
  - `python3 -m unittest tests.test_scml_generation`
  - `python3 -m unittest tests.test_asterinas_pipeline tests.test_scml_reduce_case tests.test_scml_reporting tests.test_prog2c_wrap tests.test_scml_preflight tests.test_baseline_pipeline`
  - `python3 tools/export_scml_targets.py --workflow asterinas_scml`
  - `python3 tools/generate_scml_candidates.py --workflow asterinas_scml`
  - `python3 tools/derive_scml_allowed_sequences.py --workflow asterinas_scml --source-eligible-file eligible_programs/baseline.jsonl`
  - `python3 tools/preflight_scml_gate.py --workflow asterinas_scml --jobs 32`
  - `python3 orchestrator/scheduler.py --workflow asterinas_scml --campaign full --limit 500 --jobs 32`
  - `python3 tools/prog2c_wrap.py --workflow asterinas_scml --eligible-file eligible_programs/asterinas_scml.jsonl --jobs 8`
  - `python3 tools/render_summary.py --workflow asterinas_scml --campaign full`
  - `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63`

## Goal Tracker Update Request

### Requested Changes:
- Mark the remaining active task `Make tools/reduce_case.py --workflow asterinas_scml --program-id <real-diverged-id> rerunnable in the current workspace-write environment` as completed.
- Add a Round 5 plan-evolution entry:
  - `tools/run_asterinas.py` now routes temporary files through runtime temp-dir configuration instead of a hardcoded home-directory path.
  - Packaged guest selector fallback now retries raw-header discovery during device probing, which fixes the new-worktree packaged-run `infra_error` mode.
  - The new generation pipeline now preserves outputs under partial generator failures instead of aborting after writing summaries/gaps.
- Remove the Round 4 open issue about the hardcoded temp root blocking fresh reducer reruns.

### Justification:
The original remaining blocker from Round 4 is now closed in repository reality, not only in code. Fresh SCML reducer reruns no longer fail on the hardcoded temp root, and a new minimized report was regenerated from the current official campaign using the recorded rerun command. The repository again contains the full official SCML artifact set (`eligible`, preflight summary, campaign results, summary, signoff, divergence index, minimized report), and the official full sign-off report is back to `signoff_pass=true`. That means the last active task in `goal-tracker.md` is complete.
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
- If after your investigation the actual situation does not match what Claude claims to have completed, or there is pending work to be done, output your review comments to @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-5-review-result.md.
- **CRITICAL**: Only output "COMPLETE" as the last line if ALL tasks from the original plan are FULLY completed with no deferrals
  - DEFERRED items are considered INCOMPLETE - do NOT output COMPLETE if any task is deferred
  - UNFINISHED items are considered INCOMPLETE - do NOT output COMPLETE if any task is pending
  - The ONLY condition for COMPLETE is: all original plan tasks are done, all ACs are met, no deferrals or pending work allowed
- The word COMPLETE on the last line will stop Claude.
