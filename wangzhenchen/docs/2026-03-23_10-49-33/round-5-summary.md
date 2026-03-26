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
