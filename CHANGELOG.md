# Changelog

All notable functional changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are grouped by released version.

## [Unreleased]

### 2026-02-21

- Added a diagnostic invariant validation layer in `aztec-lint-core` (`validate_diagnostic` / `validate_diagnostics`) with deterministic violation types for empty metadata, invalid spans, missing suppression reasons, and overlapping implicit multipart suggestions.
- Changed rule engine execution to validate diagnostics at the engine boundary and return structured `RuleEngineError::InvalidDiagnostics` failures instead of emitting invalid diagnostics.
- Changed CLI check flow to surface diagnostic contract violations as runtime internal errors (exit code `2`) with actionable context including rule and span from the first violation.
- Added validation-focused tests in `aztec-lint-core` and `aztec-lint-rules` and updated engine call sites/tests to the new `Result<Vec<Diagnostic>, RuleEngineError>` contract.
- Fixed diagnostic-validation determinism by sorting diagnostics before validation (stabilizing the reported first violation) and resolved `clippy::items_after_test_module` in `diagnostics::validate` tests.
- Added suggestion model v2 in `aztec-lint-core` with first-class `TextEdit` and `SuggestionGroup` types plus `Diagnostic.suggestion_groups`.
- Changed `span_suggestion(...)` and `multipart_suggestion(...)` helpers to emit grouped suggestions (`1 edit` and `N edits` respectively) while keeping legacy fields available during migration.
- Added compatibility merge helpers so grouped suggestions can be rendered/applied through legacy structured suggestion paths without breaking existing consumers.
- Changed JSON output to serialize deterministic `suggestion_groups` and keep compatibility fields (`structured_suggestions`, `fixes`) populated from grouped data when needed.
- Changed SARIF output to emit grouped fixes (one SARIF fix per suggestion group, with multiple replacements when applicable) and updated SARIF golden fixtures accordingly.
- Fixed SARIF mixed-mode compatibility so diagnostics that contain both `suggestion_groups` and additional legacy `structured_suggestions` preserve non-duplicated legacy structured fixes instead of dropping them.
- Changed text rendering and fix candidate extraction to support diagnostics that provide grouped suggestions without explicit legacy fields.
- Changed fix application in `aztec-lint-core` from single-edit candidates to grouped candidates with transactional all-or-none application semantics per group.
- Added grouped fix metadata (`group_id`, `provenance`, `edit_count`) and new group-level skip reasons (`mixed_file_group`, `group_overlap`, `invalid_group_span`, `group_noop`) to fix reports.
- Changed CLI `fix` text output to include deterministic grouped fix event lines plus skipped-reason breakdown counters for CI/editor consumption.
- Added grouped-fix regression coverage for atomic rollback, same-group overlap rejection, deterministic overlap winner selection, idempotence, and mixed-file group rejection.
- Fixed fix-report visibility for non-machine suggestions so grouped/legacy structured suggestions are now counted and reported as `unsafe_fix` skips instead of being silently excluded from candidate accounting.
- Changed rule emitters `NOIR001`, `NOIR100`, and `AZTEC021` to emit group-backed suggestions directly via `suggestion_groups` as the primary suggestion contract.
- Kept applicability discipline in migrated emitters: deterministic underscore-prefix local renames remain `MachineApplicable`, while exploratory constant/range guidance remains `MaybeIncorrect`.
- Updated rule-level and integration tests to assert suggestion-group structure and applicability for migrated emitters (`crates/aztec-lint-rules/tests/noir_core_rules.rs` and `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`).
- Added scoped lint directive resolution in `aztec-lint-rules` for `#[allow]`, `#[warn]`, and `#[deny]` with file/module/item scopes, nearest-scope precedence, and last-directive-wins behavior at equal scope.
- Changed rule engine severity/suppression resolution to apply scoped directive levels per diagnostic span on top of profile/CLI baseline levels (`allow` suppresses; `warn`/`deny` override emitted severity).
- Fixed scoped-level engine normalization to preserve pre-existing diagnostic suppression flags in non-`allow` paths so invariant validation still reports malformed suppressed diagnostics (for example missing suppression reasons) instead of masking them.
- Added Step 5 regression coverage for scoped directive precedence and CLI behavior, including JSON suppression/severity visibility and error-threshold exit-code handling.
- Changed JSON and SARIF grouped-suggestion ordering to include stable edit-span signatures in sort keys, hardening deterministic output when group metadata collides.
- Changed text output to render explicit grouped suggestion details (group id, applicability, grouped edit count, and per-edit replacement spans) while keeping legacy compatibility lines.
- Added Step 6 output regression coverage for grouped-suggestion ordering/determinism in `output::json`, `output::sarif`, and `output::text`.
- Updated grouped-suggestion text golden snapshot to reflect the expanded grouped-suggestion rendering contract.

## [0.3.0]

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
- Added typed semantic query APIs in `aztec-lint-rules` via `RuleContext::query()` (`functions`, `locals_in_function`, `index_accesses`, `assertions`, `cfg`, `dfg`).
- Changed `RuleContext` to support semantic model overrides through `set_semantic_model(...)` while keeping file-based accessors for fallback rules.
- Added engine context tests for query availability, override behavior, and deterministic query ordering.
- Changed CLI `check` pipeline to build `ProjectSemanticBundle` and inject semantic data into `RuleContext` before rule execution, while preserving diagnostic path rebasing behavior.
- Changed `NOIR001` to compute unused locals/imports from semantic statements and DFG/identifier facts, with text heuristics retained only as a fallback path.
- Fixed `NOIR001` import-usage detection to treat attribute macro references as usage (for example `#[aztec]` and aliased forms like `#[az]`), preventing false positives on macro imports.
- Fixed `NOIR001` grouped-import binding tracking so nested/grouped `use` clauses are evaluated per imported binding when determining unused imports.
- Fixed `NOIR001` to ignore `pub use` re-exports in file-local unused-import analysis.
- Fixed `NOIR001` to treat type-position identifiers (for example struct fields, function signatures, and type aliases) as import usage.
- Fixed `NOIR001` trait-import usage detection to treat trait-method calls and trait contexts (for example `x.to_field()`, `impl Trait for ...`, and `<T as Trait>::...`) as valid import usage, including aliased trait imports.
- Fixed `NOIR001` to treat value-path usages in function bodies as import usage (for example struct literals like `Type { ... }`, associated calls like `Type::from_field(...)`, and qualified value paths like `TypeOrModule::CONST`), including test-function bodies.
- Fixed `NOIR001` trait-associated call handling to treat imports such as `FromField` as used when associated calls resolve through imported traits (for example `AztecAddress::from_field(...)`), with conservative fallback to avoid correctness-breaking removals when semantic detail is partial.
- Added `NOIR001` regression coverage for used/unused trait imports and alias trait imports in method-call and trait-context paths.
- Added `NOIR001` regression coverage for struct-literal and associated-call usage patterns (`PositionReceiptNote`, `AztecAddress::from_field`, `FromField`) and test-body usage paths.
- Added Clippy-style target selection flags to lint commands (`--all-targets`, `--lib`, `--bins`, `--examples`, `--benches`, `--tests`), with default behavior equivalent to `--all-targets`.
- Fixed target filtering to apply at diagnostic-file level so test-path files are excluded when `--tests` is not selected (including `tests/` and `test/` path components such as `src/test/...`).
- Changed `NOIR002` to detect shadowing from semantic lexical scopes (function + block spans) and semantic `let` declarations, with legacy brace-depth parsing retained as fallback only.
- Added semantic-path unit coverage for `NOIR001` and `NOIR002`, plus shared statement-level `let` binding extraction utilities.
- Changed `NOIR010` to derive bool bindings from semantic types/DFG and validate assertion consumption from semantic guard/use-def links, with text heuristics retained only as fallback.
- Fixed `NOIR010` assertion sink detection to recognize identifier assertions and boolean transforms in assertion contexts (for example `assert(flag)`, `assert(!flag)`, alias flows, and `assert(flag && other)` patterns) and assertion-like wrapper calls.
- Fixed `NOIR010` reporting to emit only when a boolean has no assertion sink and no meaningful downstream use, preventing false positives in test/unconstrained function flows.
- Added `NOIR010` regression coverage for negated assertions, alias assertions, conjunction assertions, assertion-like wrappers, and meaningful non-assert uses.
- Changed `NOIR020` to detect index accesses and guard coverage from semantic expression/guard facts, with text heuristics retained only as fallback.
- Changed `NOIR020` to use local bounds proofs from loop ranges, affine index forms, and array-length facts, suppressing false positives for safe copy/packing loops and reporting a distinct message when an index is provably out of bounds versus not provable.
- Fixed `NOIR020` function-body bounds analysis for narrow semantic function spans so loop-header proofs (for example `for i in 0..32`) are applied consistently to both source and destination index expressions in byte-copy assignments.
- Changed `NOIR030` to propagate unconstrained call influence through semantic DFG into assert/constrain sinks, with text heuristics retained only as fallback.
- Added shared semantic parsing helpers in `noir_core::util` (`source_slice`, `extract_index_identifier`) and semantic-path unit coverage for `NOIR010`, `NOIR020`, and `NOIR030`.
- Changed `NOIR100` to detect magic numbers from semantic literal nodes while excluding constant declaration contexts, with text heuristics retained only as fallback.
- Fixed `NOIR100` to ignore literals in named constant definitions, including `const`, `global`, `pub const`, `pub global`, and uppercase domain-constant assignments (SCREAMING_SNAKE_CASE).
- Fixed `NOIR100` to suppress byte-packing/encoding literals (for example `[u8; 32]`, `0..32`, and index offset math such as `32 + i` in packing loops).
- Changed `NOIR100` default behavior to skip test-path files (`**/test/**`, `*_test.nr`, `*_tests.nr`), with opt-in override via `AZTEC_LINT_NOIR100_INCLUDE_TEST_PATHS=1`.
- Fixed `NOIR100` to suppress literals used in fixture-style assertion and constructor contexts in tests and mock data setup.
- Fixed `NOIR100` to suppress known hash-domain tag literals in `poseidon2_hash([K, ...])` patterns.
- Changed `NOIR100` confidence to high and tightened detection so only high-confidence magic-number findings are reported by default.
- Added regression coverage for `NOIR100` named constants, global constants, and byte-packing suppression paths.
- Changed `NOIR110` to compute complexity from semantic CFG decision blocks, with text heuristics retained only as fallback.
- Changed `NOIR120` to compute nesting from semantic block-span containment, with brace-depth parsing retained only as fallback.
- Added semantic-path unit coverage for `NOIR100`, `NOIR110`, and `NOIR120`.
- Added semantic-aware Aztec model construction via `build_aztec_model_with_semantic(...)`, while keeping `build_aztec_model(...)` as the compatibility path.
- Changed Aztec semantic site extraction (`note_read_sites`, `note_write_sites`, `nullifier_emit_sites`, `public_sinks`, `enqueue_sites`) to derive primarily from semantic call-site and statement facts, with source text heuristics retained only as fallback.
- Changed CLI `check` Aztec model wiring to pass the project semantic model into Aztec model construction.
- Added `AztecModel::normalize()` in core to centralize deterministic ordering/dedup behavior.
- Added semantic-aware Aztec taint graph construction via `build_def_use_graph_with_semantic(...)`, while keeping `build_def_use_graph(...)` as the compatibility path.
- Changed Aztec taint source/sink detection to derive from semantic DFG/CFG/call-site facts (typed node IDs) for note reads, private params, unconstrained returns, public outputs, storage writes, nullifier/commitment, hash/serialize, branch condition, and merkle-witness sinks.
- Changed hash/serialize guard sanitization in taint propagation to use CFG dominance over semantic guard coverage instead of line-offset ordering.
- Added `SemanticModel::statement_block_map(...)` and `SemanticModel::cfg_dominators(...)` in core, plus semantic-path taint tests and a CFG-dominance guard test.
- Changed Aztec rules `AZTEC001`, `AZTEC002`, `AZTEC003`, `AZTEC020`, `AZTEC021`, and `AZTEC022` to build taint analysis from semantic-aware def-use graphs via `build_def_use_graph_with_semantic(...)`.
- Changed `AZTEC022` witness-verification detection to use semantic call-site and DFG/assertion facts, with text matching retained only as fallback when semantic data is unavailable.
- Changed `noir_core::util` text parser helpers to explicit fallback APIs (`text_fallback_*`) and updated `noir_core` rule callsites to use those fallback names.
- Changed Aztec pattern helper names to explicit fallback APIs (`fallback_*`) and updated Aztec model/taint builders to call fallback helpers only on non-semantic paths.
- Added semantic-first authoring guidance and fallback gating requirements to `docs/rule-authoring.md` and `docs/architecture.md`.

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
