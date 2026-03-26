#!/usr/bin/env python3

from __future__ import annotations

from pathlib import Path


DOMAIN_PREFIXES = (
    ("kernel/src/fs/vfs/", "vfs"),
    ("kernel/src/fs/fs_impls/", "fs-impl"),
    ("kernel/src/fs/", "fs"),
    ("kernel/src/process/", "process"),
    ("kernel/src/net/", "network"),
    ("kernel/src/device/", "device"),
    ("kernel/src/vm/", "memory"),
    ("kernel/src/mm/", "memory"),
    ("kernel/src/ipc/", "ipc"),
    ("kernel/src/security/", "security"),
    ("kernel/src/syscall/", "syscall"),
    ("ostd/src/", "ostd"),
    ("osdk/", "osdk"),
    ("test/initramfs/src/apps/", "tests"),
)

# Heuristics for sub-unit bucketing to avoid giant mixed module units.
SUBKEY_HIERARCHY = {
    "fs-impl": lambda path: Path(path).parts[:4],  # kernel/src/fs/fs_impls/<fs>/
    "fs": lambda path: Path(path).parts[:4],       # kernel/src/fs/<area>/
    "vfs": lambda path: Path(path).parts[:5],      # kernel/src/fs/vfs/<area>/
    "process": lambda path: Path(path).parts[:4],  # kernel/src/process/<area>/
    "network": lambda path: Path(path).parts[:4],  # kernel/src/net/<area>/
    "memory": lambda path: Path(path).parts[:4],   # kernel/src/vm|mm/<area>/
    "ipc": lambda path: Path(path).parts[:4],
    "device": lambda path: Path(path).parts[:4],
}

GENERAL_TEST_MODULES = {
    "fs": "fs",
    "vfs": "fs",
    "fs-impl": "fs",
    "process": "process",
    "network": "network",
    "device": "device",
    "memory": "memory",
    "ipc": "ipc",
    "security": "security",
}

SYSCALL_MODULE_MAP = {
    "fs": {
        "access",
        "chmod",
        "chown",
        "faccessat",
        "fadvise64",
        "fdatasync",
        "flock",
        "fsync",
        "ftruncate",
        "getcwd",
        "getdents64",
        "getxattr",
        "inotify",
        "link",
        "listxattr",
        "lseek",
        "mkdir",
        "mknod",
        "mount",
        "open",
        "openat",
        "readlink",
        "removexattr",
        "rename",
        "rmdir",
        "setxattr",
        "stat",
        "statfs",
        "statx",
        "symlink",
        "truncate",
        "umount",
        "unlink",
        "utimens",
    },
    "process": {
        "alarm",
        "arch_prctl",
        "clone",
        "execve",
        "exit",
        "exit_group",
        "fork",
        "getcpu",
        "getpgid",
        "getpgrp",
        "getpid",
        "getppid",
        "get_priority",
        "getrusage",
        "getsid",
        "gettid",
        "kill",
        "pause",
        "pidfd_open",
        "pidfd_send_signal",
        "prctl",
        "prlimit64",
        "sched_affinity",
        "sched_get_priority_max",
        "sched_get_priority_min",
        "sched_getattr",
        "sched_getparam",
        "sched_getscheduler",
        "sched_setattr",
        "sched_setparam",
        "sched_setscheduler",
        "sched_yield",
        "set_priority",
        "set_tid_address",
        "setitimer",
        "setsid",
        "sigaltstack",
        "tgkill",
        "time",
        "timer_create",
        "timer_settime",
        "wait4",
        "waitid",
    },
    "memory": {
        "brk",
        "madvise",
        "mmap",
        "mprotect",
        "mremap",
        "msync",
        "munmap",
    },
    "ipc": {
        "eventfd",
        "futex",
        "pipe",
        "semctl",
        "semget",
        "semop",
        "set_robust_list",
        "shm",
        "signalfd",
    },
    "network": {
        "accept",
        "bind",
        "connect",
        "getpeername",
        "getsockopt",
        "listen",
        "poll",
        "ppoll",
        "pselect6",
        "recvfrom",
        "recvmsg",
        "sendfile",
        "sendmmsg",
        "sendmsg",
        "sendto",
        "setsockopt",
        "shutdown",
        "socket",
        "socketpair",
    },
    "security": {
        "capget",
        "capset",
        "chroot",
        "getegid",
        "geteuid",
        "getgid",
        "getgroups",
        "getresgid",
        "getresuid",
        "getuid",
        "setdomainname",
        "setfsuid",
        "setfsgid",
        "setgid",
        "setgroups",
        "sethostname",
        "setns",
        "setpgid",
        "setregid",
        "setresgid",
        "setresuid",
        "setreuid",
        "setuid",
        "unshare",
    },
    "device": {
        "getrandom",
        "ioctl",
        "reboot",
        "sysinfo",
        "uname",
    },
    "io": {
        "close",
        "dup",
        "epoll",
        "fcntl",
        "gettimeofday",
        "pread64",
        "preadv",
        "pwrite64",
        "pwritev",
        "read",
        "sync",
        "write",
    },
}


def normalize_syscall_name(name: str) -> str:
    normalized_name = name.removesuffix(".rs").split("/")[-1]
    if normalized_name.startswith("sys_"):
        return normalized_name[4:]
    return normalized_name


def select_test_family_for_syscall(name: str) -> str | None:
    normalized_name = normalize_syscall_name(name)
    for module, syscalls in SYSCALL_MODULE_MAP.items():
        if normalized_name in syscalls:
            return module
    return None


def detect_domain(path: str) -> str:
    for prefix, domain in DOMAIN_PREFIXES:
        if path.startswith(prefix):
            return domain
    return "other"


def detect_domain_from_paths(paths: list[str]) -> str:
    counts: dict[str, int] = {}
    for path in paths:
        domain = detect_domain(path)
        counts[domain] = counts.get(domain, 0) + 1
    if not counts:
        return "other"
    return sorted(counts.items(), key=lambda item: (-item[1], item[0]))[0][0]


def suggest_subdir(domain: str, paths: list[str]) -> str | None:
    names = {Path(path).stem for path in paths}
    if domain in {"fs", "vfs", "fs-impl"}:
        for candidate in ("overlayfs", "mount", "symlink", "inotify", "ext2"):
            if candidate in names:
                return candidate
        return "path-resolution" if domain == "vfs" else None
    if domain == "process":
        for candidate in ("sched", "signal", "prctl", "clone3", "execve"):
            if candidate in names:
                return candidate
    if domain == "memory" and "mmap" in names:
        return "mmap"
    if domain == "network" and "socket" in names:
        return "socket"
    return None


def build_validation_plan(domain: str) -> tuple[str, str | None]:
    family = GENERAL_TEST_MODULES.get(domain)
    if family is None:
        return "report-only", None
    return "general test + verify", family


def subkey_for_domain(domain: str, path: str) -> str:
    """Return a sub-key for finer unit splitting within a domain."""
    maker = SUBKEY_HIERARCHY.get(domain)
    if maker:
        parts = maker(path)
        if parts:
            return "/".join(parts)
    return str(Path(path).parent)
