# NOIR010

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Boolean computed but not asserted.

## What It Does

Flags boolean expressions that appear intended for checks but never drive an assertion.

## Why This Matters

Forgotten assertions can leave critical invariants unenforced.

## Known Limitations

Rules cannot always infer whether an unasserted boolean is intentionally stored for later use.

## How To Fix

Use assert-style checks where the boolean is intended as a safety or validity guard.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["noir_core"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["noir_core@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["NOIR010"]`.
- Override this lint level in CLI with `--deny NOIR010`, `--warn NOIR010`, or `--allow NOIR010`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `NOIR010` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Convert an unconsumed `is_valid` expression into an assertion.

## References

- `docs/rule-authoring.md`
