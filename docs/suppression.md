# Suppression Guide

This document describes suppression behavior in `aztec-lint`.

## Supported Forms

Attach suppressions directly to the item/function:

```noir
#[allow(AZTEC010)]
fn my_fn() { ... }
```

or scoped form:

```noir
#[allow(noir_core::NOIR100)]
fn my_fn() { ... }
```

## Scope Rules

- Scope is item-local only.
- File-level and module-level suppression are not supported in v0.
- Matching is case-insensitive and normalized to canonical rule IDs.

## Output Visibility

Suppressed diagnostics include:

- `suppressed: true`
- `suppression_reason: "allow(RULE_ID)"`

Formatter behavior:

- Text: hidden by default; enable with `--show-suppressed`.
- JSON/SARIF: suppression metadata is emitted deterministically.

## Interaction With Filters

Suppression is evaluated before confidence/severity gating.

- Exit code gating (`0`/`1`) is based only on unsuppressed diagnostics.
- Suppressed diagnostics are never blocking.

## Troubleshooting

- Suppression not taking effect:
  - Ensure the `#[allow(...)]` is attached to the same item that contains the diagnostic.
  - Ensure the rule ID is correct (`aztec-lint rules`).
- Suppression not visible in text output:
  - Use `--show-suppressed`.
- Suppression visible in JSON/SARIF but not text:
  - Expected behavior; text defaults to concise output.
