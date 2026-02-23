# NOIR001

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Unused variable or import.

## What It Does

Detects declared bindings and imports that are not used.

## Why This Matters

Unused items can indicate dead code, mistakes, or incomplete refactors.

## Known Limitations

Generated code and macro-like patterns may trigger noisy diagnostics.

## How To Fix

Remove unused bindings or prefix intentionally unused values with an underscore.

## Examples

- Delete unused imports after refactoring call sites.

## References

- `docs/rule-authoring.md`
