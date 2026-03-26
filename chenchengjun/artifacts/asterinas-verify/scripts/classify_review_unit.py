#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

CURRENT_DIR = Path(__file__).resolve().parent
if str(CURRENT_DIR) not in sys.path:
    sys.path.insert(0, str(CURRENT_DIR))

from paths import repo_root
from review_models import (
    build_validation_plan,
    detect_domain,
    select_test_family_for_syscall,
    subkey_for_domain,
    suggest_subdir,
)


INTERFACE_LINE_RE = re.compile(
    r"^[+-]\s*(?:pub\s+)?(?:trait\s+\w+|fn\s+\w+\s*\()"
)
HOOK_KEYWORD_RE = re.compile(
    r"\b(revalidate|lookup|validate|cache|coheren|invalidate|resolver|dentry|inode)\b",
    re.IGNORECASE,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Classify Asterinas change inputs into review units.",
    )
    parser.add_argument(
        "paths",
        nargs="*",
        help="Changed file paths when not using --git-range or --patch-file",
    )
    parser.add_argument("--git-range", help="Git revision or revision range to inspect")
    parser.add_argument("--patch-file", help="Path to a unified diff patch file")
    parser.add_argument(
        "--repo",
        help="Repository root for git-based classification. Defaults to current git toplevel or cwd.",
    )
    return parser.parse_args()


def run_git(repo: str, *args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout


def load_inputs(args: argparse.Namespace) -> tuple[list[str], str, str]:
    repo = args.repo or repo_root()
    if args.git_range:
        changed_paths = [
            line.strip()
            for line in run_git(repo, "diff", "--name-only", args.git_range).splitlines()
            if line.strip()
        ]
        patch_text = run_git(repo, "diff", "--unified=0", args.git_range)
        return changed_paths, patch_text, "git-range"

    if args.patch_file:
        patch_text = Path(args.patch_file).read_text(encoding="utf-8")
        changed_paths = []
        for line in patch_text.splitlines():
            if line.startswith("+++ b/"):
                changed_paths.append(line.removeprefix("+++ b/"))
        return sorted(dict.fromkeys(changed_paths)), patch_text, "patch-file"

    return sorted(dict.fromkeys(args.paths)), "", "files"


def detect_interface_paths(changed_paths: list[str], patch_text: str) -> set[str]:
    if not patch_text:
        return set()

    current_file: str | None = None
    interface_paths: set[str] = set()

    for line in patch_text.splitlines():
        if line.startswith("+++ b/"):
            current_file = line.removeprefix("+++ b/")
            continue
        if not current_file:
            continue
        if INTERFACE_LINE_RE.match(line) and HOOK_KEYWORD_RE.search(line):
            interface_paths.add(current_file)
            continue
        if (
            current_file.startswith("kernel/src/fs/vfs/")
            and line.startswith("+")
            and HOOK_KEYWORD_RE.search(line)
            and "trait" in line
        ):
            interface_paths.add(current_file)

    return interface_paths & set(changed_paths)


def build_syscall_unit(path: str) -> dict[str, object]:
    name = Path(path).stem
    recommended_general_test_module = select_test_family_for_syscall(name)
    return {
        "id": f"syscall-{name}",
        "kind": "syscall",
        "subsystem": "syscall",
        "anchor_paths": [path],
        "affected_behaviors": [
            f"Linux-facing `{name}` syscall semantics",
        ],
        "spec_surface": [
            "Linux man-page or documented syscall behavior",
            "Linux syscall implementation and helpers",
        ],
        "validation_surface": (
            "general test + verify"
            if recommended_general_test_module is not None
            else "report-only"
        ),
        "validation_feasible": recommended_general_test_module is not None,
        "recommended_skill": "asterinas-verify",
        "recommended_profile": "syscall",
        "recommended_general_test_module": recommended_general_test_module,
        "candidate_subdir": None,
    }


def build_module_unit(kind: str, domain: str, paths: list[str], subkey: str | None = None) -> dict[str, object]:
    slug = domain.replace("_", "-")
    if subkey:
        suffix = subkey.replace("/", "-")
        slug = f"{slug}-{suffix}"
    behavior = {
        "vfs": "path lookup, cache coherence, and filesystem state visibility",
        "fs-impl": "filesystem implementation behavior and contract conformance",
        "fs": "filesystem semantics visible to userspace",
        "process": "task or process management behavior",
        "network": "socket or networking behavior",
        "device": "device-facing behavior",
        "memory": "memory-management behavior",
        "ipc": "IPC behavior",
        "ostd": "lower-layer contract or invariants",
        "osdk": "tooling and build workflow behavior",
        "tests": "test ownership and coverage",
        "other": "subsystem behavior",
    }.get(domain, "subsystem behavior")
    validation_surface, recommended_general_test_module = build_validation_plan(domain)
    candidate_subdir = suggest_subdir(domain, paths) if recommended_general_test_module else None
    validation_feasible = validation_surface == "general test + verify" and recommended_general_test_module is not None

    return {
        "id": f"{kind}-{slug}",
        "kind": kind,
        "subsystem": domain,
        "anchor_paths": sorted(paths),
        "affected_behaviors": [behavior],
        "spec_surface": [
            "external spec if available",
            "upstream Linux implementation and tests",
            "Asterinas internal contracts and invariants",
        ],
        "validation_surface": validation_surface,
        "validation_feasible": validation_feasible,
        "recommended_skill": "asterinas-verify",
        "recommended_profile": "module",
        "recommended_general_test_module": recommended_general_test_module,
        "candidate_subdir": candidate_subdir,
    }


def classify_units(changed_paths: list[str], patch_text: str) -> list[dict[str, object]]:
    interface_paths = detect_interface_paths(changed_paths, patch_text)
    syscall_units = [
        build_syscall_unit(path)
        for path in changed_paths
        if path.startswith("kernel/src/syscall/") and path.endswith(".rs")
    ]

    domain_groups: dict[tuple[str, str, str], list[str]] = defaultdict(list)
    for path in changed_paths:
        if path.startswith("kernel/src/syscall/") and path.endswith(".rs"):
            continue
        domain = detect_domain(path)
        kind = "feature-interface" if path in interface_paths else "module"
        subkey = subkey_for_domain(domain, path)
        domain_groups[(kind, domain, subkey)].append(path)

    module_units = [
        build_module_unit(kind, domain, paths, subkey=_subkey)
        for (kind, domain, _subkey), paths in sorted(domain_groups.items())
    ]
    return syscall_units + module_units


def main() -> int:
    args = parse_args()
    changed_paths, patch_text, input_mode = load_inputs(args)
    output = {
        "input_mode": input_mode,
        "changed_paths": changed_paths,
        "review_units": classify_units(changed_paths, patch_text),
    }
    print(json.dumps(output, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
