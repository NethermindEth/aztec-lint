# ADR 0002: Suppression Semantics

Date: 2026-02-19
Status: Accepted
Owners: aztec-lint maintainers

## Context

`SPEC.md` requires suppressions:

- `#[allow(AZTEC010)]`
- `#[allow(noir_core::NOIR100)]`

and requires suppressions to be attached to function/item and visible in output.

## Decision

The suppression contract is:

1. Accepted forms:
- `#[allow(RULE_ID)]`
- `#[allow(PACK::RULE_ID)]`
2. Scope:
- Applies only to the annotated function or item.
- No file-level or module-level suppression in v0.
3. Matching:
- Case-insensitive match on `RULE_ID` after normalization to uppercase.
- `PACK::RULE_ID` and `RULE_ID` both map to canonical rule id.
4. Invalid suppression tokens:
- Ignored by suppression engine.
- Optional informational diagnostic may be added later (not blocking v0).

## Output Contract

When a diagnostic is suppressed, output retains the diagnostic record with suppression metadata when suppression visibility is enabled:

- `suppressed: true`
- `suppression_reason: "allow(AZTEC010)"` (or canonical equivalent)

Default formatter behavior:

- Text: suppressed diagnostics hidden unless `--show-suppressed`.
- JSON/SARIF: include suppression metadata deterministically when emitted.

## Precedence Rules

Suppression is applied after rule execution and before threshold filtering.

Order:

1. Rule emits diagnostic.
2. Item-level suppression marks diagnostic suppressed if matched.
3. Confidence/severity thresholds apply only to unsuppressed diagnostics for exit code gating.

## Rationale

- Matches spec syntax.
- Keeps suppression deterministic and local.
- Prevents accidental broad suppression.

## Consequences

Positive:

- Clear local ownership of suppression.
- Stable suppression metadata for CI auditing.

Negative:

- Users cannot suppress at file/module scope in v0.

## Manual Review Checklist

- [x] Required syntax forms accepted
- [x] Attachment scope restricted to function/item
- [x] Output visibility contract defined
- [x] Deterministic precedence order defined
- [x] Team sign-off recorded

Sign-off:

- 2026-02-19: Accepted for initial gate.
