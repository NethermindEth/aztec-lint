# AZTEC001

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `stable`
- Policy: `privacy`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Private data reaches a public sink.

## What It Does

Flags flows where secret or note-derived values are emitted through public channels.

## Why This Matters

Leaking private values through public outputs can permanently expose sensitive state.

## Known Limitations

Flow analysis is conservative and may miss leaks routed through unsupported abstractions.

## How To Fix

Keep private values in constrained private paths and sanitize or avoid public emission points.

## Examples

- Avoid emitting note-derived values from public entrypoints.

## References

- `docs/suppression.md`
- `docs/rule-authoring.md`
