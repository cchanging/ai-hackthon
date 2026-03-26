#!/usr/bin/env python3

from __future__ import annotations

import argparse
from datetime import date
from pathlib import Path

from paths import repo_root


CHANGE_TEMPLATE = """# `{title}` Change Review

## Executive Summary

- Review slug: `{slug}`
- Review date: `{review_date}`
- Reviewer: `<agent>`
- Input mode: `<git-range | patch-file | files | goal>`
- Validation plan: `<general test + verify | report-only>`
- User-space input focus: `<list relevant corner cases or n/a>`
- Status: `<draft | in-progress | done>`

## Findings (cards; order by severity)

- None recorded yet.

Each finding should link to evidence IDs from the appendix, list anchors as `<repo-relative-path>:<line?>`, mention relevant `input_cases`, and state whether the test intent is `regression` or `confirmation`.

## User-space Input Corner Cases

- None recorded yet.

## Changed Paths

- None recorded yet.

## Review Units

| Unit | Kind | Anchor Paths | Validation Plan | Routed Profile |
|------|------|--------------|-----------------|----------------|

## Evidence Appendix

| ID | Source (`spec, linux-derived, asterinas-contract, diff-intent`) | Path | Note | Supports (Finding IDs) |
|----|--------------------------------------------------------------|------|------|-------------------------|

## Validation Log

- Planned targets: `<list planned general-test modules or report-only>`
- Confirmation tests used to resolve uncertainty: `<none or list>`
- Commands run: `<cmd1>`, `<cmd2>`
- Outcomes: `<pass/fail + short notes>`

## Candidate Regression Tests

- None recorded yet.

## Implemented Tests

- None recorded yet.

## Open Questions

- None recorded yet.
"""


MODULE_TEMPLATE = """# `{title}` Module Review

## Executive Summary

- Review slug: `{slug}`
- Review date: `{review_date}`
- Reviewer: `<agent>`
- Anchor paths: `<path>`
- Validation plan: `<general test + verify | report-only>`
- User-space input focus: `<list relevant corner cases or n/a>`
- Status: `<draft | in-progress | done>`

## Findings (cards; order by severity)

- None recorded yet.

Each finding should link to evidence IDs from the appendix, list anchors as `<repo-relative-path>:<line?>`, mention relevant `input_cases`, and state whether the test intent is `regression` or `confirmation`.

## User-space Input Corner Cases

- None recorded yet.

## Asterinas Implementation Notes

- Entry path:
- Relevant helpers:
- Contract boundary:
- Error propagation:
- Locking or blocking points:
- Cache or lifetime implications:
- Explicit TODOs or unimplemented paths:

## Evidence Appendix

| ID | Source (`spec, linux-derived, asterinas-contract, diff-intent`) | Path | Note | Supports (Finding IDs) |
|----|--------------------------------------------------------------|------|------|-------------------------|

## Validation Log

- Planned targets: `<list planned general-test modules or report-only>`
- Confirmation tests used to resolve uncertainty: `<none or list>`
- Commands run: `<cmd1>`, `<cmd2>`
- Outcomes: `<pass/fail + short notes>`

## Candidate Regression Tests

- None recorded yet.

## Implemented Tests

- None recorded yet.

## Open Questions

- None recorded yet.
"""


SYSCALL_TEMPLATE = """# `{name}` Syscall Review

## Executive Summary

- Syscall: `{name}`
- Asterinas entry: `{root}/kernel/src/syscall/{name}.rs`
- Review date: `{review_date}`
- Reviewer: `<agent>`
- Validation plan: `<general test + verify | report-only>`
- User-space input focus: `<list relevant corner cases or n/a>`
- Status: `<draft | in-progress | done>`

## Findings (cards; order by severity)

- None recorded yet.

Each finding should link to evidence IDs from the appendix, list anchors as `<repo-relative-path>:<line?>`, mention relevant `input_cases`, and state whether the test intent is `regression` or `confirmation`.

## Sources

| Source Type | Location | Notes |
|-------------|----------|-------|
| manual | `<url-or-path>` | `<why it matters>` |
| linux | `<url-or-path>` | `<entry or helper>` |
| asterinas | `{root}/kernel/src/syscall/{name}.rs` | syscall entry |

## User-space Input Corner Cases

- Pointers / buffer validity:
- Zero-length / empty inputs:
- Boundary sizes / truncation:
- Flag combinations / reserved bits:
- Partial success / retry / state transitions:

## Linux Semantics Matrix (keep tight and reference evidence IDs)

| Case | Inputs / Preconditions | Linux Semantics | Evidence IDs | Notes |
|------|------------------------|-----------------|--------------|-------|

## Asterinas Implementation Notes

- Entry path:
- Relevant helpers:
- Argument validation:
- Return / errno behavior:
- Side effects:
- Explicit TODOs or unimplemented paths:

## Comparison Matrix (link to evidence IDs)

| Case | Inputs / Preconditions | Linux Semantics | Asterinas Status | Class | Evidence IDs | Test Status |
|------|------------------------|-----------------|------------------|-------|--------------|-------------|

## Evidence Appendix

| ID | Source (`spec|linux-derived|asterinas-contract|diff-intent`) | Path | Note | Supports (Finding IDs) |
|----|--------------------------------------------------------------|------|------|-------------------------|

## Validation Log

- Planned targets: `<list planned general-test modules or report-only>`
- Confirmation tests used to resolve uncertainty: `<none or list>`
- Commands run: `<cmd1>`, `<cmd2>`
- Outcomes: `<pass/fail + short notes>`

## Candidate Regression Tests

- None recorded yet.

## Implemented Tests

- None recorded yet.

## Open Questions

- None recorded yet.
"""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Initialize an Asterinas review report.",
    )
    subparsers = parser.add_subparsers(dest="mode", required=True)

    change_parser = subparsers.add_parser("change", help="Initialize a change review report")
    change_parser.add_argument("slug", help="Review slug, such as vfs-revalidate")
    change_parser.add_argument("--title", help="Human-readable title. Defaults to the slug.")
    change_parser.add_argument(
        "--output",
        help="Output path for the report. Defaults to <repo>/change-review/<slug>.md",
    )

    module_parser = subparsers.add_parser("module", help="Initialize a module review report")
    module_parser.add_argument("slug", help="Review slug, such as vfs-revalidate")
    module_parser.add_argument("--title", help="Human-readable title. Defaults to the slug.")
    module_parser.add_argument(
        "--output",
        help="Output path for the report. Defaults to <repo>/change-review/<slug>.md",
    )

    syscall_parser = subparsers.add_parser("syscall", help="Initialize a syscall review report")
    syscall_parser.add_argument("name", help="Syscall name, such as readlink or mmap")
    syscall_parser.add_argument(
        "--output",
        help="Output path for the report. Defaults to <repo>/syscall-review/<name>.md",
    )

    return parser.parse_args()


def write_report(output: Path, content: str) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    print(output)


def main() -> int:
    args = parse_args()
    review_date = date.today().isoformat()
    root = repo_root()
    if args.mode == "change":
        output = Path(args.output) if args.output else root / "change-review" / f"{args.slug}.md"
        write_report(
            output,
            CHANGE_TEMPLATE.format(
                slug=args.slug,
                title=args.title or args.slug,
                review_date=review_date,
            ),
        )
        return 0

    if args.mode == "module":
        output = Path(args.output) if args.output else root / "change-review" / f"{args.slug}.md"
        write_report(
            output,
            MODULE_TEMPLATE.format(
                slug=args.slug,
                title=args.title or args.slug,
                review_date=review_date,
            ),
        )
        return 0

    output = Path(args.output) if args.output else root / "syscall-review" / f"{args.name}.md"
    write_report(
        output,
        SYSCALL_TEMPLATE.format(
            name=args.name,
            root=root,
            review_date=review_date,
        ),
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
