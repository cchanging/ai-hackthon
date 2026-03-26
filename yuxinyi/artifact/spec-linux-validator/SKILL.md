---
name: spec-linux-validator
description: use when chatgpt needs to validate whether code or a patch correctly follows a provided spec and whether it remains semantically compatible with a referenced linux implementation, especially for refactors, kernel/filesystem changes, or structured code-generation workflows where exact code similarity is not required.
---

# Spec Linux Validator

Validate implementations against two separate standards:

1. the provided spec,
2. the referenced Linux semantics.

Keep those judgments separate. A structural difference from Linux is not a failure by itself. A spec violation is not excused by superficial Linux similarity.

## Workflow

### 1. Gather the three inputs

Collect and label:

* the spec,
* the implementation or diff,
* the Linux references.

If any of the three are missing or partial, continue conservatively and say what cannot be concluded.

### 2. Normalize the spec before reviewing code

Do not review the code against raw prose. First reduce the spec into buckets:

* required APIs,
* required call-graph obligations,
* preconditions,
* postconditions,
* algorithm phases,
* global invariants,
* method invariants,
* forbidden actions,
* edit-scope constraints,
* acceptance checklist items.

Use [validation-rubric.md](./references/validation-rubric.md) for the exact buckets and evidence rules.

### 3. Validate spec conformance

For each normalized requirement, classify the implementation as:

* implemented exactly,
* implemented partially,
* missing,
* contradicted,
* unclear or not statically provable.

Do not claim compliance without code evidence. Prefer exact functions, branches, guards, conditions, and call sites over general impressions.

### 4. Validate Linux semantic compatibility

Use Linux as a semantic reference, not as a structural template.

Compare by behavior:

* visible success and failure conditions,
* errno mapping,
* mutation intent and ordering,
* replacement versus no-replacement behavior,
* state transitions and metadata/accounting,
* safety properties and invariant preservation.

Allow different helper decomposition, internal factoring, and lock-transition strategy when semantics still match and local runtime constraints justify the difference.

Use [linux-semantic-compat.md](./references/linux-semantic-compat.md) for allowed differences and semantic red flags.

### 5. Prioritize runtime hazards for kernel/filesystem work

When the change touches kernel or filesystem code, always inspect:

* lock ordering,
* callback or reentrancy hazards,
* forbidden runtime paths,
* mutation path convergence,
* metadata commit behavior,
* multi-object ordering invariants,
* error propagation and cleanup.

If the spec forbids a lock state or runtime path, treat violations as spec failures even when Linux uses a different structure.

### 6. Report in the strict output format

Always produce the final answer using the report structure in [report-template.md](./references/report-template.md).
Before sending the final answer, save the full markdown report under `.trellis/reports/spec/`
with:

```bash
python3 .trellis/scripts/save_report.py --kind spec-linux-validate --subdir spec --slug <target-slug>
```

Use a short slug derived from the spec or feature being verified.
In the final answer, include the saved report path.

The report must include:

* `PASS`, `PARTIAL`, or `FAIL` for spec conformance,
* `PASS`, `PARTIAL`, or `FAIL` for Linux semantic compatibility,
* evidence for every violation or claimed match,
* uncertainty notes for anything not statically provable,
* a final verdict with must-fix items and risk level,
* the saved report path under `.trellis/reports/spec/`.

## Decision Rules

Use these rules consistently:

* Treat missing evidence as `PARTIAL` or `unclear`, not as `PASS`.
* Treat a violated `FORBIDDEN` item as a spec failure.
* Treat a changed user-visible semantic, errno condition, or required invariant as a Linux semantic divergence.
* Treat pure helper reshaping as permissible if semantics and obligations remain intact.
* Treat ambiguous spec language as ambiguity, not as permission.

## Evidence Standard

Cite concrete evidence wherever possible:

* function and method names,
* branches and conditions,
* call paths,
* guards and lock transitions,
* return paths and errno mapping,
* state updates and commit points,
* exact spec clauses,
* exact Linux reference functions or scenarios.

Separate proven problems from plausible concerns.

## Resources

Read these references as needed:

* [validation-rubric.md](./references/validation-rubric.md) for the spec-conformance checklist and bucket model.
* [linux-semantic-compat.md](./references/linux-semantic-compat.md) for semantic-vs-structural Linux comparison.
* [report-template.md](./references/report-template.md) for the required final output format.
* `/root/asterinas/.trellis/scripts/save_report.py` for persisting the final markdown report.
