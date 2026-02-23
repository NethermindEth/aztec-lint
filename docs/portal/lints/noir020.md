# NOIR020

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Array indexing without bounds validation.

## What It Does

Detects index operations lacking an obvious preceding range constraint.

## Why This Matters

Unchecked indexing can cause invalid behavior and proof failures.

## Known Limitations

Complex index sanitization paths may not always be recognized.

## How To Fix

Establish and assert index bounds before indexing operations.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["noir_core"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["noir_core@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["NOIR020"]`.
- Override this lint level in CLI with `--deny NOIR020`, `--warn NOIR020`, or `--allow NOIR020`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `NOIR020` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Assert `idx < arr.len()` before reading `arr[idx]`.

## References

- `docs/rule-authoring.md`
