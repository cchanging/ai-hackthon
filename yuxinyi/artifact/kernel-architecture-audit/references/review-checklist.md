# Review Checklist

Use this checklist mentally during every review:

- Is this a patch-style fix injected mainly to pass a specific test?
- Does VFS or another generic layer reach into a concrete filesystem implementation?
- Does this damage architectural consistency for future filesystem integrations?
- Is the design aligned with Linux's standard layering and dispatch model?
- Should this be solved through trait/interface refactoring instead of branch stacking?
- What missing abstraction is the current patch actually exposing?
- Is the conclusion a direct match, a behavioral match, or an inference?
- Is this a local implementation flaw, or will it pollute future filesystem integration paths?
