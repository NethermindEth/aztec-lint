# AZTEC041

- Pack: `aztec_pack`
- Category: `correctness`
- Maturity: `preview`
- Policy: `correctness`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.6.0`
- Lifecycle: `active`

## Summary

Field/integer cast may truncate or wrap unexpectedly.

## What It Does

Finds cast patterns between Field and bounded integers that lack nearby guard conditions proving safe range.

## Why This Matters

Unchecked narrowing conversions can silently corrupt values and invalidate downstream protocol logic.

## Known Limitations

Guard recognition focuses on known range-check idioms and may miss custom helper abstractions.

## How To Fix

Add explicit range checks before narrowing casts and keep the guarded value flow local and visible.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC041"]`.
- Override this lint level in CLI with `--deny AZTEC041`, `--warn AZTEC041`, or `--allow AZTEC041`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC041` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Assert value bounds before converting `Field` into a narrower integer type.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
