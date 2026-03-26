# Linux Semantic Notes

Use Linux as a semantic baseline, not as a structural template.

## What Linux Should Constrain

Borrow durable semantics such as:

* visible success and failure behavior
* errno behavior
* mutation intent
* replacement versus no-replacement semantics
* size, lifetime, and consistency behavior
* stable state-transition expectations visible at the API boundary

When the local state model is known, convert Linux-visible semantics into local checkable obligations.

Example:

* not just "match Linux write semantics"
* but "successful direct write must not leave stale overlapping cached pages observable"

## What Linux Should Not Freeze

Do not automatically freeze:

* Linux helper structure
* Linux control-flow shape
* Linux lock choreography
* Linux data-structure factoring
* incidental implementation layout

Those belong to implementation choices unless the local system treats them as permanent requirements.

## How To Write Linux Notes Well

Good Linux notes:

* state what visible behavior must match Linux
* state what local state obligations realize that behavior
* keep structure and semantics separate

Good examples:

* Match Linux user-visible rename replacement semantics.
* Match Linux committed-prefix write accounting.
* Match Linux data-sync versus full-sync distinction.

Bad examples:

* Use the same helper structure as Linux.
* Follow Linux locking exactly.

## Linux Notes And Local State

If local structs are known, connect Linux semantics to them directly.

Examples:

* Linux-visible size growth semantics imply `InodeInner.desc.size` must reflect committed EOF growth.
* Linux-visible direct-I/O coherence implies `InodeInner.page_cache` must not retain stale overlapping data after successful direct write.
* Linux-visible fsync semantics imply dirty inode and block-mapping mirrors must not over-claim durability before the promised flush boundary.

## Local Runtime Constraints

If the local runtime has stronger permanent rules than Linux, include them in the contract.

Examples:

* callback-driven I/O must never observe partially committed metadata
* cache-visible bytes must not contradict committed direct-write results

If the rule is only one patch's implementation tactic, keep it out of the contract spec.
