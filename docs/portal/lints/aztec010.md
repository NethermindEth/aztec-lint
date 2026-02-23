# AZTEC010

- Pack: `aztec_pack`
- Category: `protocol`
- Maturity: `stable`
- Policy: `protocol`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Private to public bridge requires #[only_self].

## What It Does

Checks enqueue-based private-to-public transitions enforce self-only invocation constraints.

## Why This Matters

Missing self-only restrictions can allow unauthorized cross-context execution.

## Known Limitations

Rule coverage is scoped to known enqueue bridge patterns.

## How To Fix

Apply the configured only-self attribute and ensure bridge entrypoints enforce it.

## Examples

- Annotate private-to-public bridge functions with #[only_self].

## References

- `docs/decisions/0001-aztec010-scope.md`
- `docs/rule-authoring.md`
