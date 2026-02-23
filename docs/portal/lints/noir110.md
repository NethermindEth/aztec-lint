# NOIR110

- Pack: `noir_core`
- Category: `maintainability`
- Maturity: `preview`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Function complexity exceeds threshold.

## What It Does

Flags functions whose control flow complexity passes the configured limit.

## Why This Matters

High complexity makes correctness and audits harder.

## Known Limitations

Simple metric thresholds cannot capture all maintainability nuances.

## How To Fix

Split large functions and isolate complex branches into focused helpers.

## Examples

- Extract nested decision trees into named helper functions.

## References

- `docs/rule-authoring.md`
