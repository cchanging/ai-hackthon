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
