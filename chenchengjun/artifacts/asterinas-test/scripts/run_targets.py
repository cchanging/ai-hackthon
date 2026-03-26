#!/usr/bin/env python3

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import platform
import shutil
import shlex
import subprocess
import sys
import tempfile

from log_utils import (
    extract_failure_excerpt,
    find_probable_failing_command,
    infer_failed_asterinas_target,
    make_timestamp,
    parse_full_log_path,
    print_failure_excerpt,
    sanitize_label,
)
from target_utils import DEFAULT_BUILD_ROOT, DEFAULT_LOG_DIR, DEFAULT_REPO, APPS_DIR, normalize_and_check_targets

BASE_INITRAMFS_DIR = Path("test/initramfs/build/initramfs")
TARGETED_INIT_SCRIPT = Path("/test/.codex/run-targets.sh")


@dataclass
class LinuxTargetResult:
    target_name: str
    build_target: str
    passed: bool
    stage: str
    exit_code: int
    log_path: Path
    failing_command: str | None = None
    excerpt: list[str] | None = None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="List, run, or verify Asterinas general test targets through one entry point.",
    )
    parser.add_argument(
        "--repo",
        type=Path,
        default=DEFAULT_REPO,
        help=f"Repository root. Defaults to {DEFAULT_REPO}.",
    )
    parser.add_argument(
        "--log-dir",
        type=Path,
        default=DEFAULT_LOG_DIR,
        help=f"Directory for saved logs. Defaults to {DEFAULT_LOG_DIR}.",
    )
    parser.add_argument(
        "--build-root",
        type=Path,
        default=DEFAULT_BUILD_ROOT,
        help=f"Directory that stores temporary Linux build output. Defaults to {DEFAULT_BUILD_ROOT}.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("list", help="Print the available general test targets.")

    run_parser = subparsers.add_parser("run", help="Run one or more general test targets on one platform.")
    run_parser.add_argument(
        "--platform",
        choices=("asterinas", "linux"),
        required=True,
        help="Execution platform for this run.",
    )
    run_parser.add_argument("target_names", nargs="+", help="General test targets without the /test/ prefix.")

    verify_parser = subparsers.add_parser(
        "verify",
        help="Run one or more general test targets on host Linux, then on Asterinas if Linux passes.",
    )
    verify_parser.add_argument("target_names", nargs="+", help="General test targets without the /test/ prefix.")
    return parser.parse_args()


def build_asterinas_initramfs_command() -> list[str]:
    return [
        "make",
        "--no-print-directory",
        "initramfs",
        "ENABLE_BASIC_TEST=true",
        "INITRAMFS_SKIP_GZIP=1",
    ]


def build_asterinas_command(repo: Path, initramfs_path: Path) -> list[str]:
    resolve_command = [
        "make",
        "-n",
        "run_kernel",
        "AUTO_TEST=none",
        "CONSOLE=ttyS0",
        "INITRAMFS_SKIP_GZIP=1",
        "ENABLE_BASIC_TEST=true",
    ]
    try:
        result = subprocess.run(
            resolve_command,
            cwd=repo,
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
    except FileNotFoundError as error:
        raise RuntimeError(f"Failed to resolve the Asterinas run command: {error}") from error
    except subprocess.CalledProcessError as error:
        message = error.stderr.strip() or error.stdout.strip() or str(error)
        raise RuntimeError(f"Failed to resolve the Asterinas run command: {message}") from error

    cargo_command: list[str] | None = None
    for line in result.stdout.splitlines():
        stripped = line.strip()
        if stripped.startswith("cd kernel && cargo osdk run "):
            cargo_command = shlex.split(stripped.removeprefix("cd kernel && "))
            break

    if cargo_command is None:
        raise RuntimeError("Failed to resolve the Asterinas run command from `make -n run_kernel`.")

    cargo_command = [arg for arg in cargo_command if not arg.startswith("--initramfs=")]
    cargo_command = [arg for arg in cargo_command if not arg.startswith("--init-args=")]
    cargo_command.append(f"--initramfs={initramfs_path}")
    cargo_command.append(f"--init-args={TARGETED_INIT_SCRIPT}")
    return cargo_command


def make_asterinas_log_path(log_dir: Path, target_names: list[str]) -> Path:
    return log_dir / f"asterinas-run-test-{sanitize_label('-'.join(target_names))}-{make_timestamp()}.log"


def make_phase_log_path(log_dir: Path, target_names: list[str], phase: str) -> Path:
    return log_dir / (
        f"asterinas-validate-linux-behavior-{sanitize_label('-'.join(target_names))}-{phase}-{make_timestamp()}.log"
    )


def make_target_build_dir(build_root: Path, target_name: str, timestamp: str) -> Path:
    return build_root / f"asterinas-validate-linux-behavior-{sanitize_label(target_name)}-{timestamp}"


def append_log_message(log_path: Path, message: str) -> None:
    with log_path.open("a", encoding="utf-8", errors="replace") as log_file:
        log_file.write(message)
        if not message.endswith("\n"):
            log_file.write("\n")


def build_targeted_init_script(target_names: list[str]) -> str:
    lines = [
        "#!/bin/sh",
        "",
        "# Generated by the asterinas-test skill.",
        "set -e",
        "",
        'run_target() {',
        '    target_name="$1"',
        '    target_path="/test/${target_name}"',
        "",
        '    if [ -d "${target_path}" ] && [ -x "${target_path}/run_test.sh" ]; then',
        '        echo "Running general test target ${target_name} via ${target_path}/run_test.sh"',
        '        (cd "${target_path}" && ./run_test.sh)',
        '        echo "General test target ${target_name} passed."',
        "        return 0",
        "    fi",
        "",
        '    if [ -f "${target_path}" ]; then',
        '        echo "Running general test target ${target_name}"',
        '        if [ "${target_path##*.}" = "sh" ]; then',
        '            (cd "$(dirname "${target_path}")" && /bin/sh -ex "./$(basename "${target_path}")")',
        "        else",
        '            (cd "$(dirname "${target_path}")" && "./$(basename "${target_path}")")',
        "        fi",
        '        echo "General test target ${target_name} passed."',
        "        return 0",
        "    fi",
        "",
        '    echo "Unknown general test target: ${target_name}" >&2',
        "    return 1",
        "}",
        "",
    ]
    lines.extend(f"run_target {shlex.quote(target_name)}" for target_name in target_names)
    lines.extend(
        [
            "",
            'echo "All requested general tests passed."',
        ]
    )
    return "\n".join(lines) + "\n"


def prepare_targeted_initramfs(
    repo: Path,
    target_names: list[str],
    log_path: Path,
) -> tuple[Path, Path]:
    base_initramfs_dir = repo / BASE_INITRAMFS_DIR
    if not base_initramfs_dir.is_dir():
        raise RuntimeError(f"Base initramfs directory not found: {base_initramfs_dir}")

    workspace_dir = Path(tempfile.mkdtemp(prefix="asterinas-test-initramfs-"))
    staging_dir = workspace_dir / "rootfs"
    custom_initramfs_path = workspace_dir / "initramfs.cpio"
    shutil.copytree(base_initramfs_dir, staging_dir, symlinks=True)

    init_script_path = staging_dir / TARGETED_INIT_SCRIPT.relative_to("/")
    init_script_path.parent.mkdir(parents=True, exist_ok=True)
    init_script_path.write_text(build_targeted_init_script(target_names), encoding="utf-8")
    init_script_path.chmod(0o755)

    pack_command = [
        "bash",
        "-lc",
        f"cd {shlex.quote(str(staging_dir))} && find . -print0 | cpio -o -H newc --null > {shlex.quote(str(custom_initramfs_path))}",
    ]
    append_log_message(log_path, f"Custom initramfs pack command: {shlex.join(pack_command)}")
    pack_exit_code, _ = stream_command(pack_command, repo, log_path, log_mode="a")
    if pack_exit_code != 0:
        raise RuntimeError(f"Failed to build custom initramfs image: {custom_initramfs_path}")

    return custom_initramfs_path, workspace_dir


def stream_command(
    command: list[str],
    cwd: Path,
    log_path: Path,
    log_mode: str = "w",
    env: dict[str, str] | None = None,
) -> tuple[int, list[str]]:
    captured_lines: list[str] = []
    process: subprocess.Popen[str] | None = None

    with log_path.open(log_mode, encoding="utf-8", errors="replace") as log_file:
        try:
            process = subprocess.Popen(
                command,
                cwd=cwd,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                encoding="utf-8",
                errors="replace",
                bufsize=1,
            )
        except FileNotFoundError as error:
            raise RuntimeError(f"Failed to launch command: {error}") from error

        assert process.stdout is not None
        try:
            for line in process.stdout:
                sys.stdout.write(line)
                log_file.write(line)
                captured_lines.append(line.rstrip("\n"))
        except KeyboardInterrupt:
            if process.poll() is None:
                process.terminate()
            raise

        return process.wait(), captured_lines


def list_command(repo: Path) -> int:
    from target_utils import get_available_targets

    try:
        targets = get_available_targets(repo)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1

    print("Available general test targets:")
    for target in targets:
        print(target)
    return 0


def run_asterinas_targets(target_names: list[str], repo: Path, log_dir: Path) -> tuple[int, list[str], Path]:
    log_dir.mkdir(parents=True, exist_ok=True)
    log_path = make_asterinas_log_path(log_dir, target_names)
    lines: list[str] = []
    initramfs_workspace: Path | None = None

    initramfs_command = build_asterinas_initramfs_command()
    print(f"Repository: {repo}")
    print(f"Preparing initramfs: {shlex.join(initramfs_command)}")
    print(f"Full log: {log_path}")
    append_log_message(log_path, f"Initramfs command: {shlex.join(initramfs_command)}")

    initramfs_exit_code, initramfs_lines = stream_command(initramfs_command, repo, log_path)
    lines.extend(initramfs_lines)
    if initramfs_exit_code != 0:
        return initramfs_exit_code or 1, lines, log_path

    try:
        custom_initramfs_path, initramfs_workspace = prepare_targeted_initramfs(repo, target_names, log_path)
    except RuntimeError as error:
        print(error, file=sys.stderr)
        append_log_message(log_path, str(error))
        lines.append(str(error))
        return 1, lines, log_path

    command = build_asterinas_command(repo, custom_initramfs_path)
    print(f"Command: {shlex.join(command)}")
    append_log_message(log_path, f"Run command: {shlex.join(command)}")

    try:
        exit_code, run_lines = stream_command(command, repo / "kernel", log_path, log_mode="a")
        lines.extend(run_lines)
        return exit_code, lines, log_path
    finally:
        if initramfs_workspace is not None:
            shutil.rmtree(initramfs_workspace, ignore_errors=True)


def run_asterinas_command(target_names: list[str], repo: Path, log_dir: Path) -> int:
    try:
        normalized_targets = normalize_and_check_targets(target_names, repo)
    except RuntimeError as error:
        print(error, file=sys.stderr)
        return 1

    exit_code, lines, log_path = run_asterinas_targets(normalized_targets, repo, log_dir)
    if exit_code == 0:
        print("\nRun result: success on Asterinas.")
        print(f"Full log: {log_path}")
        return 0

    print("\nRun result: failure on Asterinas.")
    print("Failure excerpt:")
    print_failure_excerpt(extract_failure_excerpt(lines))
    print(f"Full log: {log_path}")
    return exit_code or 1


def run_linux_target(target_name: str, repo: Path, build_root: Path, log_dir: Path) -> LinuxTargetResult:
    timestamp = make_timestamp()
    build_target = target_name.split("/", 1)[0]
    build_dir = make_target_build_dir(build_root, target_name, timestamp)
    log_path = make_phase_log_path(log_dir, [target_name], "linux")
    host_platform = f"{platform.machine()}-linux"

    build_command = [
        "make",
        "--no-print-directory",
        "-C",
        str(repo / APPS_DIR),
        f"BUILD_DIR={build_dir}",
        "TEST_PLATFORM=linux",
        f"HOST_PLATFORM={host_platform}",
        build_target,
    ]

    print(f"\n=== Linux phase: {target_name} ===")
    print(f"Build target: {build_target}")
    print(f"Build directory: {build_dir}")
    print(f"Linux log: {log_path}")
    print(f"Build command: {shlex.join(build_command)}")

    build_exit_code, build_lines = stream_command(build_command, repo, log_path)
    if build_exit_code != 0:
        return LinuxTargetResult(
            target_name=target_name,
            build_target=build_target,
            passed=False,
            stage="linux-build",
            exit_code=build_exit_code or 1,
            log_path=log_path,
            failing_command=find_probable_failing_command(build_lines),
            excerpt=extract_failure_excerpt(build_lines),
        )

    built_target_path = build_dir / "initramfs/test" / target_name
    if built_target_path.is_dir() and (built_target_path / "run_test.sh").is_file():
        run_dir = built_target_path
        run_command = ["/bin/sh", "-ex", "./run_test.sh"]
    elif built_target_path.is_file():
        run_dir = built_target_path.parent
        if built_target_path.suffix == ".sh":
            run_command = ["/bin/sh", "-ex", f"./{built_target_path.name}"]
        else:
            run_command = [f"./{built_target_path.name}"]
    else:
        message = f"Missing built general test target: {built_target_path}"
        with log_path.open("a", encoding="utf-8", errors="replace") as log_file:
            print(message, file=log_file)
        return LinuxTargetResult(
            target_name=target_name,
            build_target=build_target,
            passed=False,
            stage="linux-run",
            exit_code=1,
            log_path=log_path,
            excerpt=[message],
        )

    with log_path.open("a", encoding="utf-8", errors="replace") as log_file:
        print("\n=== Run command ===", file=log_file)
        print(shlex.join(run_command), file=log_file)

    print(f"Run command: {shlex.join(run_command)}")
    run_exit_code, run_lines = stream_command(run_command, run_dir, log_path, log_mode="a")
    if run_exit_code == 0:
        return LinuxTargetResult(
            target_name=target_name,
            build_target=build_target,
            passed=True,
            stage="linux-run",
            exit_code=0,
            log_path=log_path,
        )

    return LinuxTargetResult(
        target_name=target_name,
        build_target=build_target,
        passed=False,
        stage="linux-run",
        exit_code=run_exit_code or 1,
        log_path=log_path,
        failing_command=find_probable_failing_command(run_lines),
        excerpt=extract_failure_excerpt(run_lines),
    )


def run_linux_phase(target_names: list[str], repo: Path, log_dir: Path, build_root: Path) -> tuple[int, list[LinuxTargetResult]]:
    normalized_targets = normalize_and_check_targets(target_names, repo)

    log_dir.mkdir(parents=True, exist_ok=True)
    build_root.mkdir(parents=True, exist_ok=True)

    linux_results = [
        run_linux_target(target_name, repo=repo, build_root=build_root, log_dir=log_dir)
        for target_name in normalized_targets
    ]
    linux_failures = [result for result in linux_results if not result.passed]
    if linux_failures:
        print("\nRun result: test program issue on Linux.")
        print("Requested targets failed before the Asterinas phase:")
        for failure in linux_failures:
            print(f"- Target: {failure.target_name}")
            print(f"  Build target: {failure.build_target}")
            print(f"  Stage: {failure.stage}")
            print(f"  Exit code: {failure.exit_code}")
            if failure.failing_command is not None:
                print(f"  Probable failing command: {failure.failing_command}")
            print(f"  Linux log: {failure.log_path}")
            print("  Failure excerpt:")
            print_failure_excerpt(failure.excerpt or [])
        return 1, linux_results

    print("\nLinux phase passed for all requested targets.")
    for result in linux_results:
        print(f"- {result.target_name}: {result.log_path}")
    return 0, linux_results


def run_linux_command(target_names: list[str], repo: Path, log_dir: Path, build_root: Path) -> int:
    try:
        linux_exit_code, linux_results = run_linux_phase(target_names, repo, log_dir, build_root)
    except RuntimeError as error:
        print(error, file=sys.stderr)
        return 1

    if linux_exit_code != 0:
        return linux_exit_code

    print("\nRun result: success on host Linux.")
    for result in linux_results:
        print(f"Linux log for {result.target_name}: {result.log_path}")
    return 0


def verify_command(target_names: list[str], repo: Path, log_dir: Path, build_root: Path) -> int:
    try:
        linux_exit_code, linux_results = run_linux_phase(target_names, repo, log_dir, build_root)
    except RuntimeError as error:
        print(error, file=sys.stderr)
        return 1

    if linux_exit_code != 0:
        return linux_exit_code

    normalized_targets = [result.target_name for result in linux_results]
    asterinas_exit_code, asterinas_lines, asterinas_log_path = run_asterinas_targets(normalized_targets, repo, log_dir)
    if asterinas_exit_code == 0:
        print("\nVerify result: success.")
        for result in linux_results:
            print(f"Linux log for {result.target_name}: {result.log_path}")
        print(f"Asterinas log: {asterinas_log_path}")
        return 0

    print("\nVerify result: behavior mismatch on Asterinas.")
    failed_target = infer_failed_asterinas_target(asterinas_lines)
    if failed_target is not None:
        print(f"Probable failing target: {failed_target}")
    for result in linux_results:
        print(f"Linux log for {result.target_name}: {result.log_path}")
    if asterinas_log_path is None:
        asterinas_log_path = parse_full_log_path(asterinas_lines)
    if asterinas_log_path is not None:
        print(f"Asterinas log: {asterinas_log_path}")
    print("Asterinas failure excerpt:")
    print_failure_excerpt(extract_failure_excerpt(asterinas_lines))
    return asterinas_exit_code or 1


def main() -> int:
    args = parse_args()
    if args.command == "list":
        return list_command(args.repo)
    if args.command == "run":
        if args.platform == "asterinas":
            return run_asterinas_command(args.target_names, args.repo, args.log_dir)
        return run_linux_command(args.target_names, args.repo, args.log_dir, args.build_root)
    return verify_command(args.target_names, args.repo, args.log_dir, args.build_root)


if __name__ == "__main__":
    raise SystemExit(main())
