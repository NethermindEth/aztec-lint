# NOIR002

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Suspicious shadowing.

## What It Does

Reports variable declarations that shadow earlier bindings in the same function scope.

## Why This Matters

Shadowing can hide logic bugs by silently changing which binding is referenced.

## Known Limitations

Intentional narrow-scope shadowing may be flagged when context is ambiguous.

## How To Fix

Rename inner bindings to make value flow explicit.

## Examples

- Use descriptive names instead of reusing accumulator variables.

## References

- `docs/rule-authoring.md`
