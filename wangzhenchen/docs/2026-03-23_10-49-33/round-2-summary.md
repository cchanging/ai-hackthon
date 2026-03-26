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
