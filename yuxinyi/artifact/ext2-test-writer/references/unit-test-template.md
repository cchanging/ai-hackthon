# `#[ktest]` Template

Use this template when the semantic target is local enough to test inside the kernel.

```rust
#[cfg(ktest)]
mod tests {
    use ostd::prelude::ktest;

    use super::*;

    #[ktest]
    fn <semantic_case_name>() {
        // Setup only the minimal state needed for the semantic branch.

        // Execute the target operation.

        // Assert the exact semantic point:
        // - return value or error
        // - local state transition
        // - invariant that must hold
    }
}
```

## Placement

* prefer an existing nearby `test.rs` or local `#[cfg(ktest)]` module
* keep the test next to the code that owns the semantic contract
* avoid reaching through too many unrelated layers

## Good unit-test targets

* helper-level errno selection
* state-machine transitions
* cache invalidation rules
* cookie or offset progression
* branch ordering that is difficult to reach via userspace

## Good assertions

* exact error values
* exact state after the operation
* absence of unwanted mutation

## Avoid

* large fake environments when a smaller constructor or helper can express the case
* asserting implementation details that the spec does not require
* mixing multiple semantic obligations into one test
