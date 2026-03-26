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
# Round 5 Review

## Findings

1. Blocking: the exact AC-11 rerun is still not reproducible because [`tools/reduce_case.py`](/home/plucky/FuzzAsterinas/tools/reduce_case.py#L96) replays against a heuristically chosen initramfs package instead of the exact campaign package that produced the selected divergence. [`tools/reduce_case.py`](/home/plucky/FuzzAsterinas/tools/reduce_case.py#L238) threads `find_campaign_package_context()` into reducer replay, and that helper picks the largest/newest manifest hit across all packages rather than a workflow-scoped exact match. For `0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63`, there are two matching package manifests: the real `asterinas_scml` campaign package at [`package-manifest.json`](/home/plucky/FuzzAsterinas/artifacts/asterinas/initramfs-packages/0f6ec6a453dfb0b64bbf28d2c1165f2edb1d6a68c9a7885e251df66035b2e531/package-manifest.json#L112) and a newer `asterinas` package at [`package-manifest.json`](/home/plucky/FuzzAsterinas/artifacts/asterinas/initramfs-packages/ac5afcf1b10b10974e6b26bad7c9b2038911b13d147317ffab34e47bd3ede580/package-manifest.json#L172). I reran the recorded command from [`minimized-report.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/minimized-report.json#L15), and it exited in [`tools/reduce_case.py`](/home/plucky/FuzzAsterinas/tools/reduce_case.py#L323) with `asterinas_scml reduce_case requires the selected source testcase to already be a passed_scml_and_diverged case with a valid syscall divergence index`. The fresh replay reduced to `event_count_mismatch` at event 0, so Claude’s completion claim in [`round-5-summary.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-5-summary.md#L6) is not verified and the last active task remains open.

2. High: Round 5 regressed the default official derivation path. [`configs/asterinas_scml_rules.json`](/home/plucky/FuzzAsterinas/configs/asterinas_scml_rules.json#L41) now makes `eligible_programs/asterinas_scml.generated.jsonl` the default derivation input, and [`Makefile`](/home/plucky/FuzzAsterinas/Makefile#L40) `derive-asterinas-scml` consumes that default. But the checked-in “official” derivation evidence in [`derivation-summary.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/derivation-summary.json#L7) still comes from `eligible_programs/baseline.jsonl`, and the checked-in generated corpus currently contains only 4 rows. That means the formal 1298/601 artifact chain Claude cites is no longer reproducible by the default workflow; it now depends on the manual `--source-eligible-file eligible_programs/baseline.jsonl` override from [`round-5-summary.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-5-summary.md#L31). This is a real workflow regression against the original plan’s stable reverse-filtering path.

3. Medium: the sign-off summary is now internally inconsistent because [`tools/render_summary.py`](/home/plucky/FuzzAsterinas/tools/render_summary.py#L118) blindly imports whatever `generation-summary.json` is present. The checked-in [`generation-summary.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/generation-summary.json#L10) says `profile_enabled_total=1` and `targets_with_candidates=1`, while [`generation-targets-summary.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/generation-targets-summary.json#L12) says there are 164 enabled targets. [`summary.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/summary.json#L20) now exposes the stale `1/1/4` generation metrics as if they were official sign-off facts. The raw campaign metrics still pass, but the report provenance is no longer trustworthy.

## Open Questions / Assumptions

- I did not rerun the full 500-case campaign or a full default `generate_scml_candidates.py` sweep. The existing checked-in artifacts and the exact recorded reducer command were already sufficient to falsify the completion claim.
- I infer that cross-workflow package mis-selection is the immediate cause of the reducer instability because the reducer explicitly picks the newest/largest matching package, and for `0492...` that newer match is from `artifacts/runs/asterinas/...`, not the selected `asterinas_scml` campaign package.

## Goal Alignment Summary

`ACs: 6/6 addressed | Forgotten items: 1 | Unjustified deferrals: 0`

- Acceptance Criteria Progress: AC-1 through AC-6 still show substantial progress and checked-in artifacts, but original-plan AC-11 is still open because the exact rerun command in the minimized report does not reproduce fresh.
- Forgotten items: the tracker did not previously add any task for preserving the official baseline-driven derivation/report path after switching the default derivation source to the generated corpus.
- Deferred Items: none.
- Plan Evolution: partially approved. The temp-dir fix, packaged selector fallback, and generator tooling are real, but the request to mark the remaining active task complete and remove the rerun blocker was not justified.

## Goal Tracker Update Handling

- Rejected: marking `Make tools/reduce_case.py --workflow asterinas_scml --program-id <real-diverged-id> rerunnable...` complete.
- Partially approved: I updated [`goal-tracker.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md) to record the valid Round 5 plan evolution, remove the obsolete temp-root blocker, and replace it with the actual current blockers: exact packaged-context replay and derivation/report provenance regressions.

## Directive Implementation Plan

1. Persist exact packaged-candidate provenance for every SCML campaign row. When scheduler/candidate packaging writes campaign outputs, record the package identifier and slot that actually produced the selected candidate run. In [`tools/reduce_case.py`](/home/plucky/FuzzAsterinas/tools/reduce_case.py), stop calling `find_campaign_package_context()` when that provenance is present, and reject cross-workflow fallback matches instead of guessing. Add a regression test with the same `program_id` present in both `asterinas` and `asterinas_scml` package manifests, and assert that SCML reducer replay uses the SCML package.
2. Restore a reproducible official derivation path. Either set the formal `asterinas_scml` default derivation input back to `eligible_programs/baseline.jsonl`, or split generator-derived admission into a separate workflow/output so [`Makefile`](/home/plucky/FuzzAsterinas/Makefile#L40) still rebuilds the 1298/601 official chain without extra flags. Add a regression test that the default config path and the Make target consume the intended source file.
3. Add provenance checks before summary rendering. Move partial/debug generation summaries out of the formal `reports/asterinas_scml/` namespace or embed enough metadata in `generation-summary.json` for [`tools/render_summary.py`](/home/plucky/FuzzAsterinas/tools/render_summary.py#L118) to ignore stale/partial runs unless they match the official workflow context. Add a regression test proving that a one-target generation summary cannot contaminate the official sign-off summary.
4. After those fixes, rerun:
   - `python3 tools/export_scml_targets.py --workflow asterinas_scml`
   - the official derivation/preflight chain without a manual source override
   - `python3 tools/render_summary.py --workflow asterinas_scml --campaign full`
   - `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63`
   Then refresh [`minimized-report.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/minimized-report.json), [`summary.json`](/home/plucky/FuzzAsterinas/reports/asterinas_scml/summary.json), and the tracker only if the rerun succeeds end to end.

## Checklist

- `add-regression-tests`: issue. The new changes add tests for temp-dir handling and partial generator failure, but there is still no regression test for workflow-scoped package provenance or for stale `generation-summary.json` contaminating the formal summary.
- `test-visible-behavior`: issue. Current tests mostly exercise helpers; they do not cover the user-visible default `derive-asterinas-scml` / `render_summary.py` / exact `reduce_case.py --program-id ...` commands that regressed.

## Verification Performed

- Read [`docs/asterinas-scml-diff-plan.md`](/home/plucky/FuzzAsterinas/docs/asterinas-scml-diff-plan.md), [`round-5-prompt.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-5-prompt.md), [`round-5-summary.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-5-summary.md), and [`goal-tracker.md`](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md).
- Re-ran `python3 -m unittest tests.test_scml_generation`.
- Re-ran `python3 -m unittest tests.test_asterinas_pipeline tests.test_scml_reduce_case tests.test_scml_reporting tests.test_prog2c_wrap tests.test_scml_preflight tests.test_baseline_pipeline`.
- Re-ran `python3 tools/export_scml_targets.py --workflow asterinas_scml`.
- Re-ran `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63` and observed the fresh failure above.
- Inspected [`configs/asterinas_scml_rules.json`](/home/plucky/FuzzAsterinas/configs/asterinas_scml_rules.json), [`Makefile`](/home/plucky/FuzzAsterinas/Makefile), [`tools/render_summary.py`](/home/plucky/FuzzAsterinas/tools/render_summary.py), the checked-in SCML reports under [`reports/asterinas_scml`](/home/plucky/FuzzAsterinas/reports/asterinas_scml), and the conflicting package manifests under [`artifacts/asterinas/initramfs-packages/0f6ec6a453dfb0b64bbf28d2c1165f2edb1d6a68c9a7885e251df66035b2e531/package-manifest.json`](/home/plucky/FuzzAsterinas/artifacts/asterinas/initramfs-packages/0f6ec6a453dfb0b64bbf28d2c1165f2edb1d6a68c9a7885e251df66035b2e531/package-manifest.json) and [`artifacts/asterinas/initramfs-packages/ac5afcf1b10b10974e6b26bad7c9b2038911b13d147317ffab34e47bd3ede580/package-manifest.json`](/home/plucky/FuzzAsterinas/artifacts/asterinas/initramfs-packages/ac5afcf1b10b10974e6b26bad7c9b2038911b13d147317ffab34e47bd3ede580/package-manifest.json).

Status: incomplete.
<!-- CODEX's REVIEW RESULT  END  -->
---

**IMPORTANT**: Codex has found Open Question(s). You must use `AskUserQuestion` to clarify those questions with user first, before proceeding to resolve any other Codex's findings.

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
2. Write your work summary into @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-6-summary.md

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
