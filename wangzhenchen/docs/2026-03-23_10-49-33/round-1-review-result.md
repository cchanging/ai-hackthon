# Round 1 Review

## Findings

1. Blocking: `tools/preflight_scml_gate.py` makes filtered verification runs destructive, so the corpus-scale artifacts Claude cites are no longer trustworthy. `selected_entries()` narrows the source rows for `--program-id` / `--limit` in `tools/preflight_scml_gate.py:156-163`, and `main()` then unconditionally rewrites `eligible_programs/asterinas_scml.jsonl`, `reports/asterinas_scml/scml-rejections.jsonl`, and `reports/asterinas_scml/preflight-summary.json` from just that subset in `tools/preflight_scml_gate.py:283-312`. The current artifacts show the fallout:
   - `eligible_programs/asterinas_scml.static.jsonl` has 915 rows, but `eligible_programs/asterinas_scml.jsonl` has only 1 row.
   - `reports/asterinas_scml/preflight-summary.json:2-6` records `eligible=1`, `rejected=0`, `source_total=1`.
   - `reports/asterinas_scml/summary.json:17-24` therefore reports `eligible_program_count=1`, `scml_rejected_count=0`, `total=1`.
   This means the ad hoc verification commands in the summary clobbered the final eligible corpus and rejection ledger. AC-5 requires stable rejection evidence and a real final eligible corpus; AC-6 then builds on those outputs, so both remain incomplete.

2. Blocking: the minimized report is not a valid "SCML-passed rerunnable report". `tools/reduce_case.py` still defaults to `controlled_divergence` (`tools/reduce_case.py:20-24`), injects synthetic candidate divergence on every run (`tools/reduce_case.py:27-35` and `tools/reduce_case.py:63-69`), and then stamps the reduced testcase with the original source testcase's SCML metadata (`tools/reduce_case.py:87-99` and `tools/reduce_case.py:129-156`). The generated report proves the mismatch:
   - `reports/asterinas_scml/minimized-report.json:10-11` says the original testcase is `0064df...`, but the minimized report `program_id` is `5d9176...`.
   - `reports/asterinas_scml/minimized-report.json:16-18` still points at the preflight artifacts for `0064df...`, not for `5d9176...`.
   - `reports/asterinas_scml/minimized-report.json:5-6` records `first_divergence_event_index=3` but `first_divergence_syscall_index=null`.
   That violates the original plan's AC-11 on two fronts: the report does not correspond to a testcase that itself passed SCML preflight, and the divergence indices are not both valid. Because the divergence is synthetic rather than discovered from the SCML workflow, this also does not satisfy the requirement for a real SCML-driven differential report.

3. High: summary/sign-off still cannot distinguish all required SCML result buckets. `tools/render_summary.py:20-26` builds `scml_result_counts` only from `campaign-results.jsonl`, while `tools/render_summary.py:29-62` reads `scml-rejections.jsonl` only as a separate count. Since the scheduler consumes only the final eligible corpus, `rejected_by_scml` cases never enter `campaign-results.jsonl`; `orchestrator/scheduler.py:80-94` defines the bucket, but the current pipeline never materializes it in summary output. The current `reports/asterinas_scml/summary.json:20-22` contains only `passed_scml_and_no_diff`, so AC-6 and original-plan AC-7 are still not met.

4. High: the original plan's target-neutral capability/gate abstraction is still missing. The plan explicitly requires reusable `CapabilitySource` / `SequenceGate` style interfaces (`docs/asterinas-scml-diff-plan.md:158-170` and `docs/asterinas-scml-diff-plan.md:428-432`), but a repo-wide search shows those names only in the plan, not in implementation code. Round 1 added more Asterinas-specific scripts, but it still did not land the abstraction the plan called out as mandatory future-proofing. Because the user asked that no original-plan work be deferred, this remains incomplete work, not a follow-up nice-to-have.

5. High: the workflow is nowhere near the required smoke/sign-off completion criteria yet, so the round cannot be treated as plan-complete. `reports/asterinas_scml/summary.json:23-24` shows `signoff_pass=false` and `total=1`, while the plan requires 100 SCML-passed smoke samples before sign-off and 500 for final completion. This is not just "more evidence later"; original-plan AC-10 is part of the requested scope, so it must remain active until the required campaign-scale run exists.

## Goal Alignment Summary

ACs: 4/6 addressed | Forgotten items: 3 | Unjustified deferrals: 2

- AC-1 through AC-4 have real progress, and AC-1/AC-2/AC-3/AC-4 are now verified.
- AC-5 is only partially addressed because the preflight tool exists but its filtered mode corrupts the final workflow artifacts.
- AC-6 is only partially addressed because the summary model still omits `rejected_by_scml`, and the minimized report is not valid for the testcase it claims.
- Forgotten original-plan items in Claude's tracker were the target-neutral capability/gate abstraction, the campaign-scale smoke/sign-off requirement, and the requirement for a rerunnable SCML-passed minimized report.
- The Round 0 tracker deferrals for AC-5 and AC-6 were not justified once Claude claimed Round 1 completion. I corrected the tracker by removing those deferrals, recording the verified progress, and adding the new blocking issues instead.

## Goal Tracker Update Handling

Claude's requested tracker update was only partially approved.

- Approved:
  - Verified the Round 0 manifest/derivation items.
  - Marked the syscall-level profile projection work as completed for AC-3.
  - Added a Round 1 plan-evolution entry for the static-derivation plus runtime-preflight split.
  - Added new open issues for the destructive preflight behavior, missing `rejected_by_scml` reporting, invalid minimized report semantics, missing capability abstraction, and absent smoke/sign-off evidence.
- Rejected:
  - Marking `Implement Linux runtime SCML preflight tool and stable rejection taxonomy output` as completed.
  - Marking `Thread scml_preflight_status and related evidence through scheduler, summaries, and minimized reports` as completed.
  - Marking AC-5 and AC-6 as completed and verified.
  Those claims do not hold against the current code and artifacts for the reasons above.

## Directive Implementation Plan

1. Make `tools/preflight_scml_gate.py` non-destructive. Full-corpus runs may continue to write `eligible_programs/asterinas_scml.jsonl`, `scml-rejections.jsonl`, and `preflight-summary.json`, but filtered/debug invocations must write isolated debug outputs or an explicitly separate destination. Add regression coverage proving that `--program-id` and `--limit` do not clobber the full eligible corpus or rejection ledger.

2. Rebuild the SCML reporting model so it accounts for all four required buckets. Keep `campaign-results.jsonl` for preflight-passed executions, but merge preflight rejection counts into `tools/render_summary.py` so `summary.json`, `summary.md`, and `signoff.md` can explicitly report `rejected_by_scml`, `passed_scml_but_candidate_failed`, `passed_scml_and_no_diff`, and `passed_scml_and_diverged`. Add tests for the summary aggregation.

3. Replace the current minimized-report shortcut with an exact-program workflow. Reduce either a real campaign result selected by program ID, or rerun SCML preflight on the exact minimized testcase before carrying forward `scml_preflight_status=passed`. Stop copying SCML evidence from a different source program ID. For SCML workflow reports, require both `first_divergence_event_index` and `first_divergence_syscall_index` to be non-null and derived from the same testcase whose evidence paths are recorded.

4. Implement the target-neutral capability layer that the plan requires. Introduce reusable capability-source and sequence-gate interfaces/modules, route manifest loading and preflight through the Asterinas backend implementation, and keep all backend-specific wiring behind that layer. Do not add a second target yet; only remove the plan gap by making the Asterinas backend conform to the abstraction.

5. After the above fixes land, rerun the workflow end to end at the required scale: rebuild the manifest, rerun static derivation, rerun full-corpus preflight, rebuild the final eligible corpus, execute the 100-case smoke slice, then the 500-case sign-off slice, regenerate summary/sign-off/divergence artifacts, and produce one real rerunnable minimized report from a preflight-passed testcase. Do not claim completion until those artifacts exist and the sign-off thresholds pass.

## Verification Performed

- `python3 -m unittest tests.test_scml_manifest tests.test_scml_derivation tests.test_scml_preflight`
- `python3 -m unittest tests.test_asterinas_pipeline`
- Verified the current artifact mismatch between:
  - `eligible_programs/asterinas_scml.static.jsonl` (915 rows)
  - `eligible_programs/asterinas_scml.jsonl` (1 row)
  - `reports/asterinas_scml/preflight-summary.json`
  - `reports/asterinas_scml/summary.json`
- Verified the minimized-report/program-ID mismatch in `reports/asterinas_scml/minimized-report.json`

Status: incomplete.
