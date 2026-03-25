# Contract Spec Template

Use this as the default strong contract skeleton.

````text
[TITLE]
<Stable contract title>

[SCOPE]
- Boundary: API-level | module-level | feature-scenario-level
- Covered APIs or scenarios:
- Out of scope:
- Allowed internal flexibility:

[STATE MODEL]
Relevant in-memory state:
- <Exact struct or field names when known>

Relevant on-disk state:
- <Exact on-disk objects or mirrored fields when known>

[GLOBAL INVARIANTS]
- <Invariant that must remain true across all future implementations>
- <Invariant about visible semantics, state coherence, or persistent state>

[CROSS-API INVARIANTS]
- <Relation that must remain true across multiple APIs>
- <Consistency rule validators should enforce across operations>

[API CONTRACTS]

## API: <name>
[PURPOSE]
<Stable semantic role of this API>

[DOMAIN]
<Which object kinds, inode types, modes, or situations this API actually defines>

[PRECONDITIONS]
- <What must already be true or valid>

[OBSERVABLE EFFECTS]
- <Allowed externally visible effect categories>
- <Maximum allowed effect domain or range>
- <Visible effects that are explicitly forbidden>

[IN-MEMORY STATE POSTCONDITIONS: SUCCESS]
- <Exact in-memory structures that changed>
- <Exact in-memory structures that must remain unchanged>
- <Dirty-state or cache-coherence obligations>

[ON-DISK STATE POSTCONDITIONS: SUCCESS]
- <Which on-disk objects changed>
- <Which on-disk objects may remain dirty but not yet durable>
- <What on-disk coherence must hold after success>

[LOGICAL POSTCONDITIONS: SUCCESS]
- <Exact success meaning>
- <Range, size, returned-value, and visibility obligations>
- <Committed-prefix semantics when relevant>

[FAILURE POSTCONDITIONS]
- <What may remain partially committed>
- <What must not be over-reported as success>
- <What must remain coherent after failure>

[PRESERVED INVARIANTS]
- <Invariant that survives success and failure>

[ALLOWED INTERNAL FLEXIBILITY]
- <What may change internally without breaking the contract>

[LINUX SEMANTIC NOTES]
- <Optional stable Linux semantic baseline notes>

[SCENARIO CONTRACTS]

## Scenario: <name>
[PURPOSE]
<Why this scenario matters as a durable semantic case>

[DOMAIN]
<When this scenario applies>

[PRECONDITIONS]
- <Scenario entry conditions>

[OBSERVABLE EFFECTS]
- <Externally visible effects of the scenario>

[IN-MEMORY STATE POSTCONDITIONS: SUCCESS]
- <Scenario-level state obligations>

[ON-DISK STATE POSTCONDITIONS: SUCCESS]
- <Scenario-level persistent-state obligations>

[LOGICAL POSTCONDITIONS: SUCCESS]
- <Scenario-level success meaning>

[FAILURE POSTCONDITIONS]
- <What still holds if the scenario aborts or fails>

[PRESERVED INVARIANTS]
- <Scenario-level invariants>

[ALLOWED INTERNAL FLEXIBILITY]
- <What scenario-internal structure may vary>

[LINUX SEMANTIC NOTES]
- <Optional Linux semantic baseline notes>

[FORBIDDEN DRIFTS]
- <Visible behavior that must not change>
- <State relation that must not drift>
- <Failure behavior that must remain stable>
````

## Template Rules

* Prefer exact state obligations over design-summary language.
* Name exact local structs and fields when known.
* Separate visibility from durability.
* Separate full success from partial success.
* Separate failure postconditions from success postconditions.
* Add cross-API invariants whenever one API contract alone would be too weak.
* Do not freeze one helper layout, one lock choreography, or one refactor plan unless they are permanent semantic requirements.
* Make `[OBSERVABLE EFFECTS]` rule out forbidden effects rather than merely listing possible actions.
