# AZTEC020

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `stable`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Unconstrained influence reaches commitments, storage, or nullifiers.

## What It Does

Detects unconstrained values that affect constrained Aztec protocol artifacts.

## Why This Matters

Unconstrained influence can break proof soundness and on-chain validity assumptions.

## Known Limitations

Transitive influence through unsupported helper layers may be missed.

## How To Fix

Introduce explicit constraints before values affect commitments, storage, or nullifiers.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC020"]`.
- Override this lint level in CLI with `--deny AZTEC020`, `--warn AZTEC020`, or `--allow AZTEC020`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC020` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Constrain intermediate values before writing storage commitments.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
