#!/usr/bin/env python3

from __future__ import annotations

from datetime import UTC, datetime
from pathlib import Path
import re
import sys


FAILURE_RE = re.compile(
    r"(?i)(?:"
    r"panic|panicked|fail(?:ed|ure)?|error|assert|segfault|"
    r"tim(?:e)?out|qemu:|backtrace|bug:|killed|exception|fatal|"
    r"not found|no such file|permission denied"
    r")"
)
TRACE_COMMAND_RE = re.compile(r"^\+\s+(.+)$")
STARTED_TARGET_RE = re.compile(r"Running general test target ([A-Za-z0-9._@+-]+(?:/[A-Za-z0-9._@+-]+)*)")
PASSED_TARGET_RE = re.compile(r"General test target ([A-Za-z0-9._@+-]+(?:/[A-Za-z0-9._@+-]+)*) passed\.")
FULL_LOG_RE = re.compile(r"^Full log:\s+(.+)$")


def make_timestamp() -> str:
    return datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")


def sanitize_label(label: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "-", label).strip("-") or "test"


def merge_ranges(ranges: list[tuple[int, int]]) -> list[tuple[int, int]]:
    if not ranges:
        return []
    merged = [ranges[0]]
    for start, end in ranges[1:]:
        last_start, last_end = merged[-1]
        if start <= last_end:
            merged[-1] = (last_start, max(last_end, end))
            continue
        merged.append((start, end))
    return merged


def extract_failure_excerpt(lines: list[str], context_lines: int = 4, max_lines: int = 80) -> list[str]:
    matching_ranges: list[tuple[int, int]] = []
    for index, line in enumerate(lines):
        if FAILURE_RE.search(line):
            start = max(0, index - context_lines)
            end = min(len(lines), index + context_lines + 1)
            matching_ranges.append((start, end))

    if not matching_ranges:
        return lines[-max_lines:]

    excerpt: list[str] = []
    for range_index, (start, end) in enumerate(merge_ranges(matching_ranges)):
        if range_index > 0:
            excerpt.append("---")
        excerpt.extend(lines[start:end])

    if len(excerpt) <= max_lines:
        return excerpt
    return excerpt[-max_lines:]


def print_failure_excerpt(excerpt: list[str]) -> None:
    if not excerpt:
        print("(no log lines captured)")
        return
    for line in excerpt:
        print(line)


def print_failure_summary(lines: list[str]) -> None:
    excerpt = extract_failure_excerpt(lines)
    print("\n=== Failure summary ===")
    print_failure_excerpt(excerpt)


def find_probable_failing_command(lines: list[str]) -> str | None:
    traced_commands = []
    for line in lines:
        match = TRACE_COMMAND_RE.match(line)
        if not match:
            continue
        command = match.group(1).strip()
        if not command or command == "set -e":
            continue
        traced_commands.append(command)

    for command in reversed(traced_commands):
        if command.startswith("./"):
            return command
    return traced_commands[-1] if traced_commands else None


def infer_failed_asterinas_target(lines: list[str]) -> str | None:
    started_targets = [match.group(1) for line in lines if (match := STARTED_TARGET_RE.search(line))]
    passed_targets = {match.group(1) for line in lines if (match := PASSED_TARGET_RE.search(line))}
    for target_name in started_targets:
        if target_name not in passed_targets:
            return target_name
    return None


def parse_full_log_path(lines: list[str]) -> Path | None:
    for line in reversed(lines):
        match = FULL_LOG_RE.match(line)
        if match:
            return Path(match.group(1).strip())
    return None


def summarize_log_file(log_path: Path) -> int:
    if not log_path.is_file():
        print(f"Log file not found: {log_path}", file=sys.stderr)
        return 1
    lines = log_path.read_text(encoding="utf-8", errors="replace").splitlines()
    print_failure_summary(lines)
    print(f"\nFull log: {log_path}")
    return 0
