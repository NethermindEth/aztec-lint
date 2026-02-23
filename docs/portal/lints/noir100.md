# NOIR100

- Pack: `noir_core`
- Category: `maintainability`
- Maturity: `stable`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Magic number literal should be named.

## What It Does

Detects high-signal numeric literals used in branch/assert/hash/serialization and related protocol-sensitive contexts.

## Why This Matters

Named constants improve readability and reduce accidental misuse.

## Known Limitations

Low-signal plain local initializer literals are intentionally excluded from this rule.

## How To Fix

Define a constant with domain meaning and use it in place of the literal.

## Examples

- Replace `42` with `MAX_NOTES_PER_BATCH`.

## References

- `docs/rule-authoring.md`
