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
# Round 2 Review

## Findings

1. Blocking: the original plan is still incomplete because the required formal SCML workflow artifacts have not been regenerated after Round 2 removed the misleading partial ones. The repository currently has no official `eligible_programs/asterinas_scml.jsonl`, no `reports/asterinas_scml/scml-rejections.jsonl`, no `reports/asterinas_scml/preflight-summary.json`, no `reports/asterinas_scml/campaign-results.jsonl`, no `reports/asterinas_scml/summary.json`, no `reports/asterinas_scml/signoff.md`, no `reports/asterinas_scml/divergence-index.jsonl`, and no `reports/asterinas_scml/minimized-report.json`. Only `reports/asterinas_scml/derivation-summary.json` plus debug-preflight outputs remain. That means AC-5 and AC-6 are not verified, and original-plan AC-10/AC-11 are still unmet. The Round 2 summary honestly admits this, so this is incomplete work rather than a false claim, but it still blocks completion.

2. High: filtered/debug preflight is still destructive with respect to the per-program evidence paths that the formal corpus would rely on. `tools/preflight_scml_gate.py` now redirects only the top-level ledgers for filtered runs (`output_targets()` at `tools/preflight_scml_gate.py:64-76`), but `run_preflight()` still always writes the strace/sctrace evidence into `artifacts/preflight/asterinas_scml/<program_id>` (`tools/preflight_scml_gate.py:87-94`). The persisted debug rejection row proves this: `reports/asterinas_scml/debug-preflight/program-505a3d94e57295c2/scml-rejections.jsonl` points at `/home/plucky/FuzzAsterinas/artifacts/preflight/asterinas_scml/505a3d94e57295c2f6814abae501560fe464fb972e0644d21fc0247d3669f48d/...`. Once the official full-corpus outputs are regenerated, a later `--program-id` debug rerun will still overwrite the formal evidence that those rows reference. This is still an AC-5 stability bug.

3. High: `tools/reduce_case.py` does not actually require a real `scml_result_bucket=passed_scml_and_diverged` case when `--program-id` is supplied, despite Claude's summary claiming that it does. In `seed_program()` the explicit-program path only filters `campaign-results.jsonl` by `program_id` (`tools/reduce_case.py:98-99`) and then accepts the first SCML-passed row without checking its bucket (`tools/reduce_case.py:111-119`). The diverged-bucket requirement is enforced only in the no-`--program-id` fallback (`tools/reduce_case.py:100-105`). This leaves the exact-program path able to target `passed_scml_and_no_diff` or `passed_scml_but_candidate_failed` rows, which is still inconsistent with AC-11 and with the Round 2 summary.

4. High: the reducer still validates SCML exact-program correctness only after the unconstrained greedy reduction has already finished, and it does not roll back to the last valid SCML-passed divergent testcase. `greedy_reduce()` minimizes solely on the condition that the current trial still diverges (`tools/reduce_case.py:179-203`), while the SCML preflight check happens only once at the end (`tools/reduce_case.py:213-216`). If reduction crosses outside the SCML-supported boundary, the tool just exits instead of preserving the last testcase that both diverged and still passed exact-program SCML preflight. That is still weaker than the original plan's AC-11 expectation for a rerunnable SCML-driven minimized report and is likely to surface once a real diverged campaign result exists.

## Goal Alignment Summary

ACs: 6/6 addressed | Forgotten items: 0 | Unjustified deferrals: 0

- AC-1 through AC-4 remain completed and verified.
- AC-5 has meaningful progress: the preflight gate exists, and top-level filtered/debug ledgers no longer clobber the official corpus files. It is still incomplete because the official full-corpus outputs were removed and not regenerated, and filtered runs still reuse the shared per-program evidence directory.
- AC-6 has meaningful progress: the summary path now counts `rejected_by_scml`, and the reducer no longer fabricates the previous synthetic minimized report. It is still incomplete because there is no official SCML-aware summary/signoff/divergence/minimized artifact set, and the reducer's exact-program contract is still too weak.
- Original-plan AC-9 now has real progress: `CapabilitySource` / `SequenceGate` exist in code and are used by derivation/preflight.
- Original-plan AC-10 and AC-11 remain active and blocking because the required 100-case smoke, 500-case sign-off, and valid rerunnable minimized report from a real SCML-passed diverged case do not exist.

## Goal Tracker Update Handling

Claude's requested tracker update was approved in part, and I updated `goal-tracker.md` directly.

- Approved:
  - Recorded the new debug-ledger isolation behavior and the removal of misleading partial formal outputs in the Plan Evolution Log.
  - Recorded that the capability/gate abstraction now exists and advances original-plan AC-9.
  - Kept the AC-5 / AC-6 / AC-9-11 tasks active rather than marking them complete.
  - Updated the active-task notes to reflect the verified progress in summary aggregation and the abstraction layer.

- Adjusted beyond Claude's request:
  - Replaced stale open issues that are no longer accurate (`rejected_by_scml` reporting gap and missing capability abstraction).
  - Added the still-open evidence-path clobber issue in filtered/debug preflight.
  - Added the still-open `reduce_case.py --program-id` contract issue and the missing rollback/invariant issue under the reducer task notes.

## Directive Implementation Plan

1. Finish the non-destructive preflight fix completely. Keep the current debug ledger split, but also isolate filtered/debug evidence roots so `run_preflight()` writes `preflight.strace.log`, `preflight.sctrace.txt`, and related files under a debug-specific directory when `--program-id` or `--limit` is used. Update the emitted debug JSONL rows to point at those debug-local evidence paths. Add regression coverage proving that filtered runs do not reuse `artifacts/preflight/asterinas_scml/<program_id>` and therefore cannot overwrite the official evidence referenced by full-corpus outputs.

2. Tighten `tools/reduce_case.py` to enforce the exact-program contract instead of only approximating it. When `--workflow asterinas_scml` is used, require `scml_result_bucket=passed_scml_and_diverged` even when `--program-id` is supplied. Then change reduction so both "still diverges" and "still passes exact-program SCML preflight" are treated as invariants during minimization; if a trial breaks either invariant, reject that trial and retain the last valid testcase instead of exiting only after the whole reduction finishes. Add focused tests for both behaviors.

3. Regenerate the formal SCML workflow artifacts end to end. Rebuild or confirm the manifest/profile, rerun `tools/derive_scml_allowed_sequences.py`, rerun an unfiltered full-corpus `tools/preflight_scml_gate.py`, and restore the official `eligible_programs/asterinas_scml.jsonl`, `reports/asterinas_scml/scml-rejections.jsonl`, and `reports/asterinas_scml/preflight-summary.json`. Only after those official preflight outputs exist should the campaign phase continue.

4. Produce the missing campaign-scale evidence from the official preflight-passed corpus. Run the 100-case smoke slice first, then the 500-case sign-off slice if smoke passes. Regenerate `reports/asterinas_scml/campaign-results.jsonl`, `reports/asterinas_scml/summary.json`, `reports/asterinas_scml/summary.md`, `reports/asterinas_scml/signoff.md`, and `reports/asterinas_scml/divergence-index.jsonl`, and do not claim completion unless the sign-off thresholds pass.

5. Create one real SCML-driven minimized report from a preflight-passed diverged case after the sign-off artifacts exist. The report must be based on a real `passed_scml_and_diverged` campaign result, keep `scml_preflight_status=passed` for the minimized testcase itself, and record non-null `first_divergence_event_index` plus `first_divergence_syscall_index` for that same testcase.

## Verification Performed

- Read `docs/asterinas-scml-diff-plan.md`, `round-2-prompt.md`, and `goal-tracker.md`.
- Inspected the Round 2 implementation in:
  - `orchestrator/capability.py`
  - `tools/preflight_scml_gate.py`
  - `tools/render_summary.py`
  - `tools/reduce_case.py`
  - `tools/derive_scml_allowed_sequences.py`
  - `tests/test_scml_preflight.py`
  - `tests/test_scml_reporting.py`
- Verified the current artifact state:
  - `reports/asterinas_scml/` contains only `derivation-summary.json` plus debug-preflight outputs.
  - `eligible_programs/asterinas_scml.jsonl` does not exist.
  - `reports/asterinas_scml/campaign-results.jsonl` does not exist.
  - `reports/asterinas_scml/minimized-report.json` does not exist.
- Verified the shared-evidence-path issue by inspecting `reports/asterinas_scml/debug-preflight/program-505a3d94e57295c2/scml-rejections.jsonl`, which still points at `artifacts/preflight/asterinas_scml/<program_id>/...`.
- Ran:
  - `python3 -m unittest tests.test_scml_manifest tests.test_scml_derivation tests.test_scml_preflight tests.test_scml_reporting tests.test_asterinas_pipeline`

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

---

Note: You MUST NOT try to exit by lying, editing loop state files, or executing `cancel-rlcr-loop`.

After completing the work, please:
0. If the `code-simplifier` plugin is installed, use it to review and optimize your code. Invoke via: `/code-simplifier`, `@agent-code-simplifier`, or `@code-simplifier:code-simplifier (agent)`
1. Commit your changes with a descriptive commit message
2. Write your work summary into @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-3-summary.md

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
