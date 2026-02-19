# ADR 0001: AZTEC010 Scope

Date: 2026-02-19
Status: Accepted
Owners: aztec-lint maintainers

## Context

`SPEC.md` requires rule `AZTEC010`:

- "Public function called via enqueue must be `#[only_self]`"

Research in `RESEARCH.md` found valid cross-contract enqueue patterns in current Aztec code. A literal global enforcement would produce false positives on legitimate cross-contract interactions.

## Decision

`AZTEC010` will enforce `#[only_self]` only for same-contract private-to-public bridge calls.

Rule trigger contract:

1. Source call site is in a private context.
2. Call site uses `enqueue` semantics (including `self.enqueue(...)` and `self.enqueue_self.*` forms).
3. Target function resolves to a public function in the same contract.
4. Diagnostic is emitted if target function is missing `#[only_self]`.

Out of scope for `AZTEC010`:

- Cross-contract enqueue calls.
- Non-public targets.
- Cases where symbol resolution fails (defer, do not guess).

## Rationale

- Preserves high-signal default behavior.
- Aligns with real Aztec patterns where cross-contract enqueue is valid.
- Avoids noisy diagnostics from ambiguous target ownership.

## Consequences

Positive:

- Lower false-positive rate.
- Clear implementable predicate for semantic model.

Negative:

- Some risky cross-contract patterns are intentionally not covered by this rule and may require separate rules later.

## Implementation Notes

- Requires target function symbol resolution and contract ownership mapping.
- If ownership cannot be resolved deterministically, do not emit `AZTEC010`.
- Any future scope expansion requires a new ADR update.

## Manual Review Checklist

- [x] Scope constrained to same-contract bridge only
- [x] Cross-contract enqueue explicitly excluded
- [x] Failure behavior on unresolved symbols defined
- [x] Rule remains deterministic
- [x] Team sign-off recorded

Sign-off:

- 2026-02-19: Accepted for Phase 0 gate.

