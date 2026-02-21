# ADR 0003: Confidence Model

Date: 2026-02-19
Status: Accepted
Owners: aztec-lint maintainers

## Context

The CLI includes confidence filtering (`--min-confidence`) but the model was not previously defined.

## Decision

Confidence is deterministic and rule-scoped. Each rule has a fixed default confidence:

- `high`: low-ambiguity semantic pattern, low false-positive risk
- `medium`: useful heuristic with bounded ambiguity
- `low`: exploratory/approximate signal

Initial mapping for default and planned rules:

- High:
  - `NOIR001`, `NOIR010`, `AZTEC010`, `AZTEC020`
- Medium:
  - `NOIR002`, `NOIR020`, `NOIR030`, `AZTEC001`, `AZTEC003`, `AZTEC011`, `AZTEC012`, `AZTEC021`, `AZTEC022`
- Low:
  - `NOIR100`, `NOIR110`, `NOIR120`, `NOIR200`, `AZTEC002`, `AZTEC040`, `AZTEC041`

Confidence assignment does not depend on runtime randomness or environment.

## Filtering Contract

Filtering order for emitted diagnostics:

1. Rule execution
2. Suppression evaluation
3. Confidence filter (`--min-confidence`)
4. Severity threshold filter (`--severity-threshold`)
5. Formatter output

Confidence ordering:

- `high` >= `medium` >= `low`

Meaning:

- `--min-confidence high`: include only high
- `--min-confidence medium`: include high and medium
- `--min-confidence low`: include all

## Rationale

- Simple deterministic policy aligned with CI reproducibility.
- Stable behavior across runs and machines.
- Gives users immediate signal/noise control.

## Consequences

Positive:

- Predictable filtering behavior.
- Easy to document and test.

Negative:

- Single fixed confidence per rule can be coarse for edge cases.

## Manual Review Checklist

- [x] Confidence levels and ordering defined
- [x] Rule-to-confidence mapping defined
- [x] Filter stage position defined
- [x] Determinism requirement explicit
- [x] Team sign-off recorded

Sign-off:

- 2026-02-19: Accepted for initial gate.
