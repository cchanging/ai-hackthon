# Cross-Check Guidance

Use this guidance when attributing a concurrency concern.

## Linux Versus Local Spec

Keep these separate:

* Linux ext2 is the behavioral reference.
* Asterinas specs are the locking and protocol reference.

This means:

* a local locking difference from Linux is not automatically a bug,
* a violation of a local lock protocol may still be a bug even when Linux uses a different structure,
* visible behavior under races should still match Linux expectations unless the spec documents a valid local difference.

## Attribution Labels

When a finding affects behavior or protocol, say whether it diverges from:

* `Asterinas spec`
* `Linux semantics`
* `both`
* `unclear`

Use:

* `Asterinas spec` when the issue is local locking or protocol correctness without proven Linux-visible divergence.
* `Linux semantics` when the race changes visible success, failure, revalidation, or error behavior compared with Linux.
* `both` when the code violates the local protocol and also changes Linux-relevant behavior.

## VFS Boundary Review

Treat `impl_for_vfs/` as part of the feature's concurrency protocol.

Check:

* whether the wrapper remains thin,
* whether it duplicates stateful checks,
* whether it stages snapshots or side effects before dispatch,
* whether it splits an operation that should remain atomic in Ext2 core,
* whether it changes cross-layer lock ordering.

If the wrapper adds logic, explain why it is safe or why it creates a risk.

## Linux Semantic Cross-Check

When a concurrency issue may affect observable behavior in the reviewed feature:

1. inspect the relevant Linux path,
2. inspect the relevant Asterinas spec or protocol note,
3. inspect the VFS entry path if the operation starts there,
4. state what Linux-visible behavior may change.

Focus on:

* wrong errno under races,
* wrong visibility of rename, unlink, or directory-entry state,
* incorrect revalidation behavior,
* wrong link-count or lifetime behavior,
* incorrect mutation ordering that leaks inconsistent state.

## Structural Differences That Are Usually Fine

Do not flag these by themselves:

* helper extraction,
* different internal factoring,
* callback-safe lock choreography that differs from Linux,
* local naming or control-flow differences.

Only flag them when they create a deadlock, stale-state window, or semantic divergence.
