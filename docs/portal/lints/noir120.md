# NOIR120

- Pack: `noir_core`
- Category: `maintainability`
- Maturity: `preview`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Function nesting depth exceeds threshold.

## What It Does

Flags deeply nested control flow that reduces readability and maintainability.

## Why This Matters

Deep nesting increases cognitive load and maintenance risk.

## Known Limitations

Certain generated or domain-specific patterns can be naturally nested.

## How To Fix

Use early returns and helper functions to flatten nested control flow.

## Examples

- Refactor nested conditionals into guard clauses.

## References

- `docs/rule-authoring.md`
