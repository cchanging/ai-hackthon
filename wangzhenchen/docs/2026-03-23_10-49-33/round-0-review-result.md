# Round 0 Review

## Findings

1. Blocking: the `asterinas_scml` corpus is still admitted by static name-level derivation, not by a runtime SCML gate. `tools/derive_scml_allowed_sequences.py:83-116` loads `meta_path`, checks only `full_syscall_list`, and writes directly to `eligible_programs/asterinas_scml.jsonl`; there is still no `tools/preflight_scml_gate.py`. This already admits sequences the plan says must be rejected before candidate execution. Concrete accepted examples:
   - `eligible_programs/asterinas_scml.jsonl:296` points at `corpus/normalized/505a3d94e57295c2f6814abae501560fe464fb972e0644d21fc0247d3669f48d.syz:3`, which executes `lseek(..., 0x4)` (`SEEK_HOLE`).
   - `eligible_programs/asterinas_scml.jsonl:356` points at `corpus/normalized/6043a7ac77bc2339f58832c51c1aa1c2af66a1309eee45a3eccdd87f80592744.syz:2`, which executes `lseek(..., 0x3)` (`SEEK_DATA`).
   - `eligible_programs/asterinas_scml.jsonl:459` points at `corpus/normalized/7eabe657bdb1075ac265c5e61d229938cc189a2c3d551470dc519cb7a6ab0373.syz:2`, which again executes `lseek(..., 0x4)` (`SEEK_HOLE`).
   This is a direct violation of the original plan’s AC-5 and AC-8 requirements in `docs/asterinas-scml-diff-plan.md:371-424`.

2. Blocking: the execution/reporting path is still completely unaware of SCML preflight state. `orchestrator/scheduler.py:80-169` runs candidate execution for every eligible row without reading or recording any preflight evidence; `orchestrator/scheduler.py:205-244`, `tools/render_summary.py:39-130`, and `tools/reduce_case.py:132-170` do not emit `scml_preflight_status`, SCML-aware classification buckets, or SCML evidence paths. The required report artifacts are correspondingly missing: `reports/asterinas_scml/` currently contains only `derivation-summary.json` and `scml-rejections.jsonl`, with no `summary.json`, `signoff.md`, `divergence-index.jsonl`, or SCML-aware minimized report. That leaves AC-6, AC-7, AC-10, and AC-11 from `docs/asterinas-scml-diff-plan.md:383-468` and `docs/asterinas-scml-diff-plan.md:543-547` unmet.

3. High: the profile/manifest contract still cannot express required syscall-level defers, so privileged syscalls are implicitly campaign-enabled. `compat_specs/asterinas/generation-profile.json:19-24` enables the whole `system-information-and-misc` category, while `compat_specs/asterinas/scml-manifest.json:6006-6009` marks `reboot` as `generation_enabled: true` with `defer_reason: null`. The plan explicitly requires high-privilege paths like `reboot / pivot_root / mount / unshare / setns` to be deferred with reasons and forbids default-including a syscall merely because it appears in SCML (`docs/asterinas-scml-diff-plan.md:349-358`, `docs/asterinas-scml-diff-plan.md:520-522`, `docs/asterinas-scml-diff-plan.md:574-579`). The current category-only profile does not satisfy that contract.

4. High: the tracked rejection taxonomy and the tracker scope both understate the remaining work. `configs/asterinas_scml_rules.json:35-49` defines only static derivation reasons and omits the plan-required preflight reasons `unsupported_flag_pattern`, `unsupported_struct_pattern`, `unsupported_path_pattern`, and `scml_parser_gap` from `docs/asterinas-scml-diff-plan.md:409-424`. At the same time, `.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md:74-75` explicitly defers the runtime gate and SCML-aware reporting even though those are core acceptance criteria, and `.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md:28-39` compresses away original-plan AC-9, AC-10, and AC-11 as independent tracked items. The tracker is therefore not a faithful representation of plan-complete scope.

5. Medium: the new alias fields are still not a reliable consumer-facing schema because they mirror README prose instead of normalized tokens. `compat_specs/asterinas/scml-manifest.json:5929-5935` records `getrandom.ignored_flags` as `GRND_NONBLOCK\` because the underlying operation never blocks`, sourced from `third_party/asterinas/book/src/kernel/linux-compatibility/syscall-flag-coverage/system-information-and-misc/README.md:48-49`. Similar combined prose appears in other alias-style fields. That means the manifest slice Claude claims to have “aligned” is still not ready to drive downstream gating logic without another normalization pass.

## Goal Alignment Summary

ACs: 3/6 addressed | Forgotten items: 4 | Unjustified deferrals: 2

- Progress exists on AC-1 and AC-2, and there is partial static-only progress on AC-4.
- AC-3 is only partially addressed because the profile cannot defer privileged syscalls inside enabled categories.
- AC-5 and AC-6 are not implemented; they are explicitly deferred in the tracker.
- Forgotten original-plan items that are not tracked as first-class remaining scope include:
  - the full preflight rejection taxonomy (original AC-8),
  - target-neutral capability/gate abstractions (original AC-9),
  - smoke/sign-off production and SCML-gated metrics (original AC-10),
  - a rerunnable SCML-passed minimized report requirement (original AC-11).
- The deferrals in `.humanize/rlcr/2026-03-23_10-49-33/goal-tracker.md:74-75` are not justified for this review because the round is being presented as completed work against the original implementation plan, and the user explicitly asked that deferred tasks be forced to completion rather than carried forward.

## Goal Tracker Update Request Handling

No `Goal Tracker Update Request` section was provided in Claude’s summary, so I did not edit `goal-tracker.md`.

## Directive Implementation Plan

1. Finish the manifest/profile contract instead of leaving it category-only. Extend the SCML workflow data model so every syscall has an effective `generation_enabled` / `defer_reason` decision after profile application, and explicitly defer privileged or environment-destroying syscalls such as `reboot`, `mount`, `pivot_root`, `unshare`, and `setns` even when their broader category is enabled. Add tests that the derived allowlist excludes those syscalls and that defer reasons are materialized, not implicit.

2. Split static derivation from runtime admission. Keep `tools/derive_scml_allowed_sequences.py` as a static candidate filter if desired, but stop writing its output as the final eligible corpus. Introduce `tools/preflight_scml_gate.py` that consumes the statically accepted candidates, runs each one on Linux, captures `strace -yy -f`, invokes `sctrace` against the SCML source, and writes two outputs:
   - the final `eligible_programs/asterinas_scml.jsonl` containing only SCML-passed rows,
   - `reports/asterinas_scml/scml-rejections.jsonl` with stable structured reasons.

3. Implement the full rejection taxonomy required by the plan. The preflight gate must classify at least `syscall_not_in_manifest`, `unsupported_flag_pattern`, `unsupported_struct_pattern`, `unsupported_path_pattern`, `deferred_category`, and `scml_parser_gap`, and it must keep the reason stable for repeated runs of the same input. Add focused tests that reject:
   - `renameat2(..., RENAME_EXCHANGE)`,
   - `lseek(..., SEEK_DATA)` and `lseek(..., SEEK_HOLE)`,
   - at least one unsupported path/struct-pattern example taken from the current SCML corpus.

4. Thread SCML evidence through execution instead of bolting it on later. Extend the eligible-row schema and campaign result schema with `scml_preflight_status`, `scml_rejection_reason`, preflight evidence paths, and any SCML matcher diagnostics you need. Update `orchestrator/scheduler.py`, `tools/render_summary.py`, and `tools/reduce_case.py` so the workflow can distinguish and report:
   - `rejected_by_scml`,
   - `passed_scml_but_candidate_failed`,
   - `passed_scml_and_no_diff`,
   - `passed_scml_and_diverged`.

5. Produce the missing artifacts end to end. Run the SCML workflow on real preflight-passed programs and materialize `reports/asterinas_scml/summary.json`, `reports/asterinas_scml/signoff.md`, `reports/asterinas_scml/divergence-index.jsonl`, and at least one minimized report whose testcase has `scml_preflight_status=passed`. Do not use a preflight-rejected testcase to satisfy the minimized-report requirement.

6. Close the tracker gap operationally even if the immutable section cannot be rewritten. Treat the omitted original-plan items as mandatory remaining work in subsequent rounds: target-neutral capability/gate naming, smoke/sign-off metrics that include SCML gate statistics, and the rerunnable SCML-passed minimized report. Do not mark the work complete until those artifacts exist and the SCML gate is the actual hard boundary before candidate execution.

## Verification Performed

- `python3 -m unittest tests.test_scml_manifest tests.test_scml_derivation`

