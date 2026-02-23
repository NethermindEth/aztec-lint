# AZTEC038

- Pack: `aztec_pack`
- Category: `correctness`
- Maturity: `preview`
- Policy: `correctness`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.6.0`
- Lifecycle: `active`

## Summary

Change note appears to miss fresh randomness.

## What It Does

Detects change-note construction patterns that appear to reuse deterministic randomness or omit freshness inputs.

## Why This Matters

Weak randomness freshness can increase linkage risk and break expected note uniqueness properties.

## Known Limitations

Freshness detection is heuristic and may miss user-defined entropy helper conventions.

## How To Fix

Derive change-note randomness from a fresh, non-reused source and thread it explicitly into note construction.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC038"]`.
- Override this lint level in CLI with `--deny AZTEC038`, `--warn AZTEC038`, or `--allow AZTEC038`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC038` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Use a per-note fresh randomness value instead of reusing an existing note nonce.

## References

- `docs/rule-authoring.md`
- `docs/suppression.md`
