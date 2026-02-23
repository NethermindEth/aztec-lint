# Lint Reference

This document lists active enforced lints in `aztec-lint` and explains what each lint checks, why it matters, known limitations, and typical remediation.

Source of truth for this data is the canonical lint metadata catalog in `crates/aztec-lint-core/src/lints/mod.rs`.

Policy note: `performance` is the canonical metadata policy name; roadmap shorthand `cost` maps to `performance`.

## AZTEC Pack

### AZTEC001

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `stable`
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
- Maturity: `preview`
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
- Maturity: `stable`
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
- Maturity: `stable`
- Policy: `protocol`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Private to public bridge requires #[only_self].

What it does:
Checks enqueue-based private-to-public transitions enforce self-only invocation constraints.

Why this matters:
Missing self-only restrictions can allow unauthorized cross-context execution.

Known limitations:
Rule coverage is scoped to known enqueue bridge patterns.

How to fix:
Apply the configured only-self attribute and ensure bridge entrypoints enforce it.

Examples:
- Annotate private-to-public bridge functions with #[only_self].

References:
- `docs/decisions/0001-aztec010-scope.md`
- `docs/rule-authoring.md`

### AZTEC020

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `stable`
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
- Maturity: `stable`
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
- Maturity: `stable`
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

### AZTEC030

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `preview`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.5.0`
- Lifecycle: `active`
- Summary: Note consumption without nullifier emission.

What it does:
Reports note pop/consume patterns when the same function does not emit a nullifier.

Why this matters:
Consumed notes without nullifiers can enable replay or double-spend style state inconsistencies.

Known limitations:
Function-local matching does not prove path-complete nullifier coverage in highly dynamic control flow.

How to fix:
Emit nullifiers for consumed notes or switch to helper APIs that enforce consume-and-nullify semantics.

Examples:
- After `pop_note` or `pop_notes`, emit the associated nullifier in the same function path.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### AZTEC031

- Pack: `aztec_pack`
- Category: `protocol`
- Maturity: `preview`
- Policy: `protocol`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.5.0`
- Lifecycle: `active`
- Summary: Nullifier hash appears missing domain separation inputs.

What it does:
Flags nullifier hash call sites where required domain components are not present in hash inputs.

Why this matters:
Weak nullifier domain separation can cause collisions across domains or protocol contexts.

Known limitations:
Heuristic token matching may miss custom domain-separation helpers or aliases.

How to fix:
Include configured domain fields (for example contract address and nonce) in nullifier hash inputs.

Examples:
- Include `this_address` and `nonce` (or equivalent fields) in the nullifier hash tuple.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### AZTEC032

- Pack: `aztec_pack`
- Category: `protocol`
- Maturity: `preview`
- Policy: `protocol`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.5.0`
- Lifecycle: `active`
- Summary: Commitment hash appears missing domain separation inputs.

What it does:
Detects commitment-style hash sinks that do not include configured domain-separation components.

Why this matters:
Insufficient commitment domain separation can blur security boundaries and weaken protocol assumptions.

Known limitations:
Rule matching focuses on recognizable commitment sink names and hash-shaped inputs.

How to fix:
Add required context fields (such as contract address and note type) to commitment hash construction.

Examples:
- Derive commitments with explicit domain tags instead of hashing only payload values.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### AZTEC033

- Pack: `aztec_pack`
- Category: `protocol`
- Maturity: `preview`
- Policy: `protocol`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.5.0`
- Lifecycle: `active`
- Summary: Public entrypoint mutates private state without #[only_self].

What it does:
Reports public entrypoints that appear to mutate private note/state transitions and lack only-self protection.

Why this matters:
Publicly callable private-state mutation surfaces can break intended access boundaries.

Known limitations:
Detection relies on recognized mutation patterns and may not cover every custom state transition helper.

How to fix:
Add `#[only_self]` to the public entrypoint or refactor the mutation into a safer private flow.

Examples:
- Mark public state-transition bridges with `#[only_self]` before calling note mutation APIs.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0001-aztec010-scope.md`

### AZTEC034

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `preview`
- Policy: `soundness`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.5.0`
- Lifecycle: `active`
- Summary: Hash input cast to Field without prior range guard.

What it does:
Finds hash inputs that are cast or converted to Field without an earlier range-style constraint.

Why this matters:
Missing range proofs can make hashed representations ambiguous for bounded integer semantics.

Known limitations:
Nearby helper-based constraints may not be recognized when they do not resemble explicit range checks.

How to fix:
Constrain numeric width before Field conversion and hashing, then keep the guarded value flow explicit.

Examples:
- Assert bounded `amount` before hashing `amount as Field`.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### AZTEC035

- Pack: `aztec_pack`
- Category: `correctness`
- Maturity: `preview`
- Policy: `correctness`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.5.0`
- Lifecycle: `active`
- Summary: Suspicious repeated nested storage key.

What it does:
Flags `.at(x).at(x)`-style nested key repetition that often indicates copy-paste key mistakes.

Why this matters:
Repeating nested map keys unintentionally can corrupt indexing logic and authorization behavior.

Known limitations:
Some intentionally duplicated keying patterns may require suppression when semantically correct.

How to fix:
Use distinct key expressions for each nested `.at(...)` level or extract named key variables for clarity.

Examples:
- Replace `.at(owner).at(owner)` with the intended second key such as `.at(owner).at(spender)`.

References:
- `docs/rule-authoring.md`
- `docs/suppression.md`

### AZTEC036

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `preview`
- Policy: `privacy`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.6.0`
- Lifecycle: `active`
- Summary: Secret-dependent branch affects enqueue behavior.

What it does:
Flags private or secret-influenced branching that changes whether or how enqueue-style bridge calls are emitted.

Why this matters:
Observer-visible enqueue shape differences can leak private branch decisions.

Known limitations:
Pattern matching is currently heuristic and may not cover every custom enqueue wrapper.

How to fix:
Refactor enqueue behavior so public bridge decisions are independent of secret branch predicates.

Examples:
- Emit a fixed enqueue pattern and move secret-dependent logic into constrained private computation.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### AZTEC037

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `preview`
- Policy: `privacy`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.6.0`
- Lifecycle: `active`
- Summary: Secret-dependent branch affects delivery count.

What it does:
Reports branch-dependent behavior where secret inputs influence the number or presence of delivery-style effects.

Why this matters:
Varying delivery cardinality on secret predicates can reveal private state through externally visible behavior.

Known limitations:
Delivery sink coverage is currently scoped to recognized call patterns.

How to fix:
Keep delivery count and emission structure invariant with respect to secret branch conditions.

Examples:
- Avoid conditional delivery emission based on private note values.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### AZTEC038

- Pack: `aztec_pack`
- Category: `correctness`
- Maturity: `preview`
- Policy: `correctness`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.6.0`
- Lifecycle: `active`
- Summary: Change note appears to miss fresh randomness.

What it does:
Detects change-note construction patterns that appear to reuse deterministic randomness or omit freshness inputs.

Why this matters:
Weak randomness freshness can increase linkage risk and break expected note uniqueness properties.

Known limitations:
Freshness detection is heuristic and may miss user-defined entropy helper conventions.

How to fix:
Derive change-note randomness from a fresh, non-reused source and thread it explicitly into note construction.

Examples:
- Use a per-note fresh randomness value instead of reusing an existing note nonce.

References:
- `docs/rule-authoring.md`
- `docs/suppression.md`

### AZTEC039

- Pack: `aztec_pack`
- Category: `correctness`
- Maturity: `preview`
- Policy: `correctness`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.6.0`
- Lifecycle: `active`
- Summary: Partial spend logic appears unbalanced.

What it does:
Flags partial-spend arithmetic patterns that do not clearly reconcile consumed, spent, and change values.

Why this matters:
Unbalanced partial-spend accounting can cause invalid state transitions or silent value drift.

Known limitations:
Equivalent arithmetic forms may not all be recognized by pattern-driven detection.

How to fix:
Make spend and change reconciliation explicit and assert conservation-style invariants near the transition point.

Examples:
- Ensure `consumed = spend + change` is enforced before emitting updated notes.

References:
- `docs/rule-authoring.md`
- `docs/suppression.md`

### AZTEC040

- Pack: `aztec_pack`
- Category: `protocol`
- Maturity: `preview`
- Policy: `protocol`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.6.0`
- Lifecycle: `active`
- Summary: Initializer entrypoint missing #[only_self].

What it does:
Reports initializer functions that are not protected by the expected only-self access restriction.

Why this matters:
Unrestricted initializers can allow unauthorized setup flows or protocol-state takeover.

Known limitations:
Framework-equivalent guards not expressed through the configured only-self signal may need suppression.

How to fix:
Annotate initializer entrypoints with `#[only_self]` or move privileged initialization behind a self-only gate.

Examples:
- Mark contract initializer functions with `#[only_self]` before deployment use.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0001-aztec010-scope.md`

### AZTEC041

- Pack: `aztec_pack`
- Category: `correctness`
- Maturity: `preview`
- Policy: `correctness`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.6.0`
- Lifecycle: `active`
- Summary: Field/integer cast may truncate or wrap unexpectedly.

What it does:
Finds cast patterns between Field and bounded integers that lack nearby guard conditions proving safe range.

Why this matters:
Unchecked narrowing conversions can silently corrupt values and invalidate downstream protocol logic.

Known limitations:
Guard recognition focuses on known range-check idioms and may miss custom helper abstractions.

How to fix:
Add explicit range checks before narrowing casts and keep the guarded value flow local and visible.

Examples:
- Assert value bounds before converting `Field` into a narrower integer type.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

## Noir Core Pack

### NOIR001

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
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
- Maturity: `stable`
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
- Maturity: `stable`
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
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `high`
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
- Maturity: `stable`
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
- Maturity: `stable`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Magic number literal should be named.

What it does:
Detects high-signal numeric literals used in branch/assert/hash/serialization and related protocol-sensitive contexts.

Why this matters:
Named constants improve readability and reduce accidental misuse.

Known limitations:
Low-signal plain local initializer literals are intentionally excluded from this rule.

How to fix:
Define a constant with domain meaning and use it in place of the literal.

Examples:
- Replace `42` with `MAX_NOTES_PER_BATCH`.

References:
- `docs/rule-authoring.md`

### NOIR101

- Pack: `noir_core`
- Category: `maintainability`
- Maturity: `preview`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`
- Summary: Repeated local initializer magic number should be named.

What it does:
Reports repeated literal values used in plain local initializer assignments within the same function/module scope.

Why this matters:
Repeated unexplained initializer literals are often copy-pasted constants that should be named for clarity.

Known limitations:
Single local initializer literals are intentionally skipped to reduce noise.

How to fix:
Extract the repeated literal into a named constant and reuse it.

Examples:
- Replace repeated `let fee = 42; let limit = 42;` with a shared constant.

References:
- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`

### NOIR110

- Pack: `noir_core`
- Category: `maintainability`
- Maturity: `preview`
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
- Maturity: `preview`
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

