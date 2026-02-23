# AZTEC003

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `stable`
- Policy: `privacy`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Private entrypoint uses debug logging.

## What It Does

Reports debug logging in private contexts where logging may leak sensitive state.

## Why This Matters

Debug output can disclose values intended to remain private.

## Known Limitations

Custom logging wrappers are only detected when call patterns are recognizable.

## How To Fix

Remove debug logging from private code paths or replace it with safe telemetry patterns.

## Examples

- Do not print private witnesses in private functions.

## References

- `docs/suppression.md`
- `docs/rule-authoring.md`
