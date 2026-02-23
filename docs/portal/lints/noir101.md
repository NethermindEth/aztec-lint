# NOIR101

- Pack: `noir_core`
- Category: `maintainability`
- Maturity: `preview`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Repeated local initializer magic number should be named.

## What It Does

Reports repeated literal values used in plain local initializer assignments within the same function/module scope.

## Why This Matters

Repeated unexplained initializer literals are often copy-pasted constants that should be named for clarity.

## Known Limitations

Single local initializer literals are intentionally skipped to reduce noise.

## How To Fix

Extract the repeated literal into a named constant and reuse it.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["noir_core"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["noir_core@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["NOIR101"]`.
- Override this lint level in CLI with `--deny NOIR101`, `--warn NOIR101`, or `--allow NOIR101`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `NOIR101` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Replace repeated `let fee = 42; let limit = 42;` with a shared constant.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
