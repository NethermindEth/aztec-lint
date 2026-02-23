# NOIR010

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Boolean computed but not asserted.

## What It Does

Flags boolean expressions that appear intended for checks but never drive an assertion.

## Why This Matters

Forgotten assertions can leave critical invariants unenforced.

## Known Limitations

Rules cannot always infer whether an unasserted boolean is intentionally stored for later use.

## How To Fix

Use assert-style checks where the boolean is intended as a safety or validity guard.

## Examples

- Convert an unconsumed `is_valid` expression into an assertion.

## References

- `docs/rule-authoring.md`
