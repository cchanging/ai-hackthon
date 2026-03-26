# Behavioral Spec Patterns

Use these patterns when the change has no single manual page or standard.

## Cache revalidation and path lookup

Describe the contract in observable terms:

- stale cached entries must not make later `lookup`, `open`, or `stat` return an object that is no longer authoritative
- negative dentries must be invalidated when the backing state changes
- repeated lookups should converge to the backing filesystem state

Useful Linux evidence families:

- dcache and namei paths
- `d_revalidate`
- overlayfs and networked filesystem lookup behavior

## New trait hooks or callbacks

Treat the spec as a cross-product:

- caller obligations
- implementer obligations
- default behavior when the hook is absent
- error propagation
- locking and blocking constraints

The main bug pattern is incomplete adoption rather than a single wrong return value.

## Cache coherency changes

Look for:

- stale positive cache entries
- stale negative cache entries
- rename, unlink, and mount transitions
- cross-directory and cross-mount visibility

## Internal-only semantic changes

If no user-visible oracle exists, define:

- which invariant must hold
- where that invariant is observable in tests or logs
- why the item must remain `report-only`

## User-space input surfaces

When the behavior is reachable from syscalls or other user-controlled entry points, enumerate the corner cases that can invalidate hidden assumptions:

- null, empty, and zero-length inputs
- maximum-size and boundary-size buffers or paths
- invalid, partially valid, or misaligned user pointers
- flag combinations, reserved bits, and arguments that disagree with each other
- repeated calls, retries, partial success, and state transitions around error handling

Prefer the cases that a user can trigger directly and that have a crisp observable oracle.
