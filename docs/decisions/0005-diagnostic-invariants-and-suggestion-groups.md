# ADR 0005: Diagnostic Invariants and Suggestion Groups

Date: 2026-02-21
Status: Accepted
Owners: aztec-lint maintainers

## Context

The diagnostics-and-fixes track requires stronger diagnostic contracts, richer machine-applicable suggestions, and compatibility-safe output evolution for JSON/SARIF consumers.

Current behavior allows flattened suggestions and per-edit fix handling without a first-class grouped model or explicit invariant gate.

## Decision

1. Diagnostic invariants are mandatory before output/fix pipelines:
  - `rule_id`, `policy`, and `message` must be non-empty.
  - All spans must satisfy `start <= end`.
  - Suppressed diagnostics must include `suppression_reason`.
  - Grouped suggestion edits must be valid, same-file, and non-overlapping within the group.
2. Suggestion groups are first-class:
  - A suggestion group contains one or more edits and shared metadata (`id`, message, applicability, provenance).
  - Multipart suggestions are represented as one grouped suggestion, not flattened independent edits.
3. Fix application semantics align with grouped model:
  - A grouped suggestion is applied as one transaction (all edits applied or none).
  - Partial application of a grouped suggestion is invalid.
4. Output compatibility policy for migration:
  - JSON and SARIF emit new grouped suggestion data and keep legacy fields during the migration window.
  - Migration window starts 2026-02-21 and remains active until a follow-up ADR explicitly schedules removal.
  - Legacy fields must be deterministically derived from the grouped canonical model to prevent drift.
  - Consumers should prefer grouped fields when present; legacy fields remain compatibility-only.
5. Any invariant violation is treated as an internal contract error and must surface through deterministic internal-error handling.

## Compatibility Contract

During migration:

- JSON:
  - Emit `suggestion_groups` as canonical machine-applicable structure.
  - Also emit legacy suggestion/fix fields for backward compatibility.
- SARIF:
  - Map each suggestion group to one SARIF fix object with one or multiple replacements.
  - Preserve deterministic ordering and stable identifiers.

No compatibility field may carry contradictory edit data relative to the grouped canonical model.

## Rationale

- Prevents malformed diagnostics from reaching users and CI integrations.
- Enables multi-location safe autofix without breaking existing consumers.
- Makes migration explicit and testable.

## Consequences

Positive:

- Stronger diagnostics guarantees and safer fix behavior.
- Backward-compatible output transition path.

Negative:

- Additional validation and serialization complexity.
- Temporary duplication while legacy fields remain.

## Manual Review Checklist

- [x] Diagnostic invariants explicitly defined
- [x] Grouped suggestion model defined
- [x] Transaction semantics for grouped fixes defined
- [x] JSON/SARIF compatibility migration policy defined
- [x] Team sign-off recorded

Sign-off:

- 2026-02-21: Accepted for diagnostics and grouped suggestions.
