# Red Flags

## Generic-layer concrete knowledge

Treat the following as strong architectural smells when they appear in VFS or any generic filesystem abstraction:

- `downcast_ref`
- `downcast_mut`
- `Any`
- concrete type matching
- filesystem-name branches
- helpers added only to detect one concrete filesystem
- one-off dispatch hooks added only for a single test or filesystem

Reason: generic abstractions must not learn concrete filesystem internals.

## Ownership violations

Flag these as critical unless strongly justified:

- generic-layer writes to filesystem-owned readonly/readwrite state
- generic-layer mutation of mount-private or superblock-private flags
- state transitions driven outside the owner without a stable abstract interface
- policy decisions moved from concrete filesystem code into generic dispatch code

Reason: state-machine autonomy must remain with the owning subsystem.

## Test-driven patch smells

Treat these as suspicious:

- comments or logic explicitly tied to one xfstests case
- helper booleans or branches that exist only to make one test pass
- duplicated logic inserted near VFS entry points rather than fixing the underlying trait/interface gap
- "temporary" success paths for unsupported behavior

A test may reveal the bug, but it does not justify placing the fix in the wrong layer.

## Error-handling red flags

Flag these:

- unsupported operation converted to success
- swallowed errors
- silent fallback without explicit semantic justification
- lossy translation that hides actionable distinctions

Preserve observability and Linux-visible error semantics when possible.
