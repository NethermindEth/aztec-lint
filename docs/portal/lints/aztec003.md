# AZTEC003

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `stable`
- Policy: `privacy`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Private entrypoint uses debug logging.

## What It Does

Reports debug logging in private contexts where logging may leak sensitive state.

## Why This Matters

Debug output can disclose values intended to remain private.

## Known Limitations

Custom logging wrappers are only detected when call patterns are recognizable.

## How To Fix

Remove debug logging from private code paths or replace it with safe telemetry patterns.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC003"]`.
- Override this lint level in CLI with `--deny AZTEC003`, `--warn AZTEC003`, or `--allow AZTEC003`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC003` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Do not print private witnesses in private functions.

## References

- `docs/suppression.md`
- `docs/rule-authoring.md`
