# AZTEC021

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `stable`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Missing range constraints before hashing or serialization.

## What It Does

Reports values hashed or serialized without proving required numeric bounds first.

## Why This Matters

Unchecked ranges can make hash and encoding logic semantically ambiguous.

## Known Limitations

The rule cannot infer all user-defined range proof helper conventions.

## How To Fix

Apply explicit range constraints before hashing, packing, or serialization boundaries.

## Examples

- Add a range check before converting a field to a bounded integer.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
