# AZTEC030

- Pack: `aztec_pack`
- Category: `soundness`
- Maturity: `preview`
- Policy: `soundness`
- Default Level: `deny`
- Confidence: `high`
- Introduced In: `0.5.0`
- Lifecycle: `active`

## Summary

Note consumption without nullifier emission.

## What It Does

Reports note pop/consume patterns when the same function does not emit a nullifier.

## Why This Matters

Consumed notes without nullifiers can enable replay or double-spend style state inconsistencies.

## Known Limitations

Function-local matching does not prove path-complete nullifier coverage in highly dynamic control flow.

## How To Fix

Emit nullifiers for consumed notes or switch to helper APIs that enforce consume-and-nullify semantics.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC030"]`.
- Override this lint level in CLI with `--deny AZTEC030`, `--warn AZTEC030`, or `--allow AZTEC030`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC030` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- After `pop_note` or `pop_notes`, emit the associated nullifier in the same function path.

## References

- `docs/rule-authoring.md`
- `docs/decisions/0003-confidence-model.md`
