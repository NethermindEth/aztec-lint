# AZTEC010

- Pack: `aztec_pack`
- Category: `protocol`
- Maturity: `stable`
- Policy: `protocol`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.1.0`
- Lifecycle: `active`

## Summary

Private to public bridge requires #[only_self].

## What It Does

Checks enqueue-based private-to-public transitions enforce self-only invocation constraints.

## Why This Matters

Missing self-only restrictions can allow unauthorized cross-context execution.

## Known Limitations

Rule coverage is scoped to known enqueue bridge patterns.

## How To Fix

Apply the configured only-self attribute and ensure bridge entrypoints enforce it.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@stable"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:stable"]` (alias `maturity:stable`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC010"]`.
- Override this lint level in CLI with `--deny AZTEC010`, `--warn AZTEC010`, or `--allow AZTEC010`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC010` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Annotate private-to-public bridge functions with #[only_self].

## References

- `docs/decisions/0001-aztec010-scope.md`
- `docs/rule-authoring.md`
