#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

from classify_review_unit import classify_units, load_inputs
from paths import repo_root


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Prepare a compact review manifest for asterinas-verify.",
    )

    classify_group = parser.add_mutually_exclusive_group()
    classify_group.add_argument("--git-range", help="Git revision or revision range to inspect")
    classify_group.add_argument("--patch-file", help="Path to a unified diff patch file")
    parser.add_argument(
        "paths",
        nargs="*",
        help="Changed file paths when not using --git-range or --patch-file",
    )
    parser.add_argument(
        "--repo",
        help="Repository root for classification. Defaults to current git toplevel or cwd.",
    )
    parser.add_argument(
        "--slug",
        help="Optional review slug to pre-fill default report paths (used for change/module).",
    )
    return parser.parse_args()


def default_reports(repo: Path, slug: str | None) -> dict[str, str | None]:
    return {
        "change": str((repo / "change-review" / f"{slug}.md")) if slug else None,
        "module": str((repo / "change-review" / f"{slug}.md")) if slug else None,
        "syscall": None,
    }


def main() -> int:
    args = parse_args()
    repo = Path(args.repo or repo_root())

    # Reuse the classifier to keep logic in one place; discard patch text to avoid bloat.
    changed_paths, patch_text, input_mode = load_inputs(args)
    review_units = classify_units(changed_paths, patch_text)

    manifest = {
        "repo_root": str(repo),
        "input_mode": input_mode,
        "changed_paths": changed_paths,
        "review_units": review_units,
        "default_reports": default_reports(repo, args.slug),
        "schemas": {
            "finding": {
                "id": "string (stable per finding)",
                "class": "bug | unsupported | contract-risk | regression-risk | open-question",
                "claim": "single-sentence summary",
                "anchors": ["<repo-relative-path>:<line?>"],
                "input_cases": ["zero-length buffer", "invalid user pointer"],
                "evidence": [
                    {
                        "id": "Ex",
                        "source": "spec | linux-derived | asterinas-contract | diff-intent",
                        "path": "<repo-relative-path>:<line?>",
                        "text": "short paraphrase or snippet",
                    }
                ],
                "test": {
                    "kind": "general | report-only",
                    "module": "fs|process|...|null",
                    "goal": "regression | confirmation",
                    "idea": "short test intent",
                    "oracle": "what observable result confirms the behavior",
                },
                "confidence": "low|med|high",
            }
        },
    }

    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
