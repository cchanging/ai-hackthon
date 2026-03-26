from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from tools import reduce_case
from tools.reduce_case import find_campaign_package_context, scml_reduction_invariants_hold, select_scml_campaign_row


class SCMLReduceCaseTests(unittest.TestCase):
    class FakeRun(SimpleNamespace):
        def to_dict(self) -> dict[str, object]:
            return {
                "status": self.status,
                "trace_json_path": self.trace_json_path,
                "external_state_path": self.external_state_path,
                "console_log_path": getattr(self, "console_log_path", ""),
            }

    def test_select_scml_campaign_row_requires_diverged_bucket_for_program_id(self) -> None:
        rows = [
            {
                "program_id": "no-diff",
                "scml_preflight_status": "passed",
                "scml_result_bucket": "passed_scml_and_no_diff",
            }
        ]
        with self.assertRaises(SystemExit):
            select_scml_campaign_row(rows, program_id="no-diff")

    def test_select_scml_campaign_row_accepts_passed_diverged_case(self) -> None:
        row = {
            "program_id": "diverged",
            "scml_preflight_status": "passed",
            "scml_result_bucket": "passed_scml_and_diverged",
        }
        self.assertEqual(select_scml_campaign_row([row]), row)

    def test_scml_reduction_invariants_require_non_runtime_divergence_index(self) -> None:
        comparison = {"equivalent": False, "first_divergence_index": 1}
        reference_canonical = {
            "events": [
                {"index": 0, "syscall_name": "mmap"},
                {"index": 1, "syscall_name": "exit_group"},
            ]
        }
        self.assertTrue(scml_reduction_invariants_hold(comparison, reference_canonical, "passed"))

    def test_scml_reduction_invariants_reject_missing_program_syscall_index(self) -> None:
        comparison = {"equivalent": False, "first_divergence_index": 0}
        reference_canonical = {
            "events": [
                {"index": 0, "syscall_name": "mmap"},
                {"index": 1, "syscall_name": "exit_group"},
            ]
        }
        self.assertFalse(scml_reduction_invariants_hold(comparison, reference_canonical, "passed"))
        self.assertFalse(scml_reduction_invariants_hold(comparison, reference_canonical, "rejected_by_scml"))

    def test_run_case_uses_packaged_candidate_path_for_asterinas_scml(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            program_path = root / "program.syz"
            program_path.write_text("openat(0, 0, 0)\n", encoding="utf-8")
            reference_trace = root / "reference-trace.json"
            reference_state = root / "reference-state.json"
            reference_triage_trace = root / "reference-triage-trace.json"
            reference_triage_state = root / "reference-triage-state.json"
            candidate_trace = root / "candidate-trace.json"
            candidate_state = root / "candidate-state.json"
            candidate_triage_trace = root / "candidate-triage-trace.json"
            candidate_triage_state = root / "candidate-triage-state.json"
            for path, payload in (
                (reference_trace, {"events": []}),
                (reference_state, {"files": []}),
                (reference_triage_trace, {"events": []}),
                (reference_triage_state, {"files": []}),
                (candidate_trace, {"events": []}),
                (candidate_state, {"files": []}),
                (candidate_triage_trace, {"events": []}),
                (candidate_triage_state, {"files": []}),
            ):
                path.write_text(json.dumps(payload), encoding="utf-8")

            reference_run = self.FakeRun(
                status="ok",
                trace_json_path=str(reference_trace),
                external_state_path=str(reference_state),
                console_log_path=str(root / "reference-console.log"),
            )
            candidate_run = self.FakeRun(
                status="ok",
                trace_json_path=str(candidate_trace),
                external_state_path=str(candidate_state),
                console_log_path=str(root / "candidate-console.log"),
            )
            reference_triage_run = self.FakeRun(
                status="ok",
                trace_json_path=str(reference_triage_trace),
                external_state_path=str(reference_triage_state),
                console_log_path=str(root / "reference-triage-console.log"),
            )
            candidate_triage_run = self.FakeRun(
                status="ok",
                trace_json_path=str(candidate_triage_trace),
                external_state_path=str(candidate_triage_state),
                console_log_path=str(root / "candidate-triage-console.log"),
            )

            with patch(
                "tools.reduce_case.config",
                return_value={
                    "workflow": "asterinas_scml",
                    "stability": {"timeout_sec": 120, "rerun_count": 1},
                    "normalization": {"runtime_syscalls": []},
                },
            ), patch(
                "tools.reduce_case.inspect_program",
                return_value={"program_id": "diverged", "call_count": 1},
            ), patch(
                "tools.reduce_case.build_one"
            ), patch(
                "tools.reduce_case.execute_side",
                side_effect=[reference_run, reference_triage_run],
            ) as execute_side, patch(
                "tools.reduce_case.execute_candidate_batch_with_context",
                return_value=({"diverged": candidate_run}, Path("/tmp/package"), {"diverged": 0}),
            ) as execute_candidate_batch_with_context, patch(
                "tools.reduce_case.execute_candidate_case_in_package",
                return_value=candidate_triage_run,
            ) as execute_candidate_case_in_package, patch(
                "tools.reduce_case.canonicalize",
                side_effect=lambda raw, external: {"events": raw.get("events", [])},
            ), patch(
                "tools.reduce_case.compare_canonical",
                side_effect=[
                    {"equivalent": False, "first_divergence_index": 0},
                    {"equivalent": True, "first_divergence_index": None},
                ],
            ), patch(
                "tools.reduce_case.dump_json"
            ):
                info, comparison, runs = reduce_case.run_case(program_path)

        self.assertEqual(info["program_id"], "diverged")
        self.assertTrue(comparison["equivalent"])
        self.assertEqual(runs["candidate"]["status"], "ok")
        self.assertEqual(execute_side.call_count, 2)
        self.assertEqual(execute_side.call_args_list[0].kwargs["side"], "reference")
        execute_candidate_batch_with_context.assert_called_once()
        execute_candidate_case_in_package.assert_called_once()
        batch_case = execute_candidate_batch_with_context.call_args.kwargs["batch_cases"][0]
        self.assertEqual(batch_case["program_id"], "diverged")
        self.assertTrue(batch_case["run_id"].endswith("-candidate"))

    def test_run_case_prefers_campaign_package_context_for_source_seed(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            program_path = root / "program.syz"
            program_path.write_text("openat(0, 0, 0)\n", encoding="utf-8")
            reference_trace = root / "reference-trace.json"
            reference_state = root / "reference-state.json"
            candidate_trace = root / "candidate-trace.json"
            candidate_state = root / "candidate-state.json"
            for path, payload in (
                (reference_trace, {"events": []}),
                (reference_state, {"files": []}),
                (candidate_trace, {"events": []}),
                (candidate_state, {"files": []}),
            ):
                path.write_text(json.dumps(payload), encoding="utf-8")

            reference_run = self.FakeRun(
                status="ok",
                trace_json_path=str(reference_trace),
                external_state_path=str(reference_state),
                console_log_path=str(root / "reference-console.log"),
            )
            candidate_run = self.FakeRun(
                status="ok",
                trace_json_path=str(candidate_trace),
                external_state_path=str(candidate_state),
                console_log_path=str(root / "candidate-console.log"),
            )

            with patch(
                "tools.reduce_case.config",
                return_value={
                    "workflow": "asterinas_scml",
                    "stability": {"timeout_sec": 120, "rerun_count": 0},
                    "normalization": {"runtime_syscalls": []},
                },
            ), patch(
                "tools.reduce_case.inspect_program",
                return_value={"program_id": "diverged", "call_count": 1},
            ), patch(
                "tools.reduce_case.build_one"
            ), patch(
                "tools.reduce_case.execute_side",
                return_value=reference_run,
            ), patch(
                "tools.reduce_case.execute_candidate_case_in_package",
                return_value=candidate_run,
            ) as execute_candidate_case_in_package, patch(
                "tools.reduce_case.execute_candidate_batch_with_context",
                side_effect=AssertionError("unexpected single-case package creation"),
            ), patch(
                "tools.reduce_case.canonicalize",
                side_effect=lambda raw, external: {"events": raw.get("events", [])},
            ), patch(
                "tools.reduce_case.compare_canonical",
                return_value={"equivalent": True, "first_divergence_index": None},
            ), patch(
                "tools.reduce_case.dump_json"
            ):
                reduce_case.run_case(
                    program_path,
                    campaign_package_dir=Path("/tmp/campaign-package"),
                    campaign_package_slot=7,
                )

        execute_candidate_case_in_package.assert_called_once()
        self.assertEqual(execute_candidate_case_in_package.call_args.kwargs["package_dir"], Path("/tmp/campaign-package"))
        self.assertEqual(execute_candidate_case_in_package.call_args.kwargs["slot"], 7)

    def test_find_campaign_package_context_prefers_larger_campaign_package(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            campaign_dir = root / "campaign"
            single_dir = root / "single"
            campaign_dir.mkdir()
            single_dir.mkdir()
            (campaign_dir / "package-manifest.json").write_text(
                json.dumps(
                    {
                        "workflow": "asterinas_scml",
                        "cases": [
                            {"program_id": "target", "slot": 5},
                            {"program_id": "other", "slot": 6},
                        ]
                    }
                ),
                encoding="utf-8",
            )
            (single_dir / "package-manifest.json").write_text(
                json.dumps({"workflow": "asterinas", "cases": [{"program_id": "target", "slot": 0}]}),
                encoding="utf-8",
            )
            with patch("tools.reduce_case.Path.exists", return_value=True), patch(
                "tools.reduce_case.Path.iterdir",
                return_value=[single_dir, campaign_dir],
            ):
                context = find_campaign_package_context("target", workflow="asterinas_scml")

        self.assertIsNotNone(context)
        self.assertEqual(context["slot"], 5)
        self.assertEqual(Path(context["package_dir"]), campaign_dir.resolve())

    def test_seed_program_prefers_campaign_row_package_provenance(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            eligible_file = root / "eligible.jsonl"
            campaign_results = root / "campaign-results.jsonl"
            normalized = root / "program.syz"
            meta = root / "program.json"
            normalized.write_text("openat(0x0)\n", encoding="utf-8")
            meta.write_text("{}", encoding="utf-8")
            eligible_file.write_text(
                json.dumps(
                    {
                        "program_id": "target",
                        "normalized_path": str(normalized),
                        "meta_path": str(meta),
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            campaign_results.write_text(
                json.dumps(
                    {
                        "program_id": "target",
                        "scml_preflight_status": "passed",
                        "scml_result_bucket": "passed_scml_and_diverged",
                        "candidate_package_dir": "/tmp/package",
                        "candidate_package_slot": 9,
                    }
                )
                + "\n",
                encoding="utf-8",
            )

            with patch(
                "tools.reduce_case.config",
                return_value={
                    "workflow": "asterinas_scml",
                    "paths": {"eligible_file": str(eligible_file)},
                },
            ), patch(
                "tools.reduce_case.report_path",
                return_value=campaign_results,
            ), patch(
                "tools.reduce_case.find_campaign_package_context",
                side_effect=AssertionError("unexpected fallback lookup"),
            ):
                program_path, row = reduce_case.seed_program("fixture", program_id="target")

        self.assertEqual(program_path, normalized)
        self.assertEqual(row["campaign_package_dir"], "/tmp/package")
        self.assertEqual(row["campaign_package_slot"], 9)


if __name__ == "__main__":
    unittest.main()
