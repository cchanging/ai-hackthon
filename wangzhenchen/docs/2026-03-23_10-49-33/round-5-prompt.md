Your work is not finished. Read and execute the below with ultrathink.

## Original Implementation Plan

**IMPORTANT**: Before proceeding, review the original plan you are implementing:
@docs/asterinas-scml-diff-plan.md

This plan contains the full scope of work and requirements. Ensure your work aligns with this plan.

---

For all tasks that need to be completed, please use the Task system (TaskCreate, TaskUpdate, TaskList) to track each item in order of importance.
You are strictly prohibited from only addressing the most important issues - you MUST create Tasks for ALL discovered issues and attempt to resolve each one.

---
Below is Codex's review result:
<!-- CODEX's REVIEW RESULT START -->
# Round 4 Review

## Findings

1. Blocking: the repository now contains a real SCML-backed minimized report, but the exact rerun command recorded in that report is still not independently reproducible in the current review environment, so original-plan AC-11 remains open. [`reports/asterinas_scml/minimized-report.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/minimized-report.json#L5) records valid divergence indices and the rerun command, and the old live-Git-fetch failure is gone, but a fresh `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 01ec90666ede845c1b4215e4ad9a094ab568e3c79588984aa5ac895775455eb1` now fails because [`tools/run_asterinas.py`](/home/plucky/FuzzAsterinas/tools/run_asterinas.py#L417) hardcodes `local_tmp_dir()` to `$HOME/tmp/fuzzasterinas` and [`tools/run_asterinas.py`](/home/plucky/FuzzAsterinas/tools/run_asterinas.py#L793) uses that path for `NamedTemporaryFile`. The fresh reducer rerun proves the failure mode in [`stderr.txt`](/home/plucky/FuzzAsterinas/artifacts/runs/asterinas_scml/01ec90666ede845c1b4215e4ad9a094ab568e3c79588984aa5ac895775455eb1/reduce-01ec90666ede-1774410565472368207-candidate-triage1/candidate/stderr.txt#L1). Because the plan requires a rerunnable SCML-driven minimized report, I cannot mark the overall plan complete yet.

## Acceptance Criteria Audit

### 1.1 Acceptance Criteria Status

| AC | Status | Evidence (if MET) | Blocker (if NOT MET) | Justification (if DEFERRED) |
|----|--------|-------------------|---------------------|----------------------------|
| AC-1 | MET | [`compat_specs/asterinas/scml-manifest.json`](/home/plucky/FuzzAsterinas/compat_specs/asterinas/scml-manifest.json), prior verified manifest rebuild/tests recorded in [`goal-tracker.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md#L67) | - | - |
| AC-2 | MET | Manifest schema/tests remain verified in [`goal-tracker.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md#L68) | - | - |
| AC-3 | MET | Profile/derivation verification remains recorded in [`goal-tracker.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md#L69) | - | - |
| AC-4 | MET | Static derivation contract remains verified in [`goal-tracker.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md#L70) | - | - |
| AC-5 | MET | Official eligible/rejection artifacts still exist and remain stable: [`eligible_programs/asterinas_scml.jsonl`](/home/plucky/FuzzAsterinas/eligible_programs/asterinas_scml.jsonl), [`reports/asterinas_scml/scml-rejections.jsonl`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/scml-rejections.jsonl), [`reports/asterinas_scml/preflight-summary.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/preflight-summary.json) | - | - |
| AC-6 | MET | [`reports/asterinas_scml/summary.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/summary.json#L18) now carries all four SCML buckets and passing sign-off, and [`reports/asterinas_scml/minimized-report.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/minimized-report.json#L5) carries exact-program SCML evidence with non-null divergence indices | - | - |

### 1.2 Forgotten Items Detection

- No normalized plan items are missing from `Active`, `Completed and Verified`, or `Explicitly Deferred` after the tracker update.
- The Round 4 summary’s sign-off and minimized-report claims were verified against checked-in artifacts.
- The request to clear all remaining blockers was only partially approved: original-plan AC-11 still has a fresh-rerun portability gap, and that gap is now tracked explicitly in [`goal-tracker.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md#L61) and [`goal-tracker.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md#L78).

### 1.3 Deferred Items Audit

- No items are currently deferred.
- That remains valid; the remaining AC-11 work stays active rather than being deferred.
- No deferral currently contradicts the Ultimate Goal.

### 1.4 Goal Completion Summary

```text
Acceptance Criteria: 6/6 met (0 deferred)
Active Tasks: 1 remaining
Estimated remaining rounds: 1
Critical blockers: fresh SCML reducer reruns still fail in the current workspace-write environment because Asterinas packaged runs write temp files under unwritable $HOME/tmp/fuzzasterinas
```

## Implementation Review

### Verified Round 4 Progress

- The official 500-case campaign now passes sign-off. [`reports/asterinas_scml/summary.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/summary.json#L18) shows `dual_execution_completion_rate=0.996`, `trace_generation_success_rate=1.0`, `canonicalization_success_rate=1.0`, `baseline_invalid_rate=0.004`, and `signoff_pass=true`.
- The SCML result buckets now reflect analyzable crash traces instead of hiding them as generic candidate failures. [`reports/asterinas_scml/summary.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/summary.json#L22) shows `passed_scml_and_diverged=495`, `passed_scml_but_candidate_failed=2`, and `rejected_by_scml=697`.
- The minimized report is no longer synthetic or cross-program. [`reports/asterinas_scml/minimized-report.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/minimized-report.json#L5) ties `program_id=01ec...` to `scml_preflight_status=passed` and valid divergence indices.
- The round-3 reducer replay/cache blockers were genuinely addressed in code: packaged reducer replay is now wired through [`tools/reduce_case.py`](/home/plucky/FuzzAsterinas/tools/reduce_case.py#L129), and bundle reuse metadata validation is implemented in [`tools/run_asterinas.py`](/home/plucky/FuzzAsterinas/tools/run_asterinas.py#L829).

## Goal Tracker Update Handling

- Approved: marked the SCML evidence-threading task complete.
- Approved: marked the abstraction plus smoke/sign-off evidence task complete.
- Approved: added Round 4 plan-evolution entries for packaged reducer replay reuse, bundle metadata validation, crash-row comparison retention, and regenerated official artifacts.
- Approved: removed the round-3 open issues that Round 4 actually fixed.
- Rejected in part: I did not replace all remaining blockers with a clean bill of health, because the fresh rerun portability gap above is real and blocks original-plan AC-11.
- Rejected in part: I updated [`goal-tracker.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md) directly to reflect that state.

## Directive Implementation Plan

1. Replace the hardcoded Asterinas temp root in [`tools/run_asterinas.py`](/home/plucky/FuzzAsterinas/tools/run_asterinas.py#L417) with `orchestrator.common.temp_dir()` / `TMPDIR` or another writable repo-local path that works under the current sandbox.
2. Re-run `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 01ec90666ede845c1b4215e4ad9a094ab568e3c79588984aa5ac895775455eb1` and confirm it freshly reproduces [`reports/asterinas_scml/minimized-report.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/minimized-report.json) instead of relying on the checked-in artifact.
3. Re-run the relevant reducer/Asterinas pipeline tests after that fix, including the previously failing temp-file path behavior.

## Stagnation Check

- Not stagnating. Rounds 1 through 4 show concrete forward progress: destructive preflight was fixed, the capability abstraction landed, official campaign artifacts were regenerated, sign-off now passes, and the reducer no longer falls back to live Git fetches.

## Verification Performed

- Read [`docs/asterinas-scml-diff-plan.md`](/home/plucky/FuzzAsterinas/docs/asterinas-scml-diff-plan.md), [`goal-tracker.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md), and the Round 1 to Round 3 summaries/review results under [`/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33).
- Verified current official artifacts: [`reports/asterinas_scml/summary.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/summary.json), [`reports/asterinas_scml/signoff.md`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/signoff.md), [`reports/asterinas_scml/minimized-report.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/minimized-report.json), [`reports/asterinas_scml/campaign-results.jsonl`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/campaign-results.jsonl).
- Re-ran `python3 -m unittest tests.test_prog2c_wrap tests.test_scml_preflight tests.test_baseline_pipeline tests.test_scml_reporting`.
- Re-ran `python3 -m unittest tests.test_scml_reduce_case`.
- Re-ran `python3 -m unittest tests.test_asterinas_pipeline.AsterinasPipelineTests.test_ensure_packaged_docker_bundle_rebuilds_on_metadata_mismatch tests.test_asterinas_pipeline.AsterinasPipelineTests.test_write_bug_likely_reports_materializes_index_and_testcase_copy tests.test_asterinas_pipeline.AsterinasPipelineTests.test_scheduler_main_writes_summary_signoff_and_failure_reports`.
- Re-ran `python3 tools/render_summary.py --workflow asterinas_scml`.
- Attempted `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 01ec90666ede845c1b4215e4ad9a094ab568e3c79588984aa5ac895775455eb1`; reducer replay reached the packaged candidate path, but the candidate rerun failed on the hardcoded unwritable temp root shown in [`stderr.txt`](/home/plucky/FuzzAsterinas/artifacts/runs/asterinas_scml/01ec90666ede845c1b4215e4ad9a094ab568e3c79588984aa5ac895775455eb1/reduce-01ec90666ede-1774410565472368207-candidate-triage1/candidate/stderr.txt#L1).

Status: incomplete.
<!-- CODEX's REVIEW RESULT  END  -->
---

## Goal Tracker Reference (READ-ONLY after Round 0)

Before starting work, **read** @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md to understand:
- The Ultimate Goal and Acceptance Criteria you're working toward
- Which tasks are Active, Completed, or Deferred
- Any Plan Evolution that has occurred
- Open Issues that need attention

**IMPORTANT**: You CANNOT directly modify goal-tracker.md after Round 0.
If you need to update the Goal Tracker, include a "Goal Tracker Update Request" section in your summary (see below).

### Post-Alignment Check Action Items

This round follows a Full Goal Alignment Check. Pay special attention to:
- **Forgotten Items**: Codex may have identified tasks that were being ignored. Address them.
- **AC Status**: If any Acceptance Criteria were marked NOT MET, prioritize work toward those.
- **Deferred Items**: If any deferrals were flagged as unjustified, un-defer them now.

---

Note: You MUST NOT try to exit by lying, editing loop state files, or executing `cancel-rlcr-loop`.

After completing the work, please:
0. If the `code-simplifier` plugin is installed, use it to review and optimize your code. Invoke via: `/code-simplifier`, `@agent-code-simplifier`, or `@code-simplifier:code-simplifier (agent)`
1. Commit your changes with a descriptive commit message
2. Write your work summary into @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-5-summary.md

**If Goal Tracker needs updates**, include this section in your summary:
```markdown
## Goal Tracker Update Request

### Requested Changes:
- [E.g., "Mark Task X as completed with evidence: tests pass"]
- [E.g., "Add to Open Issues: discovered Y needs addressing"]
- [E.g., "Plan Evolution: changed approach from A to B because..."]
- [E.g., "Defer Task Z because... (impact on AC: none/minimal)"]

### Justification:
[Explain why these changes are needed and how they serve the Ultimate Goal]
```

Codex will review your request and update the Goal Tracker if justified.
