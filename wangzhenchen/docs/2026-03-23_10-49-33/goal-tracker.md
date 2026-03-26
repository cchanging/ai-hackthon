# Goal Tracker

<!--
This file tracks the ultimate goal, acceptance criteria, and plan evolution.
It prevents goal drift by maintaining a persistent anchor across all rounds.

RULES:
- IMMUTABLE SECTION: Do not modify after initialization
- MUTABLE SECTION: Update each round, but document all changes
- Every task must be in one of: Active, Completed, or Deferred
- Deferred items require explicit justification
-->

## IMMUTABLE SECTION
<!-- Do not modify after initialization -->

### Ultimate Goal
把 Asterinas 已文档化的 SCML 支持边界提升为当前仓库中的正式能力输入，使最终进入 `Linux reference vs Asterinas candidate` 差分执行的 testcase 都满足两层约束：其 syscall 必须来自 SCML manifest/profile 导出的允许集合，且其运行时调用模式必须通过 Linux 上的 SCML preflight gate。该 workflow 需要降低 unconstrained workload 的 timeout/unsupported 噪声，并把 candidate 失败更集中地暴露为真实行为差异或内核问题。

Source plan: docs/asterinas-scml-diff-plan.md

### Acceptance Criteria
<!-- Each criterion must be independently verifiable -->
<!-- Claude must extract or define these in Round 0 -->

以下验收标准从源计划抽取并归一化为当前 loop 的执行锚点。

- AC-1：SCML 输入源必须固定、完整且可复现
  - 独立验证：manifest 产物记录 source revision、SCML 根路径和动态统计；同一 revision 重跑得到一致计数。
- AC-2：manifest 必须保留每个 syscall 的可追溯来源和支持约束
  - 独立验证：每个条目都包含 source SCML 文件、support tier、notes，以及 consumer-friendly 的 ignored/partial/unsupported 约束字段。
- AC-3：必须用稳定差分 profile 分离“SCML 支持”与“当前 campaign 启用”
  - 独立验证：enabled/deferred categories 有结构化原因，且 allowlist 只来源于 manifest/profile。
- AC-4：静态 derivation 只能让 manifest/profile 允许的 unspecialized 序列进入 SCML workflow
  - 独立验证：未在 manifest 中声明、被 profile defer、或 specialized variant 的序列都被拒绝。
- AC-5：runtime SCML preflight 必须是进入 candidate 执行前的硬门槛
  - 独立验证：每个候选序列在 Linux 上产生 runtime trace，经 `sctrace` 或等价 gate 通过后才能进入最终 eligible corpus，并且 rejection reason 稳定落盘。
- AC-6：通过 preflight 的样例必须复用现有 Linux/Asterinas 差分主链，并输出 SCML-aware 报告
  - 独立验证：summary、rejections、divergence index、signoff/minimized report 能区分 `rejected_by_scml`、`passed_scml_but_candidate_failed`、`passed_scml_and_no_diff`、`passed_scml_and_diverged`。

---

## MUTABLE SECTION
<!-- Update each round with justification for changes -->

### Plan Version: 7 (Updated: Round 6 review)

#### Plan Evolution Log
<!-- Document any changes to the plan with justification -->
| Round | Change | Reason | Impact on AC |
|-------|--------|--------|--------------|
| 0 | Initialized goal tracker from the SCML diff plan and selected manifest-schema alignment as the first implementation slice | The repository already contains SCML scaffolding; tightening the manifest contract is the lowest-risk dependency-ordered step before profile/gate/report integration | Advances AC-1 and AC-2 first without blocking later AC-3 to AC-6 work |
| 1 | Split `asterinas_scml` admission into static derivation (`eligible_programs/asterinas_scml.static.jsonl`) and runtime preflight admission (`eligible_programs/asterinas_scml.jsonl`) | This matches the original plan's two-stage gating model and keeps runtime SCML evidence separate from name-level derivation | Advances AC-4, AC-5, and AC-6, but still needs artifact-stability fixes before the round can be treated as complete |
| 2 | Filtered SCML preflight runs now write debug ledgers under `reports/asterinas_scml/debug-preflight/<label>/`, and stale partial formal SCML outputs were removed | Prevent subset verification runs from overwriting the top-level eligible corpus/rejection ledgers and stop misleading partial artifacts from being treated as sign-off evidence | Advances AC-5 and AC-6, but the workflow still needs isolated per-program evidence paths and a full-corpus rerun |
| 2 | Introduced `CapabilitySource` / `SequenceGate` abstractions and routed derivation/preflight through the Asterinas backend | Closes the original-plan abstraction gap without prematurely adding a second target | Advances original-plan AC-9 while leaving AC-10 and AC-11 active |
| 3 | Isolated filtered/debug SCML preflight evidence under debug-specific artifact roots and regenerated the official full-corpus preflight outputs | This closes the remaining destructive-preflight gap and restores the formal eligible/rejection baseline that later campaign artifacts depend on | Completes AC-5 and advances AC-6 |
| 3 | Reused `build-result.json`, parallelized `finalize_prepared_case()` triage, and reused packaged candidate bundles during scheduler triage | Removes the round-2 runtime inflation so the repository can produce real smoke/full SCML campaign artifacts in one round | Advances AC-6 and original-plan AC-10, but the reducer replay path still needs equivalent packaged reuse for AC-11 |
| 4 | Reducer source-seed replay now reuses packaged campaign candidate contexts, packaged bundle reuse validates revision/docker image/`kcmd_args`/initramfs metadata, and trace-complete crash rows stay on the SCML comparison path | Closes the round-3 replay/cache/reporting gaps that were suppressing real divergence evidence and stale-bundle invalidation | Completes AC-6 and original-plan AC-9/AC-10, while leaving AC-11 dependent on a fresh rerun check |
| 4 | Regenerated the official 500-case SCML campaign outputs and a real minimized report from a `passed_scml_and_diverged` seed | The repository now contains passing sign-off artifacts plus a non-synthetic minimized report instead of only code-path readiness | Advances original-plan AC-10 and AC-11, but a fresh reducer rerun still needs to succeed in the current review environment |
| 5 | Routed Asterinas temp files through runtime temp-dir config, added raw-header selector fallback for packaged guests, and introduced SCML target export/generation tooling | Round 5 closed the old workspace temp-root failure and started the planned forward-generation path | Advances original-plan AC-11 portability and milestone 3, but does not complete AC-11 because fresh reruns still need exact packaged-context replay and the official derivation/report path now needs provenance cleanup |
| 6 | Recorded exact candidate package provenance in SCML campaign rows, restored the official derivation source to `eligible_programs/baseline.jsonl`, and gated generation-metric import on the configured derivation input | These are real fixes for the specific Round 5 provenance and generated-summary contamination regressions | Advances original-plan AC-10 and AC-11, but does not complete them because the exact reducer replay still fails fresh and the formal summary still shares the wrong rejection ledger with derivation output |

#### Active Tasks
<!-- Map each task to its target Acceptance Criterion -->
| Task | Target AC | Status | Notes |
|------|-----------|--------|-------|
| Make `tools/reduce_case.py --workflow asterinas_scml --program-id <real-diverged-id>` rerunnable in the current workspace-write environment | original-plan AC-11 | pending | The package-provenance fix landed, but a fresh rerun of `python3 tools/reduce_case.py --workflow asterinas_scml --program-id 0492bbe25ce222396170176f4b59c84c743421f5d1fe9f0a08fad21488ae9a63` still exits in `greedy_reduce()`: the first fresh replay of the source testcase now uses the recorded SCML package path/slot, yet the candidate rerun comes back `infra_error`, the comparison regresses to `event_count_mismatch` at event `0`, and there is still no valid syscall divergence index. The fresh candidate stderr shows `tools/run_asterinas.py` trying to refresh Asterinas Git mirrors through the configured proxy, so the packaged replay path is still not reproducible in the current workspace-write environment |
| Keep the formal SCML rejection ledger and sign-off summary sourced from runtime preflight evidence instead of derivation-stage rejects | AC-5 / AC-6 / original-plan AC-10 | pending | `tools/derive_scml_allowed_sequences.py` and `tools/preflight_scml_gate.py` still share `reports/asterinas_scml/scml-rejections.jsonl`. The checked-in file currently contains `703` derivation rejects with `reasons`, while `reports/asterinas_scml/preflight-summary.json` reports `697` runtime rejects; `tools/render_summary.py` therefore publishes stale `rejected_by_scml` / `scml_preflight_pass_rate` values in the formal summary |

### Completed and Verified
<!-- Only move tasks here after Codex verification -->
| AC | Task | Completed Round | Verified Round | Evidence |
|----|------|-----------------|----------------|----------|
| AC-1 | Rebuilt the SCML manifest from the current Asterinas snapshot and kept dynamic source metadata in the artifact | 0 | 1 | `python3 tools/build_scml_manifest.py`; regenerated `compat_specs/asterinas/scml-manifest.json`; `python3 -m unittest tests.test_scml_manifest` |
| AC-2 | Aligned manifest syscall entries with plan-facing alias fields such as `ignored_flags`, `partial_flags`, `unsupported_flags`, while preserving the existing bucketed metadata | 0 | 1 | Updated `tools/build_scml_manifest.py`; added assertions in `tests/test_scml_manifest.py`; `python3 -m unittest tests.test_scml_manifest` |
| AC-3 | Profile application now projects syscall-level defer decisions into derivation, including explicit `reboot` deferral | 1 | 1 | Updated `compat_specs/asterinas/generation-profile.json` and `tools/derive_scml_allowed_sequences.py`; `python3 -m unittest tests.test_scml_derivation` |
| AC-4 | Confirmed the new manifest schema remains compatible with current SCML derivation logic | 0 | 1 | `python3 -m unittest tests.test_scml_derivation` |
| AC-5 | Isolated filtered/debug SCML preflight evidence and regenerated the official full-corpus preflight artifacts | 3 | 3 | Verified `reports/asterinas_scml/debug-preflight/program-505a3d94e57295c2/scml-rejections.jsonl` now points at `artifacts/preflight/asterinas_scml/debug/...`; regenerated `eligible_programs/asterinas_scml.jsonl`, `reports/asterinas_scml/scml-rejections.jsonl`, and `reports/asterinas_scml/preflight-summary.json` with `1298` source rows, `601` eligible, `697` rejected |
| AC-6 | Threaded `scml_preflight_status` and SCML result buckets through scheduler outputs, summary/sign-off reports, and the minimized report with exact-program evidence | 4 | 4 | Verified `reports/asterinas_scml/summary.json` now reports `rejected_by_scml=697`, `passed_scml_but_candidate_failed=2`, `passed_scml_and_no_diff=3`, and `passed_scml_and_diverged=495`; verified `reports/asterinas_scml/minimized-report.json` for program `01ec90666ede845c1b4215e4ad9a094ab568e3c79588984aa5ac895775455eb1` carries `scml_preflight_status=passed` plus non-null divergence indices; `python3 -m unittest tests.test_scml_reduce_case tests.test_scml_reporting` |
| AC-6 plus original-plan AC-9/AC-10 | Completed the target-neutral capability/gate abstraction and produced official campaign-scale smoke/sign-off evidence | 4 | 4 | Verified `CapabilitySource` / `SequenceGate` usage remains in place; `reports/asterinas_scml/summary.json` now shows `total=500`, `dual_execution_completion_rate=0.996`, `trace_generation_success_rate=1.000`, `canonicalization_success_rate=1.000`, `baseline_invalid_rate=0.004`, and `signoff_pass=true`; reran `python3 tools/render_summary.py --workflow asterinas_scml`; `python3 -m unittest tests.test_asterinas_pipeline.AsterinasPipelineTests.test_ensure_packaged_docker_bundle_rebuilds_on_metadata_mismatch tests.test_asterinas_pipeline.AsterinasPipelineTests.test_write_bug_likely_reports_materializes_index_and_testcase_copy tests.test_asterinas_pipeline.AsterinasPipelineTests.test_scheduler_main_writes_summary_signoff_and_failure_reports` |

### Explicitly Deferred
<!-- Items here require strong justification -->
| Task | Original AC | Deferred Since | Justification | When to Reconsider |
|------|-------------|----------------|---------------|-------------------|
| None | n/a | n/a | No deferrals are currently justified; remaining scope stays active until the original plan's ACs are actually met | n/a |

### Open Issues
<!-- Issues discovered during implementation -->
| Issue | Discovered Round | Blocking AC | Resolution Path |
|-------|-----------------|-------------|-----------------|
| `tools/reduce_case.py` now records and consumes exact package provenance, but the packaged replay path is still not reproducible in the current workspace-write environment because `tools/run_asterinas.py` refreshes Asterinas Git mirrors during Docker command setup. The exact recorded rerun command still fails fresh with candidate `infra_error`, `event_count_mismatch` at event `0`, and no valid syscall divergence index when mirror updates cannot reach the configured proxy | 6 | original-plan AC-11 | Make packaged reruns reuse existing local mirrors and packaged bundles without `git remote update` or any other network access; only explicit bootstrap/cache-priming flows may refresh mirrors, and add an end-to-end regression that the recorded `reduce_case.py --program-id ...` command succeeds in the current workspace-write environment |
| `tools/derive_scml_allowed_sequences.py` and `tools/preflight_scml_gate.py` still share the top-level `reports/asterinas_scml/scml-rejections.jsonl` artifact. The checked-in file currently contains derivation-stage rejects, but `tools/render_summary.py` treats it as runtime preflight evidence, so `summary.json` / `signoff.md` no longer publish trustworthy SCML gate metrics | 6 | original-plan AC-10 | Split derivation and preflight rejection ledgers into distinct artifact paths, keep `scml-rejections.jsonl` reserved for runtime preflight rejects, update `render_summary.py` to consume the runtime ledger only, and rerender the formal reports after regenerating the official chain |
