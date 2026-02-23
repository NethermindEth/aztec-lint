# AZTEC040

- Pack: `aztec_pack`
- Category: `protocol`
- Maturity: `preview`
- Policy: `protocol`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.6.0`
- Lifecycle: `active`

## Summary

Initializer entrypoint missing #[only_self].

## What It Does

Reports initializer functions that are not protected by the expected only-self access restriction.

## Why This Matters

Unrestricted initializers can allow unauthorized setup flows or protocol-state takeover.

## Known Limitations

Framework-equivalent guards not expressed through the configured only-self signal may need suppression.

## How To Fix

Annotate initializer entrypoints with `#[only_self]` or move privileged initialization behind a self-only gate.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC040"]`.
- Override this lint level in CLI with `--deny AZTEC040`, `--warn AZTEC040`, or `--allow AZTEC040`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC040` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Mark contract initializer functions with `#[only_self]` before deployment use.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0001-aztec010-scope.md`
