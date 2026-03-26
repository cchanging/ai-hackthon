---
name: xfstests-failure-analysis
description: analyze xfstests failures for Asterinas Ext2 by reading the failing xfstests case, error logs, Linux ext2 reference code, and the local Ext2 implementation. use when an agent needs to diagnose why an xfstests case failed, map the symptom to the relevant syscall or filesystem path, merge Asterinas behavior and root cause into a reviewer-friendly diagnosis, and produce repair guidance with fix location, fix logic, and post-fix semantics.
---

# Xfstests Failure Analysis

Diagnose one xfstests failure end to end and hand off a repair-ready report.

Start from the failing test and its log, prove what behavior the test expects, then explain where Asterinas Ext2 diverges from Linux-visible semantics or required internal behavior.

Before drafting the final report, open `references/report-template.md` and follow it exactly. Do not invent a different section layout.

## Working Set

Use these paths by default:

* xfstests source: `/root/xfstests/`
* xfstests logs: `/root/xfstests/error_log/`
* Linux ext2 reference: `/root/linux/fs/ext2/`
* Asterinas Ext2: `/root/asterinas/kernel/src/fs/fs_impls/ext2/`

Read adjacent VFS, page-cache, or syscall code when the failure path crosses filesystem boundaries.

## Workflow

### 1. Identify the failed case precisely

Read the failing log first. Extract:

* test name such as `generic/213`,
* visible symptom,
* expected vs actual result,
* failing operation or syscall surface,
* relevant errno or output mismatch.

Do not start from a guessed subsystem. Let the failing test and log constrain the search space.

### 2. Read the test before reasoning about the bug

Locate the exact xfstests case and inspect its helpers only as needed.

Determine:

* what sequence the test performs,
* what behavior it asserts,
* which syscalls or filesystem operations matter,
* which preconditions make the failure meaningful.

If the test relies on common helpers, read only the helper fragments needed to understand the assertion.

### 3. Establish Linux-visible behavior

Find the smallest Linux ext2 path that explains the expected behavior.

Check:

* returned errno,
* allocation or metadata update rules,
* state checks and failure paths,
* whether the behavior is really ext2-specific or enforced in a higher layer.

Linux is the semantic reference. Do not require structural similarity when a different local implementation still preserves visible behavior.

### 4. Trace the Asterinas path fully

Search first, then read the reachable code path directly.

Compare Asterinas against the test expectation and Linux behavior. Focus on:

* missing state validation,
* incorrect error propagation,
* missing block or metadata updates,
* wrong ordering around allocation, truncation, lookup, or writeback,
* wrong assumptions about sparse files, directory entries, link counts, or inode state.

When the path crosses into page cache, VFS, or syscall validation, include that code in the diagnosis instead of forcing everything into `ext2/`.

### 5. Build one diagnosis block, not two disconnected ones

Humans must be able to validate the conclusion quickly. Merge `Asterinas Behavior` and `Root Cause` into one `Diagnosis` section containing exactly:

* `Error location`: the concrete faulty function, branch, state transition, or cross-layer boundary,
* `Reason`: a short bullet list explaining why the current behavior is wrong and why it produces the observed failure,
* `Call path`: a visualized end-to-end path from user-visible operation to the faulty point,
* `Evidence`: a short bullet list of the key log, test, and code evidence.

Anchor the diagnosis to code evidence. If there are multiple plausible causes, rank them and state what evidence is missing for a higher-confidence conclusion.

Do not emit separate `Asterinas Behavior` and `Root Cause` sections.

### 6. Add a repair-guidance block for the next agent

The report must contain one `Repair Guidance` section containing exactly:

* `Fix location`: the file and function(s) that should be changed,
* `Fix logic`: the minimal logic that must be added, removed, reordered, or guarded,
* `Post-fix semantics`: the user-visible and internal semantics that must hold after the fix.

Recommend the smallest credible change set. Do not propose a fix that is not supported by the traced code path.

### 7. Persist the final analysis

Before sending the final answer, save the full markdown report under `.trellis/reports/xfstests/`
with:

```bash
python3 .trellis/scripts/save_report.py --kind xfstests-analysis --subdir xfstests --slug <case-slug>
```

Use a short slug derived from the failing case, for example `generic-029`.
In the final answer, include the saved report path.

## Output Format

Read `references/report-template.md` before drafting the final diagnosis.

Fill every field in the template. If something is unknown, say what is missing to determine it.

Prefer concrete file paths, function names, and condition-specific behavior over generic summaries.
Do not treat the analysis as complete until the final report has been persisted under `.trellis/reports/xfstests/`.

## Rules

Do:

* read the actual test before drawing conclusions,
* anchor findings to code evidence,
* distinguish Linux-visible behavior from implementation detail,
* call out uncertainty explicitly,
* make the diagnosis block easy for a reviewer to confirm or reject,
* make the repair-guidance block specific enough for another agent to implement from directly.

Do not:

* guess the test intent from the failure message alone,
* stop at the first matching function name without tracing the full path,
* propose fixes that ignore the observed call flow,
* split the final conclusion into `Asterinas Behavior` and `Root Cause`,
* modify source files as part of analysis unless the user explicitly asks for code changes.
