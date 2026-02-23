# AZTEC021

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `stable`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Missing range constraints before hashing or serialization.

## What It Does

Reports values hashed or serialized without proving required numeric bounds first.

## Why This Matters

Unchecked ranges can make hash and encoding logic semantically ambiguous.

## Known Limitations

The rule cannot infer all user-defined range proof helper conventions.

## How To Fix

Apply explicit range constraints before hashing, packing, or serialization boundaries.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC021"]`.
- Override this lint level in CLI with `--deny AZTEC021`, `--warn AZTEC021`, or `--allow AZTEC021`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC021` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Add a range check before converting a field to a bounded integer.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
