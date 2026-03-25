#!/usr/bin/env python3

from __future__ import annotations

import subprocess
from pathlib import Path


def repo_root(start: Path | None = None) -> Path:
    """Return the repository root, preferring `git rev-parse` when available.

    Falls back to the provided `start` directory (or the current working
    directory) when git metadata is absent.
    """

    base = (start or Path.cwd()).resolve()

    try:
        output = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=base,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        if output:
            return Path(output).resolve()
    except Exception:
        pass

    return base
