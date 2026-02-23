# AZTEC034

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `preview`
- Policy: `soundness`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.5.0`
- Lifecycle: `active`

## Summary

Hash input cast to Field without prior range guard.

## What It Does

Finds hash inputs that are cast or converted to Field without an earlier range-style constraint.

## Why This Matters

Missing range proofs can make hashed representations ambiguous for bounded integer semantics.

## Known Limitations

Nearby helper-based constraints may not be recognized when they do not resemble explicit range checks.

## How To Fix

Constrain numeric width before Field conversion and hashing, then keep the guarded value flow explicit.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC034"]`.
- Override this lint level in CLI with `--deny AZTEC034`, `--warn AZTEC034`, or `--allow AZTEC034`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC034` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Assert bounded `amount` before hashing `amount as Field`.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
