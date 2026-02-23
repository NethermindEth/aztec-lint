# NOIR001

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Unused variable or import.

## What It Does

Detects declared bindings and imports that are not used.

## Why This Matters

Unused items can indicate dead code, mistakes, or incomplete refactors.

## Known Limitations

Generated code and macro-like patterns may trigger noisy diagnostics.

## How To Fix

Remove unused bindings or prefix intentionally unused values with an underscore.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["noir_core"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["noir_core@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["NOIR001"]`.
- Override this lint level in CLI with `--deny NOIR001`, `--warn NOIR001`, or `--allow NOIR001`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `NOIR001` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Delete unused imports after refactoring call sites.

## References

- `docs/rule-authoring.md`
