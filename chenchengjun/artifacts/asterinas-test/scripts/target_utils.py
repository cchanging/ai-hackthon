#!/usr/bin/env python3

from __future__ import annotations

from pathlib import Path
import re


DEFAULT_REPO = Path("/root/asterinas")
DEFAULT_LOG_DIR = Path("/tmp")
DEFAULT_BUILD_ROOT = Path("/tmp")
APPS_DIR = Path("test/initramfs/src/apps")
TARGET_SEGMENT_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._@+-]*$")


def get_available_targets(repo: Path) -> list[str]:
    apps_dir = repo / APPS_DIR
    if not apps_dir.is_dir():
        raise ValueError(f"Apps directory not found: {apps_dir}")

    targets: set[str] = set()
    for path in apps_dir.rglob("*"):
        relative_path = path.relative_to(apps_dir)
        relative_text = relative_path.as_posix()
        parts = relative_path.parts

        if not parts or parts[0] in {"common", "scripts"}:
            continue

        if path.is_dir() and (path / "run_test.sh").is_file() and (path / "run_test.sh").stat().st_mode & 0o111:
            targets.add(relative_text)
            continue

        if not path.is_file():
            continue

        suffix = path.suffix
        if suffix in {".c", ".S"}:
            targets.add(relative_path.with_suffix("").as_posix())
            continue

        if suffix == ".sh" and path.name != "run_test.sh" and path.stat().st_mode & 0o111:
            targets.add(relative_text)

    targets = sorted(targets)
    if not targets:
        raise ValueError(f"No general test targets found under: {apps_dir}")
    return targets


def normalize_target_name(raw_target_name: str) -> str:
    target_name = raw_target_name.strip()
    for prefix in ("/root/asterinas/test/initramfs/src/apps/", "test/initramfs/src/apps/", "/test/"):
        if target_name.startswith(prefix):
            target_name = target_name[len(prefix) :]
            break

    if target_name.endswith("/run_test.sh"):
        target_name = target_name[: -len("/run_test.sh")]
    elif target_name.endswith(".c") or target_name.endswith(".S"):
        target_name = target_name.rsplit(".", 1)[0]

    target_name = target_name.rstrip("/")
    if not target_name:
        raise ValueError("target name must not be empty")
    if any(character.isspace() for character in target_name):
        raise ValueError("target name must not contain whitespace")
    if target_name.startswith("/"):
        raise ValueError("target name must not start with '/' unless it uses a supported prefix")

    parts = target_name.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        raise ValueError("target name must not contain empty, '.' or '..' path segments")
    if not all(TARGET_SEGMENT_RE.fullmatch(part) for part in parts):
        raise ValueError(
            "target name contains unsupported characters; use path segments made of letters, digits, '.', '_', '@', '+', or '-'"
        )
    return target_name


def normalize_and_check_targets(target_names: list[str], repo: Path) -> list[str]:
    if not repo.is_dir():
        raise RuntimeError(f"Repository not found: {repo}")

    try:
        available_targets = set(get_available_targets(repo))
    except ValueError as error:
        raise RuntimeError(str(error)) from error

    normalized_targets: list[str] = []
    for raw_target_name in target_names:
        try:
            target_name = normalize_target_name(raw_target_name)
        except ValueError as error:
            raise RuntimeError(f"Invalid general test target `{raw_target_name}`: {error}") from error
        if target_name not in available_targets:
            available_targets_text = ", ".join(sorted(available_targets))
            raise RuntimeError(
                f"Unknown general test target: {target_name}. Available targets: {available_targets_text}"
            )
        normalized_targets.append(target_name)

    return normalized_targets
