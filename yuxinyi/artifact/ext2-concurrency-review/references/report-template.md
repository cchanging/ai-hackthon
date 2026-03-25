# Concurrency Report Template

Always use this output structure.

```markdown
VERDICT: [PASS | FAIL | SUSPICIOUS]
CONFIDENCE: [HIGH | MEDIUM | LOW]
SUMMARY: One-line conclusion

## Scope

- Feature reviewed:
- Primary entry points:
- VFS entry points included:
- Linux references used:

## Findings

1. **High | Medium | Low** — `path:line`
   - Category: Deadlock | Lock-Release Window | Other
   - Diverges from: Asterinas spec | Linux semantics | both | unclear
   - Why it is risky:
   - Concurrent scenario:
   - Evidence:
   - Expected safe pattern:

## Lock-Release Windows Reviewed

1. `function_name`
   - Entry path: `impl_for_vfs/...` -> `...`
   - Window: lock released between A and B
   - State carried across window:
   - Revalidation after reacquire: [yes | no | partial]
   - Verdict: [safe | unsafe | unclear]

## VFS Boundary Review

- Entry methods reviewed:
- Wrapper remains thin: [yes | no | mixed]
- Cross-layer lock/order concerns:
- Cross-layer validation/responsibility concerns:

## Report Path

- Saved report: `.trellis/reports/concurrency/concurrency-check-YYYY-MM-DD-HHMMSS.md`
```

## No-Finding Variant

If no concrete issue is found, write:

```markdown
## Findings

No concrete concurrency correctness issues found in the reviewed feature.
Residual risk: <if any>
```

## Report Rules

* Findings come first.
* Cite exact functions, paths, and line numbers.
* Explain the interleaving that makes the issue real.
* Separate proven issues from suspicion.
* Save the full markdown report before replying.
* Use `python3 .trellis/scripts/save_report.py --kind concurrency-check --subdir concurrency --slug <target-slug>`.
* Print the saved report path.
