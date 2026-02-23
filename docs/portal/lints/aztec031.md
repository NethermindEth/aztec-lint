# AZTEC031

- Pack: `aztec_pack`
- Category: `protocol`
- Maturity: `preview`
- Policy: `protocol`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.5.0`
- Lifecycle: `active`

## Summary

Nullifier hash appears missing domain separation inputs.

## What It Does

Flags nullifier hash call sites where required domain components are not present in hash inputs.

## Why This Matters

Weak nullifier domain separation can cause collisions across domains or protocol contexts.

## Known Limitations

Heuristic token matching may miss custom domain-separation helpers or aliases.

## How To Fix

Include configured domain fields (for example contract address and nonce) in nullifier hash inputs.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC031"]`.
- Override this lint level in CLI with `--deny AZTEC031`, `--warn AZTEC031`, or `--allow AZTEC031`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC031` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Include `this_address` and `nonce` (or equivalent fields) in the nullifier hash tuple.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
