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

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["noir_core"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["noir_core@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["NOIR120"]`.
- Override this lint level in CLI with `--deny NOIR120`, `--warn NOIR120`, or `--allow NOIR120`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `NOIR120` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Refactor nested conditionals into guard clauses.

## References

- `docs/rule-authoring.md`
