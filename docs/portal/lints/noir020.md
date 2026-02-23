# NOIR020

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Array indexing without bounds validation.

## What It Does

Detects index operations lacking an obvious preceding range constraint.

## Why This Matters

Unchecked indexing can cause invalid behavior and proof failures.

## Known Limitations

Complex index sanitization paths may not always be recognized.

## How To Fix

Establish and assert index bounds before indexing operations.

## Examples

- Assert `idx < arr.len()` before reading `arr[idx]`.

## References

- `docs/rule-authoring.md`
