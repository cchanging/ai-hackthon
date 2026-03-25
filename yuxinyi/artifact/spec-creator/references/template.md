# Agent-Friendly Spec Template

Use this template as a starting point. Remove sections that do not apply, but keep the overall separation between background, hard constraints, code shape, and acceptance.

````text
[TASK]
<One paragraph describing the concrete change.>

Goal:
- <What must be achieved.>
- <What bug / risk / design problem this solves.>

Output:
- <Expected output boundary, for example "Rust code only".>
- <No prose / no placeholders / no TODOs if applicable.>

This is a refactor of existing code, not a fresh implementation.
Before editing, identify the current runtime paths and touch points that must converge on the final design.

[SEMANTIC REFERENCES]
Use these sources as the semantic reference for behavior:
- <Linux function> -> <path:line or function>
- <Local reference> -> <path:line or function>

Important:
- Match Linux user-visible semantics and mutation ordering where applicable.
- Do NOT copy Linux locking literally.
- Adapt locking to Asterinas callback and safety constraints.

[BACKGROUND]
<Optional explanatory notes. Put rationale here, not in hard constraint sections.>

[LOCAL MODEL]
```rust
<Only the local structs, enums, traits, and signatures the agent actually needs.>
```

[LOCKING FACTS]
- <Callback-sensitive operations and their lock requirements.>
- <Allowed guard states.>
- <Existing lock ordering that must remain unchanged.>

[REQUIRED APIS]
Implement or preserve these APIs:

```rust
<Required functions and methods.>
```

[REQUIRED CALL GRAPH]
- There must be one canonical runtime implementation for <operation>.
- <Path A> MUST call <helper>.
- <Path B> MUST call <helper>.
- Runtime code MUST NOT call <old helper> on the <runtime path>.

[TOUCH POINTS]
You will likely need to touch:
- <Function or path 1>
- <Function or path 2>
- <Function or path 3>

[ALGORITHM: <name>]
Preconditions:
- <Required precondition 1>
- <Required precondition 2>

Required phases:
- PHASE 1: <validate or prepare>
- PHASE 2: <acquire guard / inspect state>
- PHASE 3: <upgrade / downgrade / grow / reserve>
- PHASE 4: <perform core mutation>
- PHASE 5: <commit metadata / persistence>
- PHASE 6: <return / restore guard state>

Postconditions on success:
- <State guarantee 1>
- <State guarantee 2>

Postconditions on failure:
- Return only <allowed errors>.
- <No partial state / no invalid on-disk state / lock released as required>.

[ALGORITHM: <secondary name>]
<Add more algorithm blocks when multiple APIs have distinct guard transitions or workflow contracts.>

[GLOBAL INVARIANTS]
- <System-wide invariant 1 that remains true across all touched paths.>
- <Locking invariant 2 that the refactor must not break.>
- <User-visible behavior invariant 3 that remains unchanged.>

[METHOD INVARIANTS]
## <method_name>
Before the call:
- <What must already be true when this method is entered.>

After success:
- <What must still be true after a successful return.>
- <What this method preserves while mutating internal state.>

After failure:
- <What must remain true after an error return.>
- <What invalid or partial state must never be persisted or exposed.>

## <another_method_name>
Before the call:
- <Entry invariant.>

After success:
- <Success invariant.>

After failure:
- <Failure invariant.>

[FORBIDDEN]
- Do not use `unsafe`.
- Do not introduce a second runtime implementation for <operation>.
- Do not hold <write lock> across <callback-driven I/O>.
- Do not change <user-visible behavior> outside the specified path.
- Do not rename unrelated symbols.

[EDIT SCOPE]
- Make the smallest safe refactor that satisfies this spec.
- Preserve unrelated helpers, types, and public behavior where possible.
- Do not rewrite unrelated parsing, layout, or allocation logic unless required by the canonical path.

[DIFF]
<Optional. Only use when Asterinas intentionally differs from Linux. State the difference and why.>

[ACCEPTANCE CHECKLIST]
- <Happy path assertion>
- <Error-path assertion>
- <Call-graph assertion>
- <Deadlock / lock-state assertion>
- <Behavioral regression guard>
````

## Conversion Notes

When converting an older `writing-spec` style input:

* Old `[SOURCE]` usually becomes `SEMANTIC REFERENCES`.
* Old `[RELY]` usually becomes `LOCAL MODEL`.
* Old `[GUARANTEE]` usually splits into `REQUIRED APIS`, `GLOBAL INVARIANTS`, and `METHOD INVARIANTS`.
* Old `[SPECIFICATION]` usually becomes one or more `ALGORITHM` blocks plus method-specific invariants.
* Old `[DIFF]` stays `DIFF`, but only when the difference matters.
* Old `[TEST]` should become `ACCEPTANCE CHECKLIST` with short assertion-style bullets.

## Style Reminders

* Keep background and hard constraints separate.
* Name exact runtime paths and forbidden old paths.
* Prefer phased algorithms over long narrative paragraphs.
* If multiple paths must share one implementation, say so directly and explicitly.
