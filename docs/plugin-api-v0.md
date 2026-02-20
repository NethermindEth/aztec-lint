# Plugin API v0

Date: 2026-02-20
Status: Draft (Phase 10 skeleton)

## Scope

This document defines the initial plugin contract for `aztec-lint`.

v0 goals:

- Versioned rule API contracts.
- Feature-gated host integration (`plugin-api`).
- Sandbox policy placeholders for future WASM runtime.

v0 non-goals:

- Shipping a production WASM runtime.
- Untrusted plugin execution in this phase.

## Crates

- SDK crate: `crates/aztec-lint-sdk`
- Host API (feature gated): `crates/aztec-lint-core/src/plugin/api.rs`

## Version Contract

- Current host/SDK rule API version: `0.1`.
- Compatibility rule:
  - plugin major must equal host major
  - plugin minor must be less than or equal to host minor

This allows host-side additive evolution on minor versions while preventing unknown
breaking major changes.

Plugin identifier contract:

- `plugin_id` must be non-empty.
- `plugin_id` must not include leading/trailing whitespace.
- Allowed characters: lowercase ASCII letters, digits, `.`, `_`, `-`.

## SDK Data Contracts

The SDK exposes stable, compiler-agnostic contracts:

- `ApiVersion`
- `PluginDescriptor`
- `PluginRuleMetadata`
- `PluginInput` / `PluginOutput`
- `PluginDiagnostic`, `PluginSpan`, `PluginFix`
- `RulePlugin` trait

These types intentionally do not expose Noir compiler internals.

## Host Integration (Feature Flag)

`aztec-lint-core` exposes `plugin` module only when `plugin-api` is enabled.

Host-side types:

- `PluginRegistry`
- `PluginLoader` trait (loading interface placeholder)
- `PluginLoadSource`
- `PluginApiError`

Current behavior:

- Registers plugin descriptors and validates API compatibility.
- Validates plugin ID format.
- Rejects duplicate plugin IDs.
- Does not execute plugins inside check/fix command flow yet.

## Sandbox Policy Placeholders

`SandboxPolicy` exists as a contract for future runtime enforcement.

Current fields:

- `max_memory_bytes`
- `max_instructions`
- `max_execution_ms`
- filesystem policy enum
- network policy enum
- clock policy enum

Default policy is restrictive (read-only workspace, no network).

## Testing and Compatibility Gates

Implemented validation includes:

- SDK compatibility/unit tests for version contract.
- Host registry tests for compatibility and duplicate IDs.
- Compile-time integration tests with mock plugin implementations in both SDK and host tests.

## Rollback Strategy

If plugin skeleton causes regressions:

1. Disable feature consumers (keep `plugin-api` feature off).
2. Remove plugin wiring from dependent crates.
3. Keep SDK contracts intact to avoid downstream churn.
