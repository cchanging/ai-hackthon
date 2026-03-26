# Round 3 Review

## Findings

1. Blocking: the original plan is still incomplete because original-plan AC-10 and AC-11 are still unmet. `reports/asterinas_scml/signoff.md` and `reports/asterinas_scml/summary.json` show the official 500-case run still has `signoff_pass=false` with `dual_execution_completion_rate=0.792`, and `reports/asterinas_scml/minimized-report.json` is still absent. Claude is honest about this in the summary, but it still means Round 3 cannot be accepted as complete.

2. High: `tools/reduce_case.py` still cannot generate the required rerunnable minimized report from a real `passed_scml_and_diverged` case because its SCML replay path bypasses the packaged candidate runner that the official campaign now depends on. `run_case()` calls `execute_side()` directly at `tools/reduce_case.py:95-126`, while scheduler triage uses `execute_candidate_case_in_package()` through `run_candidate_once_with_package()` at `orchestrator/scheduler.py:70-90` and `orchestrator/scheduler.py:200-214`. I reproduced this with:
   - `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0064df83d066cf717ee6e6eb040250ba005291ebb1a7c976b48cd1f541f47190`
   - Result: `asterinas_scml reduce_case requires the selected source testcase to already be a passed_scml_and_diverged case with a valid syscall divergence index`
   - Fresh rerun artifacts under `artifacts/runs/asterinas_scml/0064df83d066cf717ee6e6eb040250ba005291ebb1a7c976b48cd1f541f47190/reduce-...-candidate/` show `status=infra_error`, and `candidate/stderr.txt` contains `fatal: unable to access 'https://github.com/asterinas/inherit-methods-macro/' ...`

   This means AC-11 is blocked by an implementation gap, not merely by lack of time. Existing reducer tests at `tests/test_scml_reduce_case.py:8-47` only cover row selection and divergence-index invariants; they do not cover the visible reducer replay path (`add-regression-tests`).

3. High: the round-3 packaged bundle cache can silently reuse a stale Asterinas kernel bundle across repo or boot-config changes. `prepare_candidate_initramfs_package()` keys the package only by workflow, preview bytes, template text, and testcase binary digests at `orchestrator/vm_runner.py:241-267`, while `ensure_packaged_docker_bundle()` reuses any existing `.osdk-build.ready` bundle without validating the Asterinas source revision or `kcmd_args` at `tools/run_asterinas.py:830-840`. After a repo update or command-line/config change, future smoke/sign-off reruns can keep using an old kernel bundle without rebuilding, which undermines the trustworthiness of the optimization itself (`design-decisions`).

## Goal Alignment Summary

ACs: 6/6 addressed | Forgotten items: 0 | Unjustified deferrals: 0

- AC-1 through AC-4 remain completed and verified.
- AC-5 is now complete and verified: official preflight outputs exist again, and debug rejection rows point at `artifacts/preflight/asterinas_scml/debug/...`.
- AC-6 has real campaign artifacts now, but the minimized-report leg is still not operational.
- Original-plan AC-9 is implemented in code.
- Original-plan AC-10 and AC-11 remain active and blocking.

## Goal Tracker Update Handling

Claude's update request was approved in part, and I updated `goal-tracker.md` directly.

- Approved:
  - Marked the AC-5 task completed.
  - Recorded the smoke pass, full-run artifact generation, and round-3 throughput changes.
  - Replaced the stale preflight open issues with the current campaign-era blockers.

- Rejected or adjusted:
  - Kept AC-6, AC-10, and AC-11 active.
  - Added the still-open reducer replay gap; the remaining blockers are not purely candidate-side.
  - Added the package-cache invalidation risk introduced by the new bundle reuse path.

## Directive Implementation Plan

1. Make `tools/reduce_case.py` use the packaged candidate execution path whenever `workflow=asterinas_scml`. Refactor `run_case()` so the candidate side goes through `execute_candidate_case_in_package()` or an equivalent helper that prepares a single-case package and keeps `CARGO_NET_OFFLINE=true`. Do not leave the reducer on `execute_side()` for SCML runs.

2. Add regression coverage for the visible reducer workflow, not only unit helpers. The test must assert that SCML reducer replay uses the packaged candidate path and can consume a real `passed_scml_and_diverged` seed without triggering live Git fetches. Keep the existing bucket and invariant tests.

3. Harden packaged bundle cache invalidation. Extend the package or ready-stamp metadata with the Asterinas source revision, effective boot/build inputs, and `kcmd_args`, and force a rebuild whenever any of those inputs change.

4. After the replay path is fixed, run `python3 tools/reduce_case.py --workflow asterinas_scml --program-id <real-passed_scml_and_diverged-id>` and generate `reports/asterinas_scml/minimized-report.json` from that exact case.

5. Keep the 500-case campaign active until the official sign-off thresholds pass. Triage the 100 exit-132 candidate crashes, the 2 git/TLS `infra_error` rows, and the 2 baseline-invalid rows, rerun the full slice, and do not mark the round complete until `reports/asterinas_scml/signoff.md` reports `signoff_pass=true`.

## Verification Performed

- Read `docs/asterinas-scml-diff-plan.md`, `.humanize/rlcr/2026-03-23_10-49-33/round-3-prompt.md`, and `.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md`.
- Verified official artifacts exist: `eligible_programs/asterinas_scml.jsonl`, `reports/asterinas_scml/scml-rejections.jsonl`, `reports/asterinas_scml/preflight-summary.json`, `reports/asterinas_scml/campaign-results.jsonl`, `reports/asterinas_scml/summary.json`, `reports/asterinas_scml/signoff.md`, and `reports/asterinas_scml/divergence-index.jsonl`.
- Verified debug-preflight evidence now points at debug-local artifact roots.
- Reproduced the reducer failure on a real diverged case and inspected the resulting candidate rerun artifacts and stderr.
- Inspected the packaged-run cache code path in `orchestrator/vm_runner.py` and `tools/run_asterinas.py`.
- Re-ran `python3 -m unittest tests.test_prog2c_wrap tests.test_scml_preflight tests.test_scml_reduce_case tests.test_asterinas_pipeline`; that suite hit a sandbox-local tempdir permission error in `test_materialize_guest_env_file_writes_selector_via_debugfs`, so I did not use that run as correctness evidence for Round 3.

Status: incomplete.
