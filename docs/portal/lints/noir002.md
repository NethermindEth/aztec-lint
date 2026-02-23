# NOIR002

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Suspicious shadowing.

## What It Does

Reports variable declarations that shadow earlier bindings in the same function scope.

## Why This Matters

Shadowing can hide logic bugs by silently changing which binding is referenced.

## Known Limitations

Intentional narrow-scope shadowing may be flagged when context is ambiguous.

## How To Fix

Rename inner bindings to make value flow explicit.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["noir_core"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["noir_core@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["NOIR002"]`.
- Override this lint level in CLI with `--deny NOIR002`, `--warn NOIR002`, or `--allow NOIR002`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `NOIR002` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Use descriptive names instead of reusing accumulator variables.

## References

- `docs/rule-authoring.md`
