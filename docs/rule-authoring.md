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

## 3. Suppression Contract

Do not implement suppression inside a rule.

Suppression is handled centrally in the engine and sets:

- `diagnostic.suppressed`
- `diagnostic.suppression_reason`

Rules should always emit the true positive diagnostic; suppression is applied later.

## 4. Optional Autofix Contract

If a rule emits fixes (`diagnostic.fixes`):

- mark only deterministic, span-local edits as `FixSafety::Safe`
- mark non-trivial edits as `FixSafety::NeedsReview`
- avoid overlapping/multi-file edits in one diagnostic

`aztec-lint fix` only applies safe fixes in v0.

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

## 6. Operational Notes

- `--changed-only` filtering is applied after diagnostics are produced.
- Rule behavior must not depend on filesystem traversal order.
- Prefer stable sort keys and explicit spans.
