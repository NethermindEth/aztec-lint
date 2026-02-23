# Configuration Reference

This page documents all TOML configuration keys supported by `aztec-lint`.

Code source of truth:
- `crates/aztec-lint-core/src/config/loader.rs`
- `crates/aztec-lint-core/src/config/types.rs`

## File Discovery

Config is loaded from the target root directory in this order:

1. `aztec-lint.toml` (primary)
2. `noir-lint.toml` (fallback)
3. Built-in defaults if neither file exists

If both files exist, `aztec-lint.toml` wins.

## Top-Level Schema

Supported top-level tables:

- `[profile.<name>]` (repeatable, dynamic profile names)
- `[aztec]`
- `[aztec.domain_separation]`
- `[deprecated_path]`

## Built-in Profiles

If you do not override them, built-in profiles are:

- `default`: `ruleset = ["noir_core"]`
- `noir`: `extends = ["default"]`
- `aztec`: `extends = ["default"]`, `ruleset = ["aztec_pack"]`
- `aztec_strict`: `extends = ["aztec"]`, `ruleset = ["aztec_pack@preview", "aztec_pack@experimental"]`

User-defined `[profile.<name>]` entries are merged onto built-ins by profile name.

## `[profile.<name>]` Keys

| Key | Type | Default | Notes |
|---|---|---|---|
| `extends` | `array<string>` | `[]` | Parent profiles. Supports multi-parent inheritance. |
| `ruleset` | `array<string>` | `[]` | Ruleset selectors. |
| `deny` | `array<string>` | `[]` | Force rule level to deny. |
| `warn` | `array<string>` | `[]` | Force rule level to warn. |
| `allow` | `array<string>` | `[]` | Force rule level to allow. |

### Ruleset selector forms

- `<pack>` (example: `noir_core`, `aztec_pack`)
- `tier:<stable|preview|experimental>`
- `maturity:<stable|preview|experimental>` (alias of `tier:`)
- `<pack>@<stable|preview|experimental>` (example: `aztec_pack@preview`)

### Rule ID override behavior

- Rule IDs are normalized case-insensitively (`noir120` works).
- Unknown IDs fail fast.
- Deprecated/renamed IDs fail fast with a suggested replacement when available.
- Conflicting levels for the same rule in the same override scope fail fast (example: same rule in both `allow` and `deny`).

### Resolution and precedence

Final effective levels are computed in this order:

1. Ruleset defaults from resolved profile rulesets
2. Profile overrides (`allow` then `warn` then `deny`) in inheritance order from parent to child
3. CLI overrides (`--allow/--warn/--deny`) last

Within a profile inheritance chain, profile-level overrides from the child profile can override parent profile overrides.

## `[aztec]` Keys

| Key | Type | Default |
|---|---|---|
| `contract_attribute` | `string` | `"aztec"` |
| `external_attribute` | `string` | `"external"` |
| `external_kinds` | `array<string>` | `["public", "private"]` |
| `only_self_attribute` | `string` | `"only_self"` |
| `initializer_attribute` | `string` | `"initializer"` |
| `storage_attribute` | `string` | `"storage"` |
| `imports_prefixes` | `array<string>` | `["aztec", "::aztec"]` |
| `note_getter_fns` | `array<string>` | `["get_notes"]` |
| `nullifier_fns` | `array<string>` | `["emit_nullifier", "nullify"]` |
| `enqueue_fn` | `string` | `"enqueue"` |
| `contract_at_fn` | `string` | `"at"` |

These keys tune Aztec semantic-name detection used by Aztec-specific rules.

## `[aztec.domain_separation]` Keys

| Key | Type | Default |
|---|---|---|
| `nullifier_requires` | `array<string>` | `["contract_address", "nonce"]` |
| `commitment_requires` | `array<string>` | `["contract_address", "note_type"]` |

These keys define required domain-separation components for domain-separation rules.

## `[deprecated_path]` Keys

| Key | Type | Default | Notes |
|---|---|---|---|
| `warn_on_blocked` | `bool` | `false` | Emit warning when deprecated-path rewrite is blocked. |
| `try_absolute_root` | `bool` | `true` | Try absolute-root rewrite strategy. |
| `verbose_blocked_notes` | `bool` | `false` | Emit extra blocked-note detail. |

## Complete Example

```toml
[profile.default]
ruleset = ["noir_core"]

[profile.aztec]
extends = ["default"]
ruleset = ["aztec_pack"]

[profile.ci]
extends = ["aztec"]
ruleset = ["aztec_pack@preview"]
deny = ["NOIR001"]
warn = ["NOIR120"]
allow = ["AZTEC003"]

[aztec]
contract_attribute = "aztec"
external_attribute = "external"
external_kinds = ["public", "private"]
only_self_attribute = "only_self"
initializer_attribute = "initializer"
storage_attribute = "storage"
imports_prefixes = ["aztec", "::aztec"]
note_getter_fns = ["get_notes"]
nullifier_fns = ["emit_nullifier", "nullify"]
enqueue_fn = "enqueue"
contract_at_fn = "at"

[aztec.domain_separation]
nullifier_requires = ["contract_address", "nonce"]
commitment_requires = ["contract_address", "note_type"]

[deprecated_path]
warn_on_blocked = false
try_absolute_root = true
verbose_blocked_notes = false
```

## Common Config Errors

- Unknown profile in `--profile`: profile not found.
- Unknown parent in `extends`: parent profile not found.
- Inheritance cycle in `extends`: profile cycle detected.
- Invalid `ruleset` selector: unknown ruleset.
- Unknown or retired rule ID in overrides: unknown rule ID (with replacement hint when available).
- Conflicting override levels for one rule in the same scope: conflicting rule override.
