# Changelog

All notable functional changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are grouped by released version.

## [Unreleased]

### 2026-02-21

- Added baseline-freeze fixture coverage for semantically impacted rules across `noir_core` and `aztec` test suites.
- Added edge-case fixtures for alias imports, nested scopes, range-guard ordering, branch/public-effect coupling, and hash-with-guard ordering.
- Added suppression fixtures and assertions for semantically impacted `noir_core` rules (`NOIR001/002/010/020/030/100/110/120`) and additional `aztec` advanced rules (`AZTEC002`, `AZTEC022`).
- Added a first-class semantic model in `aztec-lint-core` with typed function, expression, statement, CFG, DFG, call-site, and guard-node structures.
- Changed `ProjectModel` to include a deterministic `semantic` section with normalization helpers and backward-compatible deserialization defaults.
- Added deterministic normalization/serialization tests for semantic structures and documented the semantic model contract in `docs/architecture.md`.
- Added Noir semantic extraction in `aztec-lint-core` to populate `ProjectModel.semantic` from checked compiler HIR/interner output.
- Added `build_project_semantic_bundle(...)` and `ProjectSemanticBundle` while keeping `build_project_model(...)` as the compatibility path.
- Changed call-graph construction to derive from semantic call-site facts and added fixture coverage proving semantic nodes are extracted for `fixtures/noir_core/minimal/src/main.nr`.

## [0.2.0]

### 2026-02-21

- Added `aztec-lint update` command to self-update from GitHub releases (`latest`, `vX.Y.Z`, or `X.Y.Z`).
- Added SHA-256 checksum verification and archive extraction/replacement flow for release binaries, with platform-specific Linux/macOS (`tar.gz`) and Windows (`zip`) support.
- Changed `aztec-lint update` to resolve the target release first and skip downloads when the installed version is already up to date.
- Changed `aztec-lint update` messaging to report explicit upgrade transitions (`from vA.B.C to vX.Y.Z`).

## [0.1.0]

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
