# ADR 0002: Suppression Semantics

Date: 2026-02-19 (updated 2026-02-21)
Status: Accepted
Owners: aztec-lint maintainers

## Context

`SPEC.md` requires suppression support with explicit rule ids and visibility in output.
The diagnostics-and-fixes track requires broadening the contract from item-local `allow` to scoped lint-level directives (`allow`, `warn`, `deny`) with deterministic precedence.

Required directive forms:

- `#[allow(AZTEC010)]`
- `#[allow(noir_core::NOIR100)]`
- `#[warn(AZTEC010)]`
- `#[deny(noir_core::NOIR100)]`

## Decision

The lint directive contract is:

1. Accepted forms:
  - `#[allow(RULE_ID)]`
  - `#[allow(PACK::RULE_ID)]`
  - `#[warn(RULE_ID)]`
  - `#[warn(PACK::RULE_ID)]`
  - `#[deny(RULE_ID)]`
  - `#[deny(PACK::RULE_ID)]`
2. Scope:
  - Item-level: directive attached to an item/function applies only to that item.
  - Module-level: directive attached to a module item applies to that module subtree.
  - File-level: directive declared at file root and not attached to a following item applies to that source file.
  - Binding is source-order deterministic: directives on the same line as an item, or directly preceding that item, are attached to that item.
3. Matching:
  - Case-insensitive match on `RULE_ID` after normalization to uppercase.
  - `PACK::RULE_ID` and `RULE_ID` both map to canonical rule id.
4. Invalid directive tokens:
  - Ignored by directive engine.
  - Optional informational diagnostic may be added later (not blocking v0).
5. Effective level resolution:
  - Baseline starts from profile/CLI global rule level.
  - Nearest scope wins: item-level > module-level > file-level > global baseline.
  - If multiple directives for the same rule exist at the same scope, last directive in source order wins.
  - `allow` suppresses the diagnostic.
  - `warn` and `deny` override the effective severity for the diagnostic without suppressing it.

## Output Contract

When a diagnostic is suppressed by `allow`, output retains the diagnostic record with suppression metadata:

- `suppressed: true`
- `suppression_reason: "allow(AZTEC010)"` (or canonical equivalent)

When a diagnostic level is overridden by `warn` or `deny`, it remains unsuppressed and is emitted with the resolved severity.

Default formatter behavior:

- Text: suppressed diagnostics hidden unless `--show-suppressed`.
- JSON/SARIF: include suppression metadata deterministically when emitted.

## Precedence Rules

Directives are resolved after rule execution and before confidence/severity threshold filtering.

Order:

1. Rule emits diagnostic.
2. Engine resolves effective level from item/module/file/global directives.
3. `allow` marks diagnostic suppressed if matched.
4. `warn`/`deny` set effective severity for unsuppressed diagnostics.
5. Confidence/severity thresholds apply only to unsuppressed diagnostics for exit code gating.

## Rationale

- Matches spec syntax.
- Keeps directive resolution deterministic and auditable.
- Enables local strictness (`deny`) without forcing repository-wide policy changes.

## Consequences

Positive:

- Clear local ownership of lint levels and suppression.
- Stable suppression metadata for CI auditing.

Negative:

- Broader scopes can hide diagnostics if used carelessly.
- Directive precedence must be tested thoroughly to avoid accidental downgrades.

## Manual Review Checklist

- [x] Required `allow`/`warn`/`deny` forms accepted
- [x] file-level/module-level/item-level scopes defined
- [x] Output visibility contract defined
- [x] Deterministic precedence order defined
- [x] Team sign-off recorded

Sign-off:

- 2026-02-21: Updated for scoped lint-level semantics.
