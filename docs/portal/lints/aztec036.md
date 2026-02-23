# AZTEC036

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `preview`
- Policy: `privacy`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.6.0`
- Lifecycle: `active`

## Summary

Secret-dependent branch affects enqueue behavior.

## What It Does

Flags private or secret-influenced branching that changes whether or how enqueue-style bridge calls are emitted.

## Why This Matters

Observer-visible enqueue shape differences can leak private branch decisions.

## Known Limitations

Pattern matching is currently heuristic and may not cover every custom enqueue wrapper.

## How To Fix

Refactor enqueue behavior so public bridge decisions are independent of secret branch predicates.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC036"]`.
- Override this lint level in CLI with `--deny AZTEC036`, `--warn AZTEC036`, or `--allow AZTEC036`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC036` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Emit a fixed enqueue pattern and move secret-dependent logic into constrained private computation.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
