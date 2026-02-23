# AZTEC033

- Pack: `aztec_pack`
- Category: `protocol`
- Maturity: `preview`
- Policy: `protocol`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.5.0`
- Lifecycle: `active`

## Summary

Public entrypoint mutates private state without #[only_self].

## What It Does

Reports public entrypoints that appear to mutate private note/state transitions and lack only-self protection.

## Why This Matters

Publicly callable private-state mutation surfaces can break intended access boundaries.

## Known Limitations

Detection relies on recognized mutation patterns and may not cover every custom state transition helper.

## How To Fix

Add `#[only_self]` to the public entrypoint or refactor the mutation into a safer private flow.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC033"]`.
- Override this lint level in CLI with `--deny AZTEC033`, `--warn AZTEC033`, or `--allow AZTEC033`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC033` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Mark public state-transition bridges with `#[only_self]` before calling note mutation APIs.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0001-aztec010-scope.md`
