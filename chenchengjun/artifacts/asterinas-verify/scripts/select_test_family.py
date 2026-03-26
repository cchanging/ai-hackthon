#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json

from review_models import (
    build_validation_plan,
    detect_domain_from_paths,
    select_test_family_for_syscall,
    suggest_subdir,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Suggest a general test family and validation plan for an Asterinas review target.",
    )
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--syscall", help="Syscall name or path, such as readlink or kernel/src/syscall/readlink.rs")
    group.add_argument("--paths", nargs="+", help="Anchor file paths for a module or feature review")
    parser.add_argument(
        "--kind",
        choices=("syscall", "module", "feature-interface"),
        default="module",
        help="Review-unit kind when using --paths",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.syscall:
        family = select_test_family_for_syscall(args.syscall)
        validation_plan = "general test + verify" if family is not None else "report-only"
        output = {
            "kind": "syscall",
            "domain": "syscall",
            "recommended_validation_plan": validation_plan,
            "general_test_module": family,
            "candidate_subdir": None,
            "validation_feasible": family is not None,
            "notes": [
                "Prefer the closest existing subdirectory under the selected top-level module when a general test module is known.",
            ],
        }
    else:
        domain = detect_domain_from_paths(args.paths)
        validation_plan, family = build_validation_plan(domain)
        feasible = validation_plan == "general test + verify" and family is not None
        output = {
            "kind": args.kind,
            "domain": domain,
            "recommended_validation_plan": validation_plan,
            "general_test_module": family,
            "candidate_subdir": suggest_subdir(domain, args.paths) if family is not None else None,
            "validation_feasible": feasible,
            "notes": [
                "Choose `report-only` when no crisp user-visible general-test oracle exists.",
            ],
        }
    print(json.dumps(output, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
