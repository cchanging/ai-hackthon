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
