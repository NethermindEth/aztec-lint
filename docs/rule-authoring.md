# Rule Authoring Guide

This guide covers the minimum contract for adding a new lint rule.

## 1. Implement `Rule`

Create a rule type in `crates/aztec-lint-rules/src/<pack>/` and implement:

```rust
pub trait Rule {
    fn id(&self) -> &'static str;
    fn run(&self, ctx: &RuleContext<'_>, out: &mut Vec<Diagnostic>);
}
```

Use `ctx.diagnostic(...)` to create base diagnostics.

## 2. Register Metadata

Add the rule in `crates/aztec-lint-rules/src/engine/registry.rs` with:

- canonical `id`
- `pack`
- `policy`
- default `RuleLevel`
- deterministic `Confidence`

Engine metadata is authoritative for severity/confidence/policy assignment.

## 3. Directive/Suppression Contract

Do not implement directive handling inside a rule.
Rules must not parse source-level `allow`/`warn`/`deny` attributes directly.

Directive resolution is handled centrally in the engine using scoped levels (`allow`, `warn`, `deny`) with precedence:

- item-level > module-level > file-level > global profile/CLI baseline

Engine sets:

- `diagnostic.suppressed`
- `diagnostic.suppression_reason`

Rules should always emit the true positive diagnostic. Suppression and severity overrides are applied later by the engine.

## 4. Optional Autofix Contract

If a rule emits suggestions/fixes:

- use helper APIs (`span_suggestion`, `multipart_suggestion`) to build suggestion groups
- mark only deterministic edits as machine-applicable / `FixSafety::Safe`
- mark non-trivial edits as `FixSafety::NeedsReview`
- avoid cross-file edits in one diagnostic
- avoid overlapping edits inside a grouped suggestion

`aztec-lint fix` applies safe candidates and treats grouped candidates as a single transaction (all-or-none).

During compatibility migration, avoid manually diverging legacy fields from grouped suggestion data. Prefer canonical helper constructors so JSON/SARIF compatibility fields stay deterministic.

## 5. Testing Requirements

For every new rule:

- positive fixture test
- negative fixture test (false-positive guard)
- suppression fixture test
- deterministic ordering test if multiple diagnostics are possible

If rule emits fixes, add:

- idempotence test (`fix` twice => no further changes)
- invalid-span rejection test
- overlap resolution test if overlapping fixes are possible
- transaction test for multipart/grouped edits (partial apply must not occur)

## 6. Operational Notes

- `--changed-only` filtering is applied after diagnostics are produced.
- Rule behavior must not depend on filesystem traversal order.
- Prefer stable sort keys and explicit spans.

## 7. Semantic-First Policy (Correctness/Soundness)

- Correctness and soundness rules must use typed semantic facts (`ctx.semantic_model()` / `ctx.query()`) as the primary signal.
- Text heuristics are allowed only as explicit fallback when required semantic facts are unavailable.
- Keep fallback paths isolated and named as fallback (for example `text_fallback_*` or `fallback_*`) so reviewers can identify them quickly.
- Do not gate final decisions on raw line text when semantic equivalents exist.
- If fallback is used, add/keep tests that cover both semantic and fallback paths.
