# NOIR030

- Pack: `noir_core`
- Category: `correctness`
- Maturity: `stable`
- Policy: `correctness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Unconstrained value influences constrained logic.

## What It Does

Reports suspicious influence of unconstrained data over constrained computation paths.

## Why This Matters

Mixing unconstrained and constrained logic can invalidate proof assumptions.

## Known Limitations

Inference can be conservative for deeply indirect data flow.

## How To Fix

Constrain values before they participate in constrained branches or outputs.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["noir_core"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["noir_core@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["NOIR030"]`.
- Override this lint level in CLI with `--deny NOIR030`, `--warn NOIR030`, or `--allow NOIR030`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `NOIR030` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Introduce explicit constraints at trust boundaries.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
