# Test Validation Report Template

Always use this structure in the final answer.

```markdown
# Test Validation Report

## 1. Spec To Test Coverage
Status: PASS | PARTIAL | FAIL

### Covered
- Spec obligation:
  `.test` evidence:

### Missing Or Contradicted
- Spec obligation:
  `.test` evidence:
  Why it fails:

### Unclear
- Spec obligation:
  Why it is unclear:
  Missing evidence:

## 2. Existing Test Coverage Of `.test`
Status: PASS | PARTIAL | FAIL

### Covered By Current Tests
- `.test` case:
  Existing test evidence:

### Missing Runtime Coverage
- `.test` case:
  Missing test type:
  Why it matters:

### Partial Coverage
- `.test` case:
  Existing test evidence:
  What remains uncovered:

## 3. Code Logic Conformance To `.test`
Status: PASS | PARTIAL | FAIL

### Logically Satisfied
- `.test` case:
  Code evidence:

### Contradicted
- `.test` case:
  Code evidence:
  Impact:

### Not Statistically Provable / Unclear
- `.test` case:
  Missing code evidence:
  What can still be concluded:

## 4. Missing Or Weak `.test` Cases
- Gap type:
  Recommended addition:
  Why it matters:

## 5. Final Verdict
- Overall result:
- Must-fix issues:
- Risk level:
- Recommended next edits:

## 6. Report Path
- Saved report: `.trellis/reports/test/test-validate-YYYY-MM-DD-HHMMSS-<slug>.md`
```

## Report Rules

* Keep the four validation layers separate.
* Do not hide a code failure inside a test-coverage note.
* Do not hide a missing `.test` case inside a runtime-test gap.
* Cite concrete evidence for every coverage claim.
