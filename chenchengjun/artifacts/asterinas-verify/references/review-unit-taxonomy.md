# Review Unit Taxonomy

Use this taxonomy when a diff spans more than one logical behavior area.

## Unit Kinds

### `syscall`

Use when the review target is a Linux-facing syscall entry under `<repo>/kernel/src/syscall/`.

Typical signals:

- a single syscall file changed
- the main behavioral question is Linux syscall semantics
- the best tests belong under an existing general test module

### `module`

Use when the change modifies a subsystem implementation without introducing a new cross-cutting contract.

Typical signals:

- VFS path resolution logic
- filesystem implementation behavior
- memory-management implementation details
- networking internals with an existing socket or protocol contract

### `feature-interface`

Use when the change adds or substantially changes a trait, callback, hook, or cross-subsystem contract.

Typical signals:

- new trait methods
- new interface hooks such as `revalidate`
- new required callbacks for filesystem or device implementations
- changes whose main risk is incomplete adoption by callers or implementers

## Splitting Rules

Split into multiple units when:

- the diff contains more than one syscall family
- it mixes syscall entry changes and subsystem implementation changes
- it changes both interface contracts and concrete implementations
- distinct units would have different spec surfaces or validation surfaces

Keep one unit when:

- the paths are tightly coupled
- the behavior surface is still one externally observable change

## Routing Defaults

- `syscall` -> `asterinas-syscall-review`
- `module` -> `asterinas-module-review`
- `feature-interface` -> `asterinas-module-review`
