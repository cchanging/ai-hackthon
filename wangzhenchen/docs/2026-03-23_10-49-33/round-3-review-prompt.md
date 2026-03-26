# Code Review - Round 3

## Original Implementation Plan

**IMPORTANT**: The original plan that Claude is implementing is located at:
@docs/asterinas-scml-diff-plan.md

You MUST read this plan file first to understand the full scope of work before conducting your review.
This plan contains the complete requirements and implementation details that Claude should be following.

Based on the original plan and @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-3-prompt.md, Claude claims to have completed the work. Please conduct a thorough critical review to verify this.

---
Below is Claude's summary of the work completed:
<!-- CLAUDE's WORK SUMMARY START -->
## Round 3 Summary

- Completed the non-destructive SCML preflight fix end to end.
  - `tools/preflight_scml_gate.py` now isolates filtered/debug evidence under `artifacts/preflight/asterinas_scml/debug/...` instead of reusing the formal corpus evidence roots.
  - Verified with a real `--program-id` run that emitted debug rejection rows now point at debug-local `scml_preflight_run_root`, `scml_trace_log_path`, and `scml_sctrace_output_path`.
- Tightened SCML build/runtime throughput so the official corpus-scale reruns could complete in this round.
  - `tools/prog2c_wrap.py` now reuses fresh `build-result.json` outputs instead of rerunning `syz-prog2c + gcc` on every preflight/reducer invocation.
  - `orchestrator/scheduler.py` now parallelizes batch `finalize_prepared_case()` work instead of serializing all divergence triage behind one worker.
  - Batch candidate triage now reuses packaged Asterinas bundles via `orchestrator/vm_runner.py` + `tools/run_asterinas.py` package-aware paths, so `candidate-triage*` no longer recompiles the kernel per case.
- Rebuilt the official SCML preflight artifacts from the full static corpus.
  - `python3 tools/build_scml_manifest.py`
  - `python3 tools/derive_scml_allowed_sequences.py --workflow asterinas_scml`
  - `python3 tools/prog2c_wrap.py --workflow asterinas_scml --eligible-file eligible_programs/asterinas_scml.static.jsonl --jobs 8`
  - `python3 tools/preflight_scml_gate.py --workflow asterinas_scml --jobs 32`
  - Result: `1298` source rows -> `601` official eligible rows + `697` official rejection rows.
  - Regenerated:
    - `eligible_programs/asterinas_scml.jsonl`
    - `reports/asterinas_scml/scml-rejections.jsonl`
    - `reports/asterinas_scml/preflight-summary.json`
- Produced official campaign-scale evidence.
  - Smoke:
    - `python3 orchestrator/scheduler.py --workflow asterinas_scml --campaign smoke --limit 100 --jobs 16`
    - Result: `signoff_pass=true`, `100/100` dual execution completion, `99 BUG_LIKELY`, `1 NO_DIFF`.
  - Full:
    - `python3 orchestrator/scheduler.py --workflow asterinas_scml --campaign full --limit 500 --jobs 32`
    - Result: `500` rows generated, but `signoff_pass=false`.
    - `reports/asterinas_scml/summary.json` shows:
      - `dual_execution_completion_rate=0.792`
      - `passed_scml_and_diverged=393`
      - `passed_scml_and_no_diff=3`
      - `passed_scml_but_candidate_failed=104`
      - `classification_counts={BUG_LIKELY: 493, NO_DIFF: 3, BASELINE_INVALID: 2, WEAK_SPEC_OR_ENV_NOISE: 2}`
- Investigated the current full-run blockers.
  - The previous runtime bottleneck was real and fixed in this round:
    - serial batch-finalize triage
    - repeated testcase rebuilds
    - repeated per-triage Asterinas kernel recompiles
  - The remaining full-run failures are now concentrated in candidate behavior rather than pipeline infrastructure:
    - `104` `passed_scml_but_candidate_failed`
    - `100` candidate `crash` results, all with `exit_code=132`
    - `2` `infra_error` results caused by transient git/TLS fetch failures against `https://github.com/asterinas/inventory/`
    - `2` rows with missing candidate-run metadata
- Verification:
  - `python3 -m unittest tests.test_prog2c_wrap tests.test_scml_preflight tests.test_scml_reduce_case tests.test_asterinas_pipeline`
  - `python3 -m unittest tests.test_asterinas_pipeline tests.test_prog2c_wrap tests.test_scml_preflight tests.test_scml_reduce_case`
  - All targeted tests passed after the round-3 code changes.

## Incomplete

- Original-plan AC-10 is still blocked because the official `500`-case sign-off did not pass.
- Original-plan AC-11 is still blocked because I did not generate `reports/asterinas_scml/minimized-report.json` from a real `passed_scml_and_diverged` case in this round.
- I did not mark completion because the candidate failure cluster must be triaged further before the SCML workflow can be signed off.

## Goal Tracker Update Request

### Requested Changes:
- Mark the AC-5 task as completed with evidence:
  - filtered/debug preflight evidence is isolated
  - official full-corpus `eligible_programs/asterinas_scml.jsonl`, `reports/asterinas_scml/scml-rejections.jsonl`, and `reports/asterinas_scml/preflight-summary.json` were regenerated from the `1298`-row static corpus
- Update the AC-6 / original-plan AC-10/AC-11 task notes to record:
  - smoke sign-off now passes at `100` cases
  - official `500`-case campaign artifacts now exist but sign-off fails with `dual_execution_completion_rate=0.792`
  - `393` real `passed_scml_and_diverged` rows now exist in `reports/asterinas_scml/campaign-results.jsonl`
- Add to Plan Evolution:
  - round 3 introduced build-result reuse, parallel batch-finalize triage, and packaged candidate-triage reuse to remove pipeline-induced runtime inflation
- Replace the old “missing campaign-scale proof” open issue with the concrete current blockers:
  - `100` candidate crashes with `exit_code=132` in the official `500`-case run
  - `2` git/TLS-driven candidate infra errors
  - `2` rows with missing candidate-run metadata
  - minimized-report generation from a real `passed_scml_and_diverged` case is still outstanding

### Justification:
These changes keep the tracker aligned with the actual state reached in round 3. The runtime SCML gate and official corpus artifacts are now real and verified, so AC-5 should no longer stay pending. The remaining gap is no longer preflight or reporting plumbing; it is candidate-side behavior under the official `500`-case workload plus the missing final minimized report. Recording that shift explicitly is necessary to keep the next round focused on the true blockers to the ultimate goal.
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
- If after your investigation the actual situation does not match what Claude claims to have completed, or there is pending work to be done, output your review comments to @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-3-review-result.md.
- **CRITICAL**: Only output "COMPLETE" as the last line if ALL tasks from the original plan are FULLY completed with no deferrals
  - DEFERRED items are considered INCOMPLETE - do NOT output COMPLETE if any task is deferred
  - UNFINISHED items are considered INCOMPLETE - do NOT output COMPLETE if any task is pending
  - The ONLY condition for COMPLETE is: all original plan tasks are done, all ACs are met, no deferrals or pending work allowed
- The word COMPLETE on the last line will stop Claude.
