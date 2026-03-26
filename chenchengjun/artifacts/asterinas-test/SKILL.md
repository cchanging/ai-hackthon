---
name: asterinas-test
description: Run or compare Asterinas general test targets through one entry skill. Use for target listing, Asterinas-only runs, host-Linux-only runs, or Linux-versus-Asterinas verification of the same target set.
---

# Asterinas Test

Use this as the main entry point for Asterinas general test execution.

Supported forms:

- `list`
- `run asterinas <target...>`
- `run linux <target...>`
- `verify <target...>`

A target is a path relative to `/root/asterinas/test/initramfs/src/apps`.

Normalization rules:

- strip `/test/` or source-tree prefixes
- turn `.c` and `.S` source paths into built target paths by removing the suffix
- turn `<dir>/run_test.sh` into `<dir>`

## Workflow

1. Normalize and validate targets with `./scripts/target_utils.py`.
2. Execute the requested phases with `./scripts/run_targets.py`.
3. Surface saved log paths and a focused failure summary when a phase fails.

`run asterinas` means Asterinas-only execution.
`run linux` means host-Linux-only execution.
`verify` means Linux first, then Asterinas on the same target set.

## Commands

List targets:

```bash
python3 /root/.codex/skills/asterinas-test/scripts/run_targets.py list
```

Run on Asterinas only:

```bash
python3 /root/.codex/skills/asterinas-test/scripts/run_targets.py run --platform asterinas <target_a> [<target_b> ...]
```

Run on host Linux only:

```bash
python3 /root/.codex/skills/asterinas-test/scripts/run_targets.py run --platform linux <target_a> [<target_b> ...]
```

Verify Linux first, then Asterinas:

```bash
python3 /root/.codex/skills/asterinas-test/scripts/run_targets.py verify <target_a> [<target_b> ...]
```

## Constraints

- Run from `/root/asterinas`.
- Keep one target list per invocation whenever feasible.
- Reuse the skill runner instead of duplicating target normalization or log summarization.
- Preserve Linux logs and Asterinas logs as the source of truth.
- Prefer `verify` when the goal is behavior comparison rather than simple execution.

## Resources

- Test toolkit: `./scripts/`
