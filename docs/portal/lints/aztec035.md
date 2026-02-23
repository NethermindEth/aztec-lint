# AZTEC035

- Pack: `aztec_pack`
- Category: `correctness`
- Maturity: `preview`
- Policy: `correctness`
- Default Level: `warn`
- Confidence: `medium`
- Introduced In: `0.5.0`
- Lifecycle: `active`

## Summary

Suspicious repeated nested storage key.

## What It Does

Flags `.at(x).at(x)`-style nested key repetition that often indicates copy-paste key mistakes.

## Why This Matters

Repeating nested map keys unintentionally can corrupt indexing logic and authorization behavior.

## Known Limitations

Some intentionally duplicated keying patterns may require suppression when semantically correct.

## How To Fix

Use distinct key expressions for each nested `.at(...)` level or extract named key variables for clarity.

## Config Knobs

- Enable this lint via ruleset selector `profile.<name>.ruleset = ["aztec_pack"]`.
- Target this maturity in-pack via `profile.<name>.ruleset = ["aztec_pack@preview"]`.
- Target this maturity across packs via `profile.<name>.ruleset = ["tier:preview"]` (alias `maturity:preview`).
- Override this lint level in config with `profile.<name>.deny|warn|allow = ["AZTEC035"]`.
- Override this lint level in CLI with `--deny AZTEC035`, `--warn AZTEC035`, or `--allow AZTEC035`.
- Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.

## Fix Safety Notes

- `aztec-lint fix` applies only safe fixes for `AZTEC035` and skips edits marked as needing review.
- Suggestion applicability `machine-applicable` maps to safe fixes.
- Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.
- Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.


## Examples

- Replace `.at(owner).at(owner)` with the intended second key such as `.at(owner).at(spender)`.

## References

- `docs/rule-authoring.md`
- `docs/suppression.md`
