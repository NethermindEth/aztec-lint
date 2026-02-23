# AZTEC001

- Pack: `aztec_pack`
- Category: `privacy`
- Maturity: `stable`
- Policy: `privacy`
- Default Level: `deny`
- Confidence: `medium`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Private data reaches a public sink.

## What It Does

Flags flows where secret or note-derived values are emitted through public channels.

## Why This Matters

Leaking private values through public outputs can permanently expose sensitive state.

## Known Limitations

Flow analysis is conservative and may miss leaks routed through unsupported abstractions.

## How To Fix

Keep private values in constrained private paths and sanitize or avoid public emission points.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC001"]`.
- Override this lint level in CLI with `--deny AZTEC001`, `--warn AZTEC001`, or `--allow AZTEC001`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC001` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Avoid emitting note-derived values from public entrypoints.

## References

- `docs/suppression.md`
- `docs/rule-authoring.md`
