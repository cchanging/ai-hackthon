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
# Round 6 Review

## Findings

1. Blocking: the exact AC-11 replay command still does not rerun successfully in the current workspace-write environment. [`tools/reduce_case.py`](/home/plucky/FuzzAsterinas/tools/reduce_case.py#L322) now does consume the recorded `candidate_package_dir` / `candidate_package_slot`, and the selected campaign row for program `0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63` points at the correct `asterinas_scml` package. But rerunning `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63` still exits with `asterinas_scml reduce_case requires the selected source testcase to already be a passed_scml_and_diverged case with a valid syscall divergence index`. Directly replaying the seeded source program against that exact package reproduces `candidate status=infra_error`, `comparison.reason="event_count_mismatch"`, `first_divergence_index=0`, and no candidate events, so the active task is still open. The fresh rerun stderr at [stderr.txt](/home/plucky/FuzzAsterinas/artifacts/runs/asterinas_scml/0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63/reduce-0492bbe25ce2-1774431888395099156-candidate/candidate/stderr.txt) shows `fatal: unable to access ... inherit-methods-macro ... 127.0.0.1 port 39537`, and [`gitconfig_lines()`](/home/plucky/FuzzAsterinas/tools/run_asterinas.py#L292) plus [`prepare_docker_gitconfig()`](/home/plucky/FuzzAsterinas/tools/run_asterinas.py#L321) still call `ensure_asterinas_git_mirrors()` on every Docker invocation. So the packaged replay path is still network-dependent and Claude’s “fresh exact AC-11 rerun” claim is false. `test-visible-behavior`

2. High: the formal SCML summary and sign-off artifacts are still being rendered from the wrong rejection ledger. [`tools/derive_scml_allowed_sequences.py`](/home/plucky/FuzzAsterinas/tools/derive_scml_allowed_sequences.py#L104) and [`tools/preflight_scml_gate.py`](/home/plucky/FuzzAsterinas/tools/preflight_scml_gate.py#L68) both own [reports/asterinas_scml/scml-rejections.jsonl](/home/plucky/FuzzAsterinas/reports/asterinas_scml/scml-rejections.jsonl), and the checked-in file currently contains `703` derivation-stage rows with `reasons`, not the `697` runtime preflight rejects reported by [preflight-summary.json](/home/plucky/FuzzAsterinas/reports/asterinas_scml/preflight-summary.json). [`tools/render_summary.py`](/home/plucky/FuzzAsterinas/tools/render_summary.py#L84) treats that top-level file as SCML preflight evidence, so [summary.json](/home/plucky/FuzzAsterinas/reports/asterinas_scml/summary.json) now reports `scml_rejected_count=703` and `rejected_by_scml=703` even though the formal preflight summary says `rejected=697`. That means Claude’s claimed final official report state does not match the checked-in artifacts, and the current sign-off report is still not trustworthy. `test-visible-behavior`

## Open Questions / Assumptions

- I did not rerun the full 1298-row preflight or 500-case campaign. The exact recorded reducer command, the fresh packaged replay, and the checked-in artifact mismatch were already sufficient to falsify the completion claim.
- I infer that the current replay blocker is the unconditional mirror refresh during Docker setup because the failing rerun’s stderr contains the mirror-update error while the selected package metadata and packaged bundle metadata still match exactly.

## Goal Alignment Summary

`ACs: 5/6 addressed | Forgotten items: 1 | Unjustified deferrals: 0`

- Acceptance Criteria Progress: AC-1 through AC-4 remain addressed. AC-5 and AC-6 reporting evidence is currently regressed at the formal artifact boundary because the top-level rejection ledger and `summary.json` no longer reflect runtime preflight results. The original-plan AC-11 active task remains open because the exact recorded `reduce_case.py` command still fails fresh.
- Forgotten items: the tracker did not previously carry any active task for keeping derivation rejects separate from the formal SCML preflight rejection ledger consumed by `summary.json` / `signoff.md`.
- Deferred Items: none.
- Plan Evolution: partially approved. The Round 6 package-provenance threading, baseline-driven derivation default, and generation-summary gating are real changes, but the request to mark the last task complete and remove all open issues is not justified.

## Goal Tracker Update Handling

- Rejected: marking `Make tools/reduce_case.py --workflow asterinas_scml --program-id <real-diverged-id> rerunnable in the current workspace-write environment` as completed.
- Approved in part: I updated [goal-tracker.md](/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md) to record the real Round 6 plan evolution and to remove the obsolete Round 5 issue descriptions that were specifically fixed.
- Added/updated: the active-task note now reflects the actual packaged replay failure mode, and the tracker now carries the new reporting blocker caused by sharing `scml-rejections.jsonl` between derivation and preflight.

## Directive Implementation Plan

1. Make packaged SCML reruns fully self-contained. In [`tools/run_asterinas.py`](/home/plucky/FuzzAsterinas/tools/run_asterinas.py), stop calling `ensure_asterinas_git_mirrors()` from the per-run Docker gitconfig path. Docker/QEMU execution must reuse existing local mirrors and packaged bundles without `git remote update` or any other network/proxy dependency. If mirrors are missing, fail only in explicit bootstrap or cache-priming flows. Add an end-to-end regression around the recorded `reduce_case.py --workflow asterinas_scml --program-id 0492...` path, or an equivalent packaged replay helper, that proves no mirror refresh is attempted and the candidate status remains `ok`.

2. Split derivation-stage and runtime-preflight rejection artifacts. [`tools/derive_scml_allowed_sequences.py`](/home/plucky/FuzzAsterinas/tools/derive_scml_allowed_sequences.py) must write derivation rejects to a distinct artifact path, while [reports/asterinas_scml/scml-rejections.jsonl](/home/plucky/FuzzAsterinas/reports/asterinas_scml/scml-rejections.jsonl) remains exclusively the runtime preflight ledger written by [`tools/preflight_scml_gate.py`](/home/plucky/FuzzAsterinas/tools/preflight_scml_gate.py). Update [`tools/render_summary.py`](/home/plucky/FuzzAsterinas/tools/render_summary.py) and the checked-in artifact chain so formal summary/sign-off metrics consume the runtime ledger only. Add a regression test that rerendered `summary.json` / `signoff.md` match [preflight-summary.json](/home/plucky/FuzzAsterinas/reports/asterinas_scml/preflight-summary.json).

3. After those fixes, rerun:
   - `python3 tools/derive_scml_allowed_sequences.py --workflow asterinas_scml`
   - `python3 tools/preflight_scml_gate.py --workflow asterinas_scml --jobs 32`
   - `python3 tools/render_summary.py --workflow asterinas_scml --campaign full`
   - `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63`

   Refresh [summary.json](/home/plucky/FuzzAsterinas/reports/asterinas_scml/summary.json), [signoff.md](/home/plucky/FuzzAsterinas/reports/asterinas_scml/signoff.md), [scml-rejections.jsonl](/home/plucky/FuzzAsterinas/reports/asterinas_scml/scml-rejections.jsonl), and [minimized-report.json](/home/plucky/FuzzAsterinas/reports/asterinas_scml/minimized-report.json) only if those commands succeed end to end.

## Checklist

- `add-regression-tests`: issue. The new tests cover workflow-scoped package lookup and generation-metric gating, but there is still no regression for the exact recorded reducer command or for `summary.json` / `preflight-summary.json` consistency after the official artifact chain.
- `test-visible-behavior`: issue. The user-visible AC-11 replay command still fails fresh, and the formal summary/sign-off artifacts still publish the wrong SCML rejection counts even though the targeted unit tests pass.

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
2. Write your work summary into @/home/plucky/FuzzAsterinas/.humanize/rlcr/2026-03-23_10-49-33/round-7-summary.md

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
