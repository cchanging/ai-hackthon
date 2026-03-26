# Round 1 Summary

## What was implemented

- Split the SCML workflow into two explicit admission stages:
  - `tools/derive_scml_allowed_sequences.py` now writes static candidates to `eligible_programs/asterinas_scml.static.jsonl`.
  - `tools/preflight_scml_gate.py` now performs runtime admission and writes the final `eligible_programs/asterinas_scml.jsonl`.
- Added runtime SCML preflight:
  - builds candidate programs as needed
  - runs Linux-side `strace -yy -f`
  - runs Asterinas `sctrace` in offline mode (via installed binary if present, otherwise `cargo run` from `third_party/asterinas/tools/sctrace`)
  - emits stable structured rejections into `reports/asterinas_scml/scml-rejections.jsonl`
  - emits preflight artifacts under `artifacts/preflight/asterinas_scml/<program_id>/`
- Added structured preflight taxonomy/config in `configs/asterinas_scml_rules.json`:
  - `syscall_not_in_manifest`
  - `unsupported_flag_pattern`
  - `unsupported_struct_pattern`
  - `unsupported_path_pattern`
  - `deferred_category`
  - `scml_parser_gap`
  - `preflight_build_failure`
- Hardened the manifest/profile contract:
  - `compat_specs/asterinas/generation-profile.json` now has `deferred_syscalls` with `reboot`
  - `tools/derive_scml_allowed_sequences.py` now applies profile decisions per syscall and materializes effective `generation_enabled` / `defer_reason`
- Normalized manifest alias values so README prose no longer pollutes consumer-facing fields like `ignored_flags` / `unsupported_codes`.
- Reused the Asterinas backend for `asterinas_scml` cleanly:
  - `tools/prog2c_wrap.py` now decides whether to build the candidate binary from runner profile shape, not a hardcoded workflow name
  - `tools/run_asterinas.py` now accepts any workflow with an `asterinas` config block instead of only literal workflow `asterinas`
- Threaded SCML evidence through execution/reporting:
  - `orchestrator/models.py` `EligibleProgram` now carries preflight status/evidence fields
  - `orchestrator/scheduler.py` now copies `scml_preflight_status`, rejection reasons, preflight evidence paths, and emits `scml_result_bucket`
  - `tools/render_summary.py` now includes SCML gate counts/metrics in summary/signoff
  - `tools/reduce_case.py` now propagates SCML preflight metadata into minimized reports and uses unique run IDs for reduce trials
- Updated `Makefile` to run the SCML workflow in the new order:
  - manifest
  - static derivation
  - build static candidates
  - runtime preflight

## Files modified

- `Makefile`
- `compat_specs/asterinas/generation-profile.json`
- `compat_specs/asterinas/scml-manifest.json`
- `configs/asterinas_scml_rules.json`
- `orchestrator/models.py`
- `orchestrator/scheduler.py`
- `tests/test_scml_derivation.py`
- `tests/test_scml_manifest.py`
- `tests/test_scml_preflight.py`
- `tools/build_scml_manifest.py`
- `tools/derive_scml_allowed_sequences.py`
- `tools/preflight_scml_gate.py`
- `tools/prog2c_wrap.py`
- `tools/reduce_case.py`
- `tools/render_summary.py`
- `tools/run_asterinas.py`

## Commit

- `ece828a` `Add runtime SCML preflight gating`

## Tests added/passed

- Added new unit coverage in `tests/test_scml_preflight.py` for:
  - `lseek(..., SEEK_HOLE)` -> `unsupported_flag_pattern`
  - `renameat2(..., RENAME_EXCHANGE)` -> `unsupported_flag_pattern`
  - `clone3(...)` -> `unsupported_struct_pattern`
  - fallback path-pattern classification
  - target-syscall filtering to suppress startup noise
- Expanded:
  - `tests/test_scml_manifest.py`
  - `tests/test_scml_derivation.py`
- Passed:
  - `python3 -m unittest tests.test_scml_manifest tests.test_scml_derivation tests.test_scml_preflight`
  - `python3 -m unittest tests.test_asterinas_pipeline`

## Real workflow verification performed

- Rebuilt manifest:
  - `python3 tools/build_scml_manifest.py`
- Rebuilt static SCML candidates:
  - `python3 tools/derive_scml_allowed_sequences.py --workflow asterinas_scml`
- Verified runtime rejection on a concrete review example:
  - `python3 tools/preflight_scml_gate.py --workflow asterinas_scml --program-id 505a3d94e57295c2f6814abae501560fe464fb972e0644d21fc0247d3669f48d`
  - result: `reports/asterinas_scml/scml-rejections.jsonl` recorded `unsupported_flag_pattern`
  - evidence file narrowed to the testcase-relevant line:
    - `artifacts/preflight/asterinas_scml/505a3d94e57295c2f6814abae501560fe464fb972e0644d21fc0247d3669f48d/preflight.sctrace.txt`
- Verified runtime pass on a concrete sample:
  - `python3 tools/preflight_scml_gate.py --workflow asterinas_scml --program-id 0064df83d066cf717ee6e6eb040250ba005291ebb1a7c976b48cd1f541f47190`
  - result: final `eligible_programs/asterinas_scml.jsonl` contains a row with `scml_preflight_status=passed`
- Ran the passed sample through Linux vs Asterinas:
  - `python3 orchestrator/scheduler.py --workflow asterinas_scml --campaign smoke --program-id 0064df83d066cf717ee6e6eb040250ba005291ebb1a7c976b48cd1f541f47190 --jobs 1`
  - result: `reports/asterinas_scml/campaign-results.jsonl` shows:
    - `classification=NO_DIFF`
    - `scml_preflight_status=passed`
    - `scml_result_bucket=passed_scml_and_no_diff`
    - candidate status `ok`
- Generated SCML-aware summary/signoff:
  - `python3 tools/prog2c_wrap.py --workflow asterinas_scml --eligible-file eligible_programs/asterinas_scml.jsonl --jobs 1`
  - `python3 tools/render_summary.py --workflow asterinas_scml`
  - outputs now exist:
    - `reports/asterinas_scml/summary.json`
    - `reports/asterinas_scml/summary.md`
    - `reports/asterinas_scml/signoff.md`
    - `reports/asterinas_scml/divergence-index.jsonl`
- A minimized report was produced for a preflight-passed source testcase and now contains SCML metadata:
  - `reports/asterinas_scml/minimized-report.json`
  - `reports/asterinas_scml/minimized-report.md`
  - both include `scml_preflight_status=passed`

## Current artifact state

- `reports/asterinas_scml/summary.json` currently reports:
  - `workflow=asterinas_scml`
  - `total=1`
  - `classification_counts={"NO_DIFF": 1}`
  - `scml_result_counts={"passed_scml_and_no_diff": 1}`
  - `scml_preflight_pass_rate=1.0`
  - `signoff_pass=false`
- `reports/asterinas_scml/signoff.md` exists but is not a passing signoff because only one passed sample has been run.

## Remaining items

- The preflight taxonomy fallback is still heuristic for some unsupported non-flag patterns; only the concrete `flag` and `clone3` struct-style cases are explicitly locked in tests right now.
- `goal-tracker.md` still does not reflect the newly completed Round 1 work because post-Round-0 updates must go through Codex review.
- Signoff thresholds are still unmet because this round only exercised a tiny verification slice (`total=1`).
- The minimized report exists and carries `scml_preflight_status=passed`, but the current workflow still needs broader campaign-scale evidence before the original plan can be considered complete.

## Goal Tracker Update Request

### Requested Changes:
- Mark Active Task `Implement Linux runtime SCML preflight tool and stable rejection taxonomy output` as completed with Round 1 evidence.
- Mark Active Task `Thread scml_preflight_status and related evidence through scheduler, summaries, and minimized reports` as completed with Round 1 evidence.
- Mark Active Task `Harden generation-profile / derivation contract so deferred categories and unspecialized-only filtering stay explicit` as completed with Round 1 evidence.
- Add to Completed and Verified:
  - AC-3: profile now projects syscall-level defer decisions (`reboot`) and static derivation materializes them.
  - AC-5: runtime preflight exists and rejects concrete `SEEK_HOLE` / `SEEK_DATA` cases before candidate execution.
  - AC-6: campaign results, summary/signoff, and minimized report now carry SCML preflight metadata.
- Add a Plan Evolution row noting that the workflow was split into:
  - static candidate derivation (`eligible_programs/asterinas_scml.static.jsonl`)
  - runtime preflight admission (`eligible_programs/asterinas_scml.jsonl`)
- Add to Open Issues:
  - taxonomy fallback for non-flag unsupported patterns is still partly heuristic and may need deeper SCML-aware parsing later.
  - campaign-scale signoff is still pending; current proof only covers a one-sample smoke slice.

### Justification:
These updates reflect work that is now materially implemented and verified in Round 1. Without updating the tracker, it still misrepresents the remaining scope by showing the runtime gate and SCML-aware reporting as merely pending/deferred, even though those components now exist and have been exercised on real `asterinas_scml` samples.
