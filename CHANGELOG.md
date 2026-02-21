# Changelog

All notable functional changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Until semantic versions are tagged, entries are grouped by calendar date.

## [Unreleased]

### 2026-02-21

- Added canonical lint metadata catalog as the single source of truth for lint IDs and metadata.
- Added lint metadata fields for introduced versions and richer lifecycle details.
- Changed `catalog` and `explain` to resolve lint data from canonical metadata.
- Changed runtime rule registry to bind to canonical metadata and validate catalog/registry integrity.
- Fixed configuration to fail fast on unknown CLI and profile rule overrides.

### 2026-02-20

- Added core configuration system and initial CLI command surface.
- Added Noir frontend project model integration with the `noir_core` rule engine and baseline rules.
- Added Aztec model/rules integration plus taint engine v1 and taint-based rules.
- Added output adapters for text, JSON, and SARIF.
- Added safe fix executor and fix workflow with changed-only support.
- Added versioned plugin contract crate and feature-gated plugin host API skeleton.
- Added built-in `noir` profile alias and `aztec-lint` CLI alias/default check-fix mode.
- Changed `check` command wiring to engine filtering and improved Nargo workspace/dependency discovery.
- Changed diagnostic rendering to clippy-style output with source snippets and ANSI colors.
- Changed diagnostics/check flow to use structured suggestions and applicability.
- Fixed unused import and shadowing analysis behavior.
- Fixed compiler warning handling to surface local warnings while suppressing dependency-source warnings.
- Fixed fix pipeline to apply only machine-applicable suggestions.
- Fixed deterministic behavior across registry metadata, JSON, and SARIF output.
- Fixed clippy-style text renderer regression.

### 2026-02-19

- Added core lint model, diagnostics contracts, and policy contracts.
