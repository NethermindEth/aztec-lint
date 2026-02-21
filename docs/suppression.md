# Suppression Guide

This document describes lint directive behavior in `aztec-lint`.

## Supported Forms

Attach directives using canonical rule IDs:

```noir
#[allow(AZTEC010)]
fn my_fn() { ... }
```

```noir
#[warn(noir_core::NOIR100)]
mod arithmetic { ... }
```

```noir
#[deny(noir_core::NOIR100)]
fn critical_path() { ... }
```

Scoped rule IDs are also supported:

```noir
#[allow(noir_core::NOIR100)]
fn my_fn() { ... }
```

## Scope Rules

- `file-level`: directive attached at file root applies to that source file.
- `module-level`: directive attached to a module applies to its subtree.
- `item-level`: directive attached to a function/item applies only to that item.
- Matching is case-insensitive and normalized to canonical rule IDs.
- Precedence is nearest-scope first: item-level > module-level > file-level > global profile/CLI.
- If multiple directives for the same rule are declared at the same scope, last one in source order wins.

## Output Visibility

Diagnostics suppressed by `allow` include:

- `suppressed: true`
- `suppression_reason: "allow(RULE_ID)"`

Diagnostics matched by `warn` or `deny` are not suppressed; they are emitted with overridden severity.

Formatter behavior:

- Text: hidden by default; enable with `--show-suppressed`.
- JSON/SARIF: suppression metadata is emitted deterministically.

## Interaction With Filters

Directives are evaluated before confidence/severity gating.

- Exit code gating (`0`/`1`) is based only on unsuppressed diagnostics.
- `allow` diagnostics are never blocking.
- `warn`/`deny` can change whether a diagnostic passes `--severity-threshold`.

## Troubleshooting

- Suppression not taking effect:
  - Ensure the directive (`allow`/`warn`/`deny`) targets the intended scope (file/module/item).
  - Ensure the rule ID is correct (`aztec-lint rules`).
  - Check precedence when multiple directives exist for the same rule.
- Suppression not visible in text output:
  - Use `--show-suppressed`.
- Suppression visible in JSON/SARIF but not text:
  - Expected behavior; text defaults to concise output.
