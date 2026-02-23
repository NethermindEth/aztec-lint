# NOIR100

- Pack: `noir_core`
- Category: `maintainability`
- Maturity: `stable`
- Policy: `maintainability`
- Default Level: `warn`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Magic number literal should be named.

## What It Does

Detects high-signal numeric literals used in branch/assert/hash/serialization and related protocol-sensitive contexts.

## Why This Matters

Named constants improve readability and reduce accidental misuse.

## Known Limitations

Low-signal plain local initializer literals are intentionally excluded from this rule.

## How To Fix

Define a constant with domain meaning and use it in place of the literal.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["noir_core"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["noir_core@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["NOIR100"]`.
- Override this lint level in CLI with `--deny NOIR100`, `--warn NOIR100`, or `--allow NOIR100`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `NOIR100` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Replace `42` with `MAX_NOTES_PER_BATCH`.

## References

- `docs/rule-authoring.md`
