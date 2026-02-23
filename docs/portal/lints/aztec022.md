# AZTEC022

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `stable`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Suspicious Merkle witness usage.

## What It Does

Finds witness handling patterns that likely violate expected Merkle proof semantics.

## Why This Matters

Incorrect witness usage can invalidate inclusion guarantees.

## Known Limitations

Complex custom witness manipulation may produce conservative warnings.

## How To Fix

Verify witness ordering and path semantics against the target Merkle API contract.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC022"]`.
- Override this lint level in CLI with `--deny AZTEC022`, `--warn AZTEC022`, or `--allow AZTEC022`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC022` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Ensure witness paths and leaf values are paired using the expected order.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
