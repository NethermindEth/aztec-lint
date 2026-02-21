# aztec-lint

`aztec-lint` is a deterministic linter for Noir projects with Aztec-specific static analysis.

It helps teams catch correctness, privacy, protocol, soundness, and maintainability issues before code lands in CI or production.

## What This Repo Does

This repository contains:

- A CLI (`aztec-lint`) for checking, fixing, and explaining lints.
- A canonical lint metadata catalog (single source of truth for lint IDs, levels, docs, lifecycle).
- A runtime rule engine and lint implementations for Noir core + Aztec patterns.
- Deterministic output formatters for text, JSON, and SARIF.
- Project/config loading, suppression handling, fix application, and CI-focused behavior.

## Workspace Crates

- `crates/aztec-lint-cli`: CLI entrypoint and command handling.
- `crates/aztec-lint-core`: config, diagnostics, lint catalog metadata, output/fix infrastructure.
- `crates/aztec-lint-rules`: runtime rules and engine orchestration.
- `crates/aztec-lint-aztec`: Aztec semantic modeling helpers.
- `crates/aztec-lint-sdk`: plugin-facing API surface.

## Install (curl)

Install latest release:

```bash
curl -fsSL https://raw.githubusercontent.com/NethermindEth/aztec-lint/main/scripts/install.sh | bash
```

Install a pinned release tag:

```bash
curl -fsSL https://raw.githubusercontent.com/NethermindEth/aztec-lint/main/scripts/install.sh | bash -s -- v0.1.0
```

Default install location is `~/.local/bin/aztec-lint`.  
Override with `AZTEC_LINT_INSTALL_DIR=/your/bin/path`.

Published binary targets:

- `linux-x86_64`
- `macos-x86_64`
- `macos-aarch64`
- `windows-x86_64` (zip archive)

## Quick Start

Prerequisites:

- Rust toolchain compatible with workspace (`edition = 2024`, `rust-version = 1.93.0` in `Cargo.toml`).

Build:

```bash
cargo build --workspace
```

Run in the current directory:

```bash
cargo run -p aztec-lint-cli --bin aztec-lint -- check
```

Run on a specific project directory:

```bash
cargo run -p aztec-lint-cli --bin aztec-lint -- check /path/to/noir-project
```

Default invocation (`check` mode implicitly):

```bash
cargo run -p aztec-lint-cli --bin aztec-lint -- /path/to/noir-project
```

## Available Commands

Use `aztec-lint --help` for full CLI help.
`PATH` is optional for default mode, `check`, `fix`, and `aztec scan`; omitted `PATH` defaults to `.`.

| Command | Purpose | Example |
|---|---|---|
| `aztec-lint [PATH]` | Default check mode. Equivalent to `check` with `--profile aztec` unless overridden. | `aztec-lint .` |
| `aztec-lint check [PATH]` | Run lint analysis and report diagnostics (`PATH` defaults to `.`). | `aztec-lint check --format sarif` |
| `aztec-lint fix [PATH]` | Apply safe fixes where possible, then re-run analysis (`PATH` defaults to `.`). | `aztec-lint fix` |
| `aztec-lint fix [PATH] --dry-run` | Preview fix candidates without file writes (`PATH` defaults to `.`). | `aztec-lint fix --dry-run` |
| `aztec-lint rules` | List active lint catalog with summary metadata. | `aztec-lint rules` |
| `aztec-lint explain <RULE_ID>` | Show full documentation for one lint. | `aztec-lint explain AZTEC010` |
| `aztec-lint update` | Self-update to the latest GitHub release artifact. | `aztec-lint update` |
| `aztec-lint update --version <VERSION>` | Self-update to a specific release (`vX.Y.Z` or `X.Y.Z`). | `aztec-lint update --version v0.1.0` |
| `aztec-lint aztec scan [PATH]` | Run check using the `aztec` profile shortcut (`PATH` defaults to `.`). | `aztec-lint aztec scan` |

Common lint flags (supported by `check`, `fix`, default mode, and `aztec scan`):

- `--profile <PROFILE>`
- `--changed-only`
- `--format text|json|sarif`
- `--severity-threshold warning|error`
- `--min-confidence high|medium|low`
- `--deny <RULE_ID>`
- `--warn <RULE_ID>`
- `--allow <RULE_ID>`
- `--show-suppressed`

## Lints Enforced

Active enforced lints are defined in the canonical catalog and executed by the rule engine.

Detailed reference for each lint (metadata, rationale, limitations, and fixes):

- [`docs/lints-reference.md`](docs/lints-reference.md)

## Configuration

Config files are discovered from target root:

- Primary: `aztec-lint.toml`
- Fallback: `noir-lint.toml`

Minimal example:

```toml
[profile.default]
ruleset = ["noir_core"]

[profile.aztec]
extends = ["default"]
ruleset = ["aztec_pack"]

# Optional profile-level overrides
# deny = ["NOIR001"]
# warn = ["NOIR100"]
# allow = ["AZTEC003"]
```

Unknown rule IDs fail fast before execution (for CLI overrides and profile overrides).

## Output and Exit Codes

Formats:

- `text`
- `json`
- `sarif`

Exit codes:

- `0`: success / no blocking diagnostics
- `1`: blocking diagnostics found
- `2`: internal/config/CLI error

## Suppression

Supported item-level suppression forms:

- `#[allow(RULE_ID)]`
- `#[allow(PACK::RULE_ID)]`

See full behavior and caveats:

- [`docs/suppression.md`](docs/suppression.md)

## Development and CI

Run standard quality gates:

```bash
make ci
```

Or run directly:

```bash
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
```

## Additional Docs

- Architecture baseline: [`docs/architecture.md`](docs/architecture.md)
- Rule authoring guidance: [`docs/rule-authoring.md`](docs/rule-authoring.md)
- Plugin API: [`docs/plugin-api-v0.md`](docs/plugin-api-v0.md)
- Key decisions: [`docs/decisions`](docs/decisions)
