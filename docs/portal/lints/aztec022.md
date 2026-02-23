# AZTEC022

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `stable`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Suspicious Merkle witness usage.

## What It Does

Finds witness handling patterns that likely violate expected Merkle proof semantics.

## Why This Matters

Incorrect witness usage can invalidate inclusion guarantees.

## Known Limitations

Complex custom witness manipulation may produce conservative warnings.

## How To Fix

Verify witness ordering and path semantics against the target Merkle API contract.

## Examples

- Ensure witness paths and leaf values are paired using the expected order.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
