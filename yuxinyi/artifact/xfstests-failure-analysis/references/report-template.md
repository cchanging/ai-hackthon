# Xfstests Failure Report Template

Use this template verbatim for the saved report and the final diagnosis.

Do not split the conclusion into `Asterinas Behavior` and `Root Cause`. Keep them merged under `Diagnosis`.

```markdown
## Xfstests Failure Analysis

### Test Information
- Test: `<generic/213>`
- Failed operation: `<fallocate/write/unlink/...>`
- Failure symptom: `<expected vs actual>`

### Test Logic
1. `<step 1>`
2. `<step 2>`
3. `<failing assertion or observable mismatch>`

### Linux Reference
- Path: `<absolute path>`
- Relevant behavior: `<Linux-visible semantics enforced by this path>`

### Diagnosis
- Error location: `<file:function / branch / state transition / boundary where Asterinas diverges>`
- Reason:
  - `<reason 1>`
  - `<reason 2>`
- Call path:
```text
<test-visible operation>
  -> <VFS or syscall layer>
  -> <page-cache or VMO helper>
  -> <ext2 function or faulty point>
  -> <observable failure>
```
- Evidence:
  - `<key log evidence>`
  - `<key test or code evidence>`

### Repair Guidance
- Fix location: `<file:function(s) to modify>`
- Fix logic: `<minimal logic to add/remove/reorder/guard>`
- Post-fix semantics: `<observable behavior and internal guarantee that must hold after the fix>`

### Related Files
- `<path>`
- `<path>`
```

If a field is uncertain, fill it with the best supported conclusion and explicitly say what evidence is still missing.
