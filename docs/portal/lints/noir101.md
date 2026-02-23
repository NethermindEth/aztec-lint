# NOIR101

- Pack: `noir_core`
- Category: `maintainability`
- Maturity: `preview`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Repeated local initializer magic number should be named.

## What It Does

Reports repeated literal values used in plain local initializer assignments within the same function/module scope.

## Why This Matters

Repeated unexplained initializer literals are often copy-pasted constants that should be named for clarity.

## Known Limitations

Single local initializer literals are intentionally skipped to reduce noise.

## How To Fix

Extract the repeated literal into a named constant and reuse it.

## Examples

- Replace repeated `let fee = 42; let limit = 42;` with a shared constant.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
