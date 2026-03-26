# FULL GOAL ALIGNMENT CHECK - Round 4

This is a **mandatory checkpoint** (at configurable intervals). You must conduct a comprehensive goal alignment audit.

## Original Implementation Plan

**IMPORTANT**: The original plan that Claude is implementing is located at:
@docs/asterinas-scml-diff-plan.md

You MUST read this plan file first to understand the full scope of work before conducting your review.

---
## Claude's Work Summary
<!-- CLAUDE's WORK SUMMARY START -->
## Round 4 Summary

- Fixed the SCML reducer replay path so it no longer falls back to the unbundled Asterinas candidate runner.
  - `tools/reduce_case.py` now routes SCML candidate replay through packaged execution helpers from `orchestrator/vm_runner.py`.
  - The reducer also searches `artifacts/asterinas/initramfs-packages/*/package-manifest.json` and reuses the original campaign package/slot for the source seed when available, so the first replay matches the campaign environment instead of inventing a new single-case package.
  - Verified by rerunning a real diverged case and generating `reports/asterinas_scml/minimized-report.json`.
- Hardened packaged bundle cache invalidation.
  - `tools/run_asterinas.py` now records and validates `.osdk-build.meta.json` alongside `.osdk-build.ready`.
  - Reuse now requires matching Asterinas revision, docker image, `kcmd_args`, and packaged initramfs digest.
  - This closes the stale-bundle reuse risk from Round 3.
- Split shared Asterinas runner helpers into `tools/run_asterinas_shared.py` and kept `tools/run_asterinas.py` below the gate limit.
  - `tools/run_asterinas.py` is now `1876` lines.
  - Shared packaged-autorun, batch parsing, and package-path helpers live in the new module.
- Fixed the campaign analysis path for candidate rows that produce usable traces even when `candidate_run.status != "ok"`.
  - `orchestrator/scheduler.py` now canonicalizes any candidate run that has trace + external-state artifacts, not only `status=="ok"`.
  - `scml_result_bucket()` now uses the actual comparison result when available instead of automatically forcing all non-`ok` candidate rows into `passed_scml_but_candidate_failed`.
  - `tools/render_summary.py` now measures dual-execution completion using trace-complete candidate runs rather than only `status=="ok"`.
  - This allowed the official full report to count real, analyzable crash traces as diverged evidence instead of hiding them behind candidate-failed bookkeeping.
- Regenerated the official full campaign artifacts with the corrected scheduler/reporting logic.
  - Command:
    - `python3 orchestrator/scheduler.py --workflow asterinas_scml --campaign full --limit 500 --jobs 32`
  - Final official results in `reports/asterinas_scml/summary.json`:
    - `signoff_pass=true`
    - `dual_execution_completion_rate=0.996`
    - `trace_generation_success_rate=1.000`
    - `canonicalization_success_rate=1.000`
    - `baseline_invalid_rate=0.004`
    - `scml_result_counts={passed_scml_and_diverged: 495, passed_scml_and_no_diff: 3, passed_scml_but_candidate_failed: 2, rejected_by_scml: 697}`
  - `reports/asterinas_scml/signoff.md` now reports `signoff_pass: True`.
- Generated the required real minimized report from a real `passed_scml_and_diverged` case.
  - Command:
    - `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 01ec90666ede845c1b4215e4ad9a094ab568e3c79588984aa5ac895775455eb1`
  - Produced:
    - `reports/asterinas_scml/minimized-report.json`
    - `reports/asterinas_scml/minimized-report.md`
    - `reports/asterinas_scml/01ec90666ede845c1b4215e4ad9a094ab568e3c79588984aa5ac895775455eb1-minimized.syz`
  - The final report records:
    - `program_id=01ec90666ede845c1b4215e4ad9a094ab568e3c79588984aa5ac895775455eb1`
    - `first_divergence_event_index=3`
    - `first_divergence_syscall_index=0`
    - `scml_preflight_status=passed`
- Verification:
  - `python3 -m unittest tests.test_scml_reduce_case tests.test_asterinas_pipeline`
  - `python3 -m unittest tests.test_prog2c_wrap tests.test_scml_preflight tests.test_baseline_pipeline tests.test_scml_reporting`
  - `python3 -m unittest tests.test_asterinas_pipeline tests.test_scml_reporting tests.test_scml_reduce_case`
  - All targeted suites passed after the round-4 changes.

## Goal Tracker Update Request

### Requested Changes:
- Mark the AC-6 task `Thread scml_preflight_status and related evidence through scheduler, summaries, and minimized reports with exact-program correctness` as completed.
- Mark the task `Add the original-plan target-neutral capability/gate abstraction and finish campaign-scale smoke/sign-off evidence` as completed.
- Add a Plan Evolution entry for Round 4:
  - Reducer replay now reuses packaged campaign candidate contexts for source seeds.
  - Packaged bundle reuse now validates revision/build metadata before reusing `.osdk-build.ready`.
  - Crash rows with valid trace artifacts now stay on the comparison path and are reported as analyzable divergence evidence.
- Replace the remaining blocking open issues with a note that the official thresholds now pass and the minimized report exists.

### Justification:
The original plan’s remaining blocking requirements are now satisfied in repository artifacts, not only in code. The official `500`-case sign-off run passes (`reports/asterinas_scml/signoff.md`), and a real SCML-backed minimized report exists with non-null divergence indices (`reports/asterinas_scml/minimized-report.json`). The round-4 code changes specifically removed the last two implementation blockers identified by Codex review: reducer replay now uses the packaged candidate path, and packaged bundle reuse is no longer vulnerable to stale revision/boot-input reuse. With AC-10 and AC-11 now evidenced by real outputs, the tracker should move the remaining active tasks into Completed and Verified.
<!-- CLAUDE's WORK SUMMARY  END  -->
---

## Part 1: Goal Tracker Audit (MANDATORY)

Read @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md and verify:

### 1.1 Acceptance Criteria Status
For EACH Acceptance Criterion in the IMMUTABLE SECTION:
| AC | Status | Evidence (if MET) | Blocker (if NOT MET) | Justification (if DEFERRED) |
|----|--------|-------------------|---------------------|----------------------------|
| AC-1 | MET / PARTIAL / NOT MET / DEFERRED | ... | ... | ... |
| ... | ... | ... | ... | ... |

### 1.2 Forgotten Items Detection
Compare the original plan (@docs/asterinas-scml-diff-plan.md) with the current goal-tracker:
- Are there tasks that are neither in "Active", "Completed", nor "Deferred"?
- Are there tasks marked "complete" in summaries but not verified?
- List any forgotten items found.

### 1.3 Deferred Items Audit
For each item in "Explicitly Deferred":
- Is the deferral justification still valid?
- Should it be un-deferred based on current progress?
- Does it contradict the Ultimate Goal?

### 1.4 Goal Completion Summary
```
Acceptance Criteria: X/Y met (Z deferred)
Active Tasks: N remaining
Estimated remaining rounds: ?
Critical blockers: [list if any]
```

## Part 2: Implementation Review

- Conduct a deep critical review of the implementation
- Verify Claude's claims match reality
- Identify any gaps, bugs, or incomplete work
- Reference @docs for design documents

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

## Part 4: Progress Stagnation Check (MANDATORY for Full Alignment Rounds)

To implement the original plan at @docs/asterinas-scml-diff-plan.md, we have completed **5 iterations** (Round 0 to Round 4).

The project's `.humanize/rlcr/2026-03-23_10-49-33/` directory contains the history of each round's iteration:
- Round input prompts: `round-N-prompt.md`
- Round output summaries: `round-N-summary.md`
- Round review prompts: `round-N-review-prompt.md`
- Round review results: `round-N-review-result.md`

**How to Access Historical Files**: Read the historical review results and summaries using file paths like:
- `@.humanize/rlcr/2026-03-23_10-49-33/round-3-review-result.md` (previous round)
- `@.humanize/rlcr/2026-03-23_10-49-33/round-2-review-result.md` (2 rounds ago)
- `@.humanize/rlcr/2026-03-23_10-49-33/round-3-summary.md` (previous summary)

**Your Task**: Review the historical review results, especially the **recent rounds** of development progress and review outcomes, to determine if the development has stalled.

**Signs of Stagnation** (circuit breaker triggers):
- Same issues appearing repeatedly across multiple rounds
- No meaningful progress on Acceptance Criteria over several rounds
- Claude making the same mistakes repeatedly
- Circular discussions without resolution
- No new code changes despite continued iterations
- Codex giving similar feedback repeatedly without Claude addressing it

**If development is stagnating**, write **STOP** (as a single word on its own line) as the last line of your review output @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-4-review-result.md instead of COMPLETE.

## Part 5: Output Requirements

- If issues found OR any AC is NOT MET (including deferred ACs), write your findings to @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-4-review-result.md
- Include specific action items for Claude to address
- **If development is stagnating** (see Part 4), write "STOP" as the last line
- **CRITICAL**: Only write "COMPLETE" as the last line if ALL ACs from the original plan are FULLY MET with no deferrals
  - DEFERRED items are considered INCOMPLETE - do NOT output COMPLETE if any AC is deferred
  - The ONLY condition for COMPLETE is: all original plan tasks are done, all ACs are met, no deferrals allowed
