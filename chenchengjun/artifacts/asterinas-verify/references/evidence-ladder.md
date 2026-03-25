# Spec Evidence Ladder

Use this order when looking for the governing semantics of a change.

## Level 1: External specification

Prefer explicit documents when they exist:

- POSIX
- Linux man-pages
- RFCs
- subsystem design documents

This is the strongest source when the behavior is user visible.

## Level 2: Upstream Linux behavior

When no formal spec exists, use upstream Linux as behavioral evidence:

- implementation entry points
- closely related helpers
- upstream tests
- comments that describe contract boundaries

State when the behavior is derived from implementation rather than a formal spec.

## Level 3: Asterinas internal contracts

Look for:

- trait documentation
- existing tests
- invariants implied by naming or helper layering
- explicit TODOs or unsupported markers

This matters most for interface and module changes.

## Level 4: Diff-local intent

Read the current patch and transitive call path to infer:

- what contract is being introduced or modified
- which callers and implementers must now comply
- which old assumptions may have become stale

This level supports the earlier levels; it does not replace them.

## Usage Rules

- Record the strongest evidence actually found for each behavior.
- If the best evidence is implementation-derived, say so explicitly.
- Do not classify a low-evidence hypothesis as a confirmed bug.
- For new hooks or callbacks, prefer `contract-risk` until caller and implementer coverage is checked.
- When the remaining uncertainty is concentrated in a user-visible corner case, use a targeted confirmation test if it has a crisp oracle; record that the test supplements the evidence ladder rather than replacing it.
