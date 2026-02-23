# NOIR030

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Unconstrained value influences constrained logic.

## What It Does

Reports suspicious influence of unconstrained data over constrained computation paths.

## Why This Matters

Mixing unconstrained and constrained logic can invalidate proof assumptions.

## Known Limitations

Inference can be conservative for deeply indirect data flow.

## How To Fix

Constrain values before they participate in constrained branches or outputs.

## Examples

- Introduce explicit constraints at trust boundaries.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
