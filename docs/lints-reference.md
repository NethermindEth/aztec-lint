# Lint Reference

This document lists active enforced lints in `aztec-lint` and explains what each lint checks, why it matters, known limitations, and typical remediation.

Source of truth for this data is the canonical lint metadata catalog in `crates/aztec-lint-core/src/lints/mod.rs`.

## AZTEC Pack

### AZTEC001

- Pack: `aztec_pack`
- Category: `privacy`
- Policy: `privacy`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Private data reaches a public sink.

What it does:
Flags flows where secret or note-derived values are emitted through public channels.

Why this matters:
Leaking private values through public outputs can permanently expose sensitive state.

Known limitations:
Flow analysis is conservative and may miss leaks routed through unsupported abstractions.

How to fix:
Keep private values in constrained private paths and sanitize or avoid public emission points.

Examples:
- Avoid emitting note-derived values from public entrypoints.

References:
- `docs/suppression.md`
- `docs/rule-authoring.md`

### AZTEC002

- Pack: `aztec_pack`
- Category: `privacy`
- Policy: `privacy`
- Default Level: `deny`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Secret-dependent branching affects public state.

What it does:
Detects control flow where secret inputs influence public behavior.

Why this matters:
Secret-dependent branching can reveal private information through observable behavior.

Known limitations:
Heuristic path tracking may report false positives in complex guard patterns.

How to fix:
Refactor logic so branch predicates for public effects are independent of private data.

Examples:
- Compute public decisions from public inputs only.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### AZTEC003

- Pack: `aztec_pack`
- Category: `privacy`
- Policy: `privacy`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Private entrypoint uses debug logging.

What it does:
Reports debug logging in private contexts where logging may leak sensitive state.

Why this matters:
Debug output can disclose values intended to remain private.

Known limitations:
Custom logging wrappers are only detected when call patterns are recognizable.

How to fix:
Remove debug logging from private code paths or replace it with safe telemetry patterns.

Examples:
- Do not print private witnesses in private functions.

References:
- `docs/suppression.md`
- `docs/rule-authoring.md`

### AZTEC010

- Pack: `aztec_pack`
- Category: `protocol`
- Policy: `protocol`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Private to public bridge requires `#[only_self]`.

What it does:
Checks enqueue-based private-to-public transitions enforce self-only invocation constraints.

Why this matters:
Missing self-only restrictions can allow unauthorized cross-context execution.

Known limitations:
Rule coverage is scoped to known enqueue bridge patterns.

How to fix:
Apply the configured only-self attribute and ensure bridge entrypoints enforce it.

Examples:
- Annotate private-to-public bridge functions with `#[only_self]`.

References:
- `docs/decisions/0001-aztec010-scope.md`
- `docs/rule-authoring.md`

### AZTEC020

- Pack: `aztec_pack`
- Category: `soundness`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Unconstrained influence reaches commitments, storage, or nullifiers.

What it does:
Detects unconstrained values that affect constrained Aztec protocol artifacts.

Why this matters:
Unconstrained influence can break proof soundness and on-chain validity assumptions.

Known limitations:
Transitive influence through unsupported helper layers may be missed.

How to fix:
Introduce explicit constraints before values affect commitments, storage, or nullifiers.

Examples:
- Constrain intermediate values before writing storage commitments.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### AZTEC021

- Pack: `aztec_pack`
- Category: `soundness`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Missing range constraints before hashing or serialization.

What it does:
Reports values hashed or serialized without proving required numeric bounds first.

Why this matters:
Unchecked ranges can make hash and encoding logic semantically ambiguous.

Known limitations:
The rule cannot infer all user-defined range proof helper conventions.

How to fix:
Apply explicit range constraints before hashing, packing, or serialization boundaries.

Examples:
- Add a range check before converting a field to a bounded integer.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### AZTEC022

- Pack: `aztec_pack`
- Category: `soundness`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Suspicious Merkle witness usage.

What it does:
Finds witness handling patterns that likely violate expected Merkle proof semantics.

Why this matters:
Incorrect witness usage can invalidate inclusion guarantees.

Known limitations:
Complex custom witness manipulation may produce conservative warnings.

How to fix:
Verify witness ordering and path semantics against the target Merkle API contract.

Examples:
- Ensure witness paths and leaf values are paired using the expected order.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

## Noir Core Pack

### NOIR001

- Pack: `noir_core`
- Category: `correctness`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Unused variable or import.

What it does:
Detects declared bindings and imports that are not used.

Why this matters:
Unused items can indicate dead code, mistakes, or incomplete refactors.

Known limitations:
Generated code and macro-like patterns may trigger noisy diagnostics.

How to fix:
Remove unused bindings or prefix intentionally unused values with an underscore.

Examples:
- Delete unused imports after refactoring call sites.

References:
- `docs/rule-authoring.md`

### NOIR002

- Pack: `noir_core`
- Category: `correctness`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Suspicious shadowing.

What it does:
Reports variable declarations that shadow earlier bindings in the same function scope.

Why this matters:
Shadowing can hide logic bugs by silently changing which binding is referenced.

Known limitations:
Intentional narrow-scope shadowing may be flagged when context is ambiguous.

How to fix:
Rename inner bindings to make value flow explicit.

Examples:
- Use descriptive names instead of reusing accumulator variables.

References:
- `docs/rule-authoring.md`

### NOIR010

- Pack: `noir_core`
- Category: `correctness`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Boolean computed but not asserted.

What it does:
Flags boolean expressions that appear intended for checks but never drive an assertion.

Why this matters:
Forgotten assertions can leave critical invariants unenforced.

Known limitations:
Rules cannot always infer whether an unasserted boolean is intentionally stored for later use.

How to fix:
Use assert-style checks where the boolean is intended as a safety or validity guard.

Examples:
- Convert an unconsumed `is_valid` expression into an assertion.

References:
- `docs/rule-authoring.md`

### NOIR020

- Pack: `noir_core`
- Category: `correctness`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Array indexing without bounds validation.

What it does:
Detects index operations lacking an obvious preceding range constraint.

Why this matters:
Unchecked indexing can cause invalid behavior and proof failures.

Known limitations:
Complex index sanitization paths may not always be recognized.

How to fix:
Establish and assert index bounds before indexing operations.

Examples:
- Assert `idx < arr.len()` before reading `arr[idx]`.

References:
- `docs/rule-authoring.md`

### NOIR030

- Pack: `noir_core`
- Category: `correctness`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Unconstrained value influences constrained logic.

What it does:
Reports suspicious influence of unconstrained data over constrained computation paths.

Why this matters:
Mixing unconstrained and constrained logic can invalidate proof assumptions.

Known limitations:
Inference can be conservative for deeply indirect data flow.

How to fix:
Constrain values before they participate in constrained branches or outputs.

Examples:
- Introduce explicit constraints at trust boundaries.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### NOIR100

- Pack: `noir_core`
- Category: `maintainability`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Magic number literal should be named.

What it does:
Encourages replacing unexplained numeric constants with named constants.

Why this matters:
Named constants improve readability and reduce accidental misuse.

Known limitations:
Small obvious literals may still be reported depending on context.

How to fix:
Define a constant with domain meaning and use it in place of the literal.

Examples:
- Replace `42` with `MAX_NOTES_PER_BATCH`.

References:
- `docs/rule-authoring.md`

### NOIR110

- Pack: `noir_core`
- Category: `maintainability`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Function complexity exceeds threshold.

What it does:
Flags functions whose control flow complexity passes the configured limit.

Why this matters:
High complexity makes correctness and audits harder.

Known limitations:
Simple metric thresholds cannot capture all maintainability nuances.

How to fix:
Split large functions and isolate complex branches into focused helpers.

Examples:
- Extract nested decision trees into named helper functions.

References:
- `docs/rule-authoring.md`

### NOIR120

- Pack: `noir_core`
- Category: `maintainability`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Function nesting depth exceeds threshold.

What it does:
Flags deeply nested control flow that reduces readability and maintainability.

Why this matters:
Deep nesting increases cognitive load and maintenance risk.

Known limitations:
Certain generated or domain-specific patterns can be naturally nested.

How to fix:
Use early returns and helper functions to flatten nested control flow.

Examples:
- Refactor nested conditionals into guard clauses.

References:
- `docs/rule-authoring.md`
