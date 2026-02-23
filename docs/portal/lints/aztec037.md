# AZTEC037

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `preview`
- Policy: `privacy`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.6.0`
- Lifecycle: `active`

## Summary

Secret-dependent branch affects delivery count.

## What It Does

Reports branch-dependent behavior where secret inputs influence the number or presence of delivery-style effects.

## Why This Matters

Varying delivery cardinality on secret predicates can reveal private state through externally visible behavior.

## Known Limitations

Delivery sink coverage is currently scoped to recognized call patterns.

## How To Fix

Keep delivery count and emission structure invariant with respect to secret branch conditions.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC037"]`.
- Override this lint level in CLI with `--deny AZTEC037`, `--warn AZTEC037`, or `--allow AZTEC037`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC037` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Avoid conditional delivery emission based on private note values.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
