# AZTEC039

- Pack: `aztec_pack`
- Category: `correctness`
- Maturity: `preview`
- Policy: `correctness`
- Default Level: `warn`
- Confidence: `low`
- Introduced In: `0.6.0`
- Lifecycle: `active`

## Summary

Partial spend logic appears unbalanced.

## What It Does

Flags partial-spend arithmetic patterns that do not clearly reconcile consumed, spent, and change values.

## Why This Matters

Unbalanced partial-spend accounting can cause invalid state transitions or silent value drift.

## Known Limitations

Equivalent arithmetic forms may not all be recognized by pattern-driven detection.

## How To Fix

Make spend and change reconciliation explicit and assert conservation-style invariants near the transition point.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC039"]`.
- Override this lint level in CLI with `--deny AZTEC039`, `--warn AZTEC039`, or `--allow AZTEC039`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC039` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Ensure `consumed = spend + change` is enforced before emitting updated notes.

## References

- `docs/rule-authoring.md`
- `docs/suppression.md`
