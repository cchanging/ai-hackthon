#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import sys
from typing import Iterable


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Render structured findings JSON into markdown cards and an evidence appendix.",
    )
    parser.add_argument(
        "--findings",
        help="Path to findings JSON (list of finding objects). Defaults to stdin.",
    )
    return parser.parse_args()


def load_findings(path: str | None) -> list[dict]:
    if path:
        return json.loads(open(path, "r", encoding="utf-8").read())
    return json.loads(sys.stdin.read())


def render_findings(findings: Iterable[dict]) -> str:
    lines: list[str] = []
    lines.append("## Findings (cards; order by severity)")
    if not findings:
        lines.append("")
        lines.append("- None recorded yet.")
        return "\n".join(lines)

    for f in findings:
        evidence_ids = ", ".join(e.get("id", "") for e in f.get("evidence", []))
        anchors = ", ".join(f.get("anchors", []))
        lines.append("")
        lines.append(f"- **{f.get('class','').upper()}** — {f.get('claim','<claim>')}")
        lines.append(f"  - Anchors: {anchors or '<add anchors>'}")
        lines.append(f"  - Evidence: {evidence_ids or '<add evidence ids>'}")
        input_cases = ", ".join(f.get("input_cases", []))
        if input_cases:
            lines.append(f"  - Input cases: {input_cases}")
        lines.append(f"  - Confidence: {f.get('confidence','') or '<set confidence>'}")
        test = f.get("test")
        if test:
            goal = test.get("goal", "?")
            oracle = test.get("oracle", "")
            lines.append(
                f"  - Test: {test.get('kind','?')} / {test.get('module','?')} / {goal} — {test.get('idea','')}"
            )
            if oracle:
                lines.append(f"  - Oracle: {oracle}")

    lines.append("")
    lines.append("## Evidence Appendix")
    lines.append("")
    lines.append("| ID | Source | Path | Note | Supports |")
    lines.append("|----|--------|------|------|----------|")
    any_evidence = False
    for f in findings:
        for e in f.get("evidence", []):
            any_evidence = True
            lines.append(
                "| {id} | {source} | {path} | {text} | {finding} |".format(
                    id=e.get("id", ""),
                    source=e.get("source", ""),
                    path=e.get("path", ""),
                    text=e.get("text", "").replace("|", "\\|"),
                    finding=f.get("id", ""),
                )
            )
    if not any_evidence:
        lines.append("| - | - | - | - | - |")
    return "\n".join(lines)


def main() -> int:
    args = parse_args()
    findings = load_findings(args.findings)
    print(render_findings(findings))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
