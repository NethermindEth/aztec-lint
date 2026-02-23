# AZTEC002

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `preview`
- Policy: `privacy`
- Default Level: `deny`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Secret-dependent branching affects public state.

## What It Does

Detects control flow where secret inputs influence public behavior.

## Why This Matters

Secret-dependent branching can reveal private information through observable behavior.

## Known Limitations

Heuristic path tracking may report false positives in complex guard patterns.

## How To Fix

Refactor logic so branch predicates for public effects are independent of private data.

## Examples

- Compute public decisions from public inputs only.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
