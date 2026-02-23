# AZTEC032

- Pack: `aztec_pack`
- Category: `protocol`
- Maturity: `preview`
- Policy: `protocol`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.5.0`
- Lifecycle: `active`

## Summary

Commitment hash appears missing domain separation inputs.

## What It Does

Detects commitment-style hash sinks that do not include configured domain-separation components.

## Why This Matters

Insufficient commitment domain separation can blur security boundaries and weaken protocol assumptions.

## Known Limitations

Rule matching focuses on recognizable commitment sink names and hash-shaped inputs.

## How To Fix

Add required context fields (such as contract address and note type) to commitment hash construction.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC032"]`.
- Override this lint level in CLI with `--deny AZTEC032`, `--warn AZTEC032`, or `--allow AZTEC032`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC032` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Derive commitments with explicit domain tags instead of hashing only payload values.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
