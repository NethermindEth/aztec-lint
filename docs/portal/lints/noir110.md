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

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["noir_core"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["noir_core@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["NOIR110"]`.
- Override this lint level in CLI with `--deny NOIR110`, `--warn NOIR110`, or `--allow NOIR110`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `NOIR110` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Extract nested decision trees into named helper functions.

## References

- `docs/rule-authoring.md`
