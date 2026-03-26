# Validation Report Template

Always use this structure in the final answer.

```markdown
# Validation Report

## 1. Spec Conformance
Status: PASS | PARTIAL | FAIL

### Satisfied
- Requirement:
  Evidence:

### Violations
- Requirement:
  Evidence:
  Why it fails:

### Unclear / Not Provable
- Requirement:
  Why it is unclear:
  Missing evidence:

## 2. Linux Semantic Compatibility
Status: PASS | PARTIAL | FAIL

### Semantically Compatible
- Linux semantic expectation:
  Implementation evidence:

### Semantic Divergences
- Linux semantic expectation:
  Implementation behavior:
  Impact:

### Permissible Structural Differences
- Difference:
  Why it is acceptable:

### Unclear / Insufficient Linux Evidence
- Scenario:
  What is missing:
  What can still be concluded:

## 3. Final Verdict
- Overall result:
- Must-fix issues:
- Risk level:
- Recommended next edits:

## 4. Report Path
- Saved report: `.trellis/reports/spec/spec-linux-validate-YYYY-MM-DD-HHMMSS-<slug>.md`
```

## Report Rules

### Status Meanings

* `PASS`: no material violation found in that section, and remaining uncertainty is limited to runtime-only proof gaps.
* `PARTIAL`: some requirements or semantics are plausible but not fully provable, or there are mixed results without a complete failure.
* `FAIL`: there is at least one concrete material violation in that section.

### Findings Rules

* Keep proven violations separate from uncertainty.
* Keep spec failures separate from Linux semantic divergences.
* Do not use one section to hide a failure in the other.
* Cite concrete implementation evidence whenever possible.

### Final Verdict Rules

Summarize three things:

* whether the implementation is acceptable as-is,
* what must be fixed before acceptance,
* how severe the issues are.

Use these severity labels when useful:

* `correctness-critical`
* `safety-critical`
* `cleanup-level`

If the implementation is blocked only by missing proof, say so explicitly.

### Persistence Rules

Save the full markdown report under `.trellis/reports/spec/` before replying.
Use `python3 .trellis/scripts/save_report.py --kind spec-linux-validate --subdir spec --slug <target-slug>`.
Print the saved report path in the final answer.
