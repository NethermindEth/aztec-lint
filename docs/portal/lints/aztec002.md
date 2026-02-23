# AZTEC002

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `preview`
- Policy: `privacy`
- Default Level: `deny`
- Confidence: `low`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Secret-dependent branching affects public state.

## What It Does

Detects control flow where secret inputs influence public behavior.

## Why This Matters

Secret-dependent branching can reveal private information through observable behavior.

## Known Limitations

Heuristic path tracking may report false positives in complex guard patterns.

## How To Fix

Refactor logic so branch predicates for public effects are independent of private data.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC002"]`.
- Override this lint level in CLI with `--deny AZTEC002`, `--warn AZTEC002`, or `--allow AZTEC002`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC002` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Compute public decisions from public inputs only.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
