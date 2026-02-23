# AZTEC020

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `stable`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Unconstrained influence reaches commitments, storage, or nullifiers.

## What It Does

Detects unconstrained values that affect constrained Aztec protocol artifacts.

## Why This Matters

Unconstrained influence can break proof soundness and on-chain validity assumptions.

## Known Limitations

Transitive influence through unsupported helper layers may be missed.

## How To Fix

Introduce explicit constraints before values affect commitments, storage, or nullifiers.

## Examples

- Constrain intermediate values before writing storage commitments.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
