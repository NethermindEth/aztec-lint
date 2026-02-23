# Changelog

All notable functional changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are grouped by released version.

## [Unreleased]

### Rule Growth by Category

- `correctness`: `+3`
- `maintainability`: `+0`
- `privacy`: `+2`
- `protocol`: `+1`
- `soundness`: `+0`

- Added six second-wave preview Aztec lints to the active canonical catalog and runtime registry:
  - `AZTEC036` (`privacy`): secret-dependent branch affects enqueue behavior.
  - `AZTEC037` (`privacy`): secret-dependent branch affects delivery count.
  - `AZTEC038` (`correctness`): change note appears to miss fresh randomness.
  - `AZTEC039` (`correctness`): partial spend logic appears unbalanced.
  - `AZTEC040` (`protocol`): initializer entrypoint missing `#[only_self]`.
  - `AZTEC041` (`correctness`): Field/integer cast may truncate or wrap unexpectedly.
- Added second-wave rule module and registry scaffolding in `aztec-lint-rules` for `AZTEC036` through `AZTEC041`.
- Implemented `AZTEC036` rule logic (`secret branch affects enqueue behavior`) using taint-flow filtering for secret branch conditions plus enqueue taint in the same function.
- Added `AZTEC036` rule-case fixture matrix in `fixtures/aztec/rule_cases/` (`positive`, `negative`, `suppressed`, `false_positive_guard`) and integrated `aztec036_fixture_matrix` in `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`.
- Replaced scaffolded accepted UI fixtures for `AZTEC036` in `fixtures/ui/accepted/AZTEC036/` with concrete matrix scenarios.
- Extended Aztec taint sink coverage with dedicated delivery-call detection (`TaintSinkKind::DeliveryCall`) for `deliver(...)` style flows in semantic and fallback sink paths.
- Implemented `AZTEC037` rule logic (`secret branch affects delivery count`) using taint-flow filtering for secret branch conditions plus delivery-call taint in the same function.
- Added `AZTEC037` rule-case fixture matrix in `fixtures/aztec/rule_cases/` (`positive`, `negative`, `suppressed`, `false_positive_guard`) and integrated `aztec037_fixture_matrix` in `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`.
- Replaced scaffolded accepted UI fixtures for `AZTEC037` in `fixtures/ui/accepted/AZTEC037/` with concrete matrix scenarios.
- Implemented `AZTEC038` rule logic (`change note missing fresh randomness`) using conservative change-note randomness heuristics (reuse/deterministic derivation without freshness context).
- Added `AZTEC038` rule-case fixture matrix in `fixtures/aztec/rule_cases/` (`positive`, `negative`, `suppressed`, `false_positive_guard`) and integrated `aztec038_fixture_matrix` in `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`.
- Replaced scaffolded accepted UI fixtures for `AZTEC038` in `fixtures/ui/accepted/AZTEC038/` with concrete matrix scenarios.
- Implemented `AZTEC039` rule logic (`partial spend not balanced`) using conservative partial-spend arithmetic heuristics for unguarded subtraction and missing reconciliation assertions.
- Added `AZTEC039` rule-case fixture matrix in `fixtures/aztec/rule_cases/` (`positive`, `negative`, `suppressed`, `false_positive_guard`) and integrated `aztec039_fixture_matrix` in `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`.
- Replaced scaffolded accepted UI fixtures for `AZTEC039` in `fixtures/ui/accepted/AZTEC039/` with concrete matrix scenarios.
- Implemented `AZTEC040` rule logic (`initializer entrypoint missing #[only_self]`) using semantic Aztec entrypoint kinds (`Initializer` and `OnlySelf`).
- Added `AZTEC040` rule-case fixture matrix in `fixtures/aztec/rule_cases/` (`positive`, `negative`, `suppressed`, `false_positive_guard`) and integrated `aztec040_fixture_matrix` in `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`.
- Replaced scaffolded accepted UI fixtures for `AZTEC040` in `fixtures/ui/accepted/AZTEC040/` with concrete matrix scenarios.
- Implemented `AZTEC041` rule logic (`cast truncation risk`) with Field/integer cast checks and range-guard reuse from `AZTEC034`.
- Added `AZTEC041` rule-case fixture matrix in `fixtures/aztec/rule_cases/` (`positive`, `negative`, `suppressed`, `false_positive_guard`) and integrated `aztec041_fixture_matrix` in `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`.
- Replaced scaffolded accepted UI fixtures for `AZTEC041` in `fixtures/ui/accepted/AZTEC041/` with concrete matrix scenarios.
- Updated generated lint reference and CLI/snapshot baselines to reflect the expanded aztec profile rule set (`active_rules=28`).
- Added `docs/SECOND_WAVE_IMPLEMENTATION_PLAN.md` with sequential implementation and validation gates for the second-wave backlog.

## [0.5.0]

### 2026-02-23

### Rule Growth by Category

- `correctness`: `+1`
- `maintainability`: `+0`
- `privacy`: `+0`
- `protocol`: `+3`
- `soundness`: `+2`

- Added six new preview Aztec lints to the active catalog and runtime registry:
  - `AZTEC030` (`soundness`): note consumed without nullifier emission.
  - `AZTEC031` (`protocol`): nullifier hash missing configured domain-separation components.
  - `AZTEC032` (`protocol`): commitment hash missing configured domain-separation components.
  - `AZTEC033` (`protocol`): public private-state mutation without `#[only_self]`.
  - `AZTEC034` (`soundness`): hash input cast to `Field` without prior range guard.
  - `AZTEC035` (`correctness`): suspicious repeated nested storage key (`.at(x).at(x)`).
- Added shared Aztec text-scan utilities for contract/function line scanning and call-argument extraction in `crates/aztec-lint-rules/src/aztec/text_scan.rs`.
- Added full fixture matrix coverage for `AZTEC030` through `AZTEC035` in `fixtures/aztec/rule_cases/` and corresponding accepted UI fixture packs in `fixtures/ui/accepted/AZTEC030` through `fixtures/ui/accepted/AZTEC035`.
- Added advanced rule test coverage for `AZTEC030` through `AZTEC035` in `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`.
- Fixed `AZTEC031` and `AZTEC032` domain-separation detection to validate required components against hash preimages (for example `hash(...)`) instead of the full sink call argument list, preventing false negatives such as `emit_nullifier(hash(x), nonce)`.
- Added targeted regressions for `AZTEC031`/`AZTEC032` to assert diagnostics when required domain components appear outside the hashed payload.
- Changed CLI golden contracts to include the six new lint rows and updated aztec-profile active-rule counts (`active_rules=22`) in text snapshots.
- Regenerated lint reference and portal artifacts to include `AZTEC030` through `AZTEC035`:
  - `docs/lints-reference.md`
  - `docs/portal/index.md`
  - `docs/portal/search-index.json`
  - `docs/portal/lints/aztec030.md` through `docs/portal/lints/aztec035.md`

- Added lint maturity metadata (`stable`, `preview`, `experimental`) to the canonical `LintSpec` model and catalog.
- Changed catalog quality invariants to reject non-canonical `cost` policy naming (use `performance`) and to reject active `stable` lints with `low` confidence.
- Added tier-aware ruleset selectors in config resolution: `tier:<tier>`, `maturity:<tier>`, and `<pack>@<tier>`.
- Added builtin `aztec_strict` profile, extending `aztec` and enabling stricter tier-targeted Aztec rulesets.
- Added a category x maturity rule roadmap matrix in `docs/rule-roadmap.md` with explicit rule IDs, owners, statuses, target releases, and first-wave (`AZTEC030`-`AZTEC035`) scheduling details.
- Changed release-note roadmap policy to track net rule growth by category (not only raw total count) and to avoid placeholder catalog/registry IDs before implementation milestones and test plans exist.
- Changed `docs/NEW_LINTS.md` intake guidance to defer accepted-lint execution scheduling and matrix obligations to `docs/rule-roadmap.md`.
- Changed `aztec-lint rules` output contract to include `CATEGORY` and `MATURITY` columns.
- Changed `aztec-lint explain <RULE_ID>` output to include lint maturity.
- Changed lint reference generation to use metadata-driven pack grouping instead of hardcoded pack iteration.
- Regenerated `docs/lints-reference.md` to include maturity per lint and a policy alias note (`cost` roadmap shorthand maps to canonical `performance`).
- Added `crates/xtask` as a workspace automation crate with command routing for `new-lint`, `update-lints`, `lint-intake`, `docs-portal`, and `perf-gate`.
- Added `cargo xtask new-lint` scaffolding for rule source stubs, fixture triplets, test stubs, and canonical metadata/registry snippet files, with deterministic rule ID normalization and duplicate-ID rejection.
- Added `cargo xtask update-lints` to validate canonical lint IDs, regenerate `docs/lints-reference.md`, run registry/catalog integrity checks, and fail on generated artifact drift.
- Added `cargo xtask lint-intake` to import proposal intake statuses from `docs/NEW_LINTS.md` into `docs/rule-roadmap.md` with generated section markers and `--check` drift enforcement.
- Added `cargo xtask docs-portal` to generate a lint docs portal at `docs/portal/` (`index.md`, per-lint pages, and `search-index.json`) with deterministic `--check` verification.
- Added `cargo xtask perf-gate` with `--check` support to run the performance smoke gate and validate benchmark budget alignment when benchmark scenario/budget files are present.
- Added lint scaffolding templates under `crates/xtask/templates/` for rule source and positive/negative/suppressed fixture generation.
- Updated operator docs to reflect the implemented `new-lint` command contract (`--category`, `--tier`, optional `--policy`) and generated `docs/rule-roadmap.md` intake mapping.
- Added CLI matrix harness suites in `crates/aztec-lint-cli/tests/` (`ui_matrix.rs`, `fix_matrix.rs`, `corpus_matrix.rs`) with deterministic fixture discovery and strict file-contract validation.
- Added shared matrix test utilities in `crates/aztec-lint-cli/tests/support/mod.rs` for stable CLI invocation, temp project setup, sorted fixture traversal, and path-normalized output comparison.
- Added UI matrix fixtures under `fixtures/ui/`, including version-keyed text/json/SARIF snapshots for `noir100_magic_array_len` and accepted-lint fixture-pack scaffolds (`positive`, `negative`, `suppressed`, `false_positive_guard`) for `AZTEC030` through `AZTEC041`.
- Added fix matrix fixtures under `fixtures/fix/cases/noir001_prefix_unused/` with `before.nr`/`after.nr` source contracts and version-keyed expected fix-report metrics.
- Added corpus matrix fixtures under `fixtures/corpus/projects/` with version-keyed expected summary/golden diagnostic contracts for a clean project and a warning-producing project.
- Added benchmark scenario and budget configuration files (`benchmarks/scenarios.toml`, `benchmarks/budgets.toml`) with warmup/sample runner settings and per-scenario median/p95 regression thresholds.
- Added benchmark corpus fixtures under `fixtures/bench/` for note-consumption/nullifier, domain-separation hash tuple, and looped hash/Merkle verification stress domains.
- Changed `cargo xtask perf-gate` from a file-alignment check into a scenario runner that executes benchmark fixtures, computes median/p95 timings, validates minimum taint-flow expectations, enforces budget thresholds, and supports explicit slowdown allowlisting via `[allowlist].scenario_ids`.
- Changed `performance_smoke_stays_bounded` in `crates/aztec-lint-aztec/src/taint/propagate.rs` to use fixture-driven benchmark smoke scenarios instead of an ad hoc generated-chain timing assertion.
- Added dedicated CI workflows for scale-and-quality gates:
  - `.github/workflows/ci-matrix.yml` for diagnostics/fix regression + UI/fix/corpus matrix suites
  - `.github/workflows/ci-perf.yml` for `cargo xtask perf-gate --check --locked`
  - `.github/workflows/ci-docs.yml` for generated artifact drift checks (`xtask update-lints --check`, `xtask docs-portal --check`)
- Changed `.github/workflows/ci-test.yml` to keep workspace test execution focused, while matrix/perf/docs enforcement runs in dedicated workflows.
- Changed `.github/workflows/ci-quality.yml` to include an explicit `cargo check --workspace --all-targets --locked` gate.
- Added local CI parity entrypoints in `Makefile`: `make matrix`, `make perf`, and `make generate`, and updated `make ci` to run the full gate set.
- Added generated docs portal baseline artifacts under `docs/portal/` so docs drift checks can run as blocking CI gates.
- Changed `cargo xtask docs-portal` to generate intake roadmap pages at `docs/portal/roadmap/` (`covered`, `accepted`, `deferred`, `rejected`, plus index) sourced from `docs/NEW_LINTS.md`.
- Changed first-wave intake/roadmap status for implemented lints (`AZTEC030` through `AZTEC035`) from `accepted/planned` to `covered/active` in `docs/NEW_LINTS.md`, `docs/rule-roadmap.md`, and generated portal roadmap pages.
- Changed per-lint portal pages to include explicit `Config Knobs` and `Fix Safety Notes` sections.
- Added CI docs artifact publication (`docs/portal`) in `.github/workflows/ci-docs.yml` and linked the portal entrypoint in `README.md`.

## [0.4.0]

### 2026-02-22

- Changed `NOIR100` magic-number detection to report only high-signal contexts (branching, assertions/constraints, range boundaries, hash/serialization/protocol-sensitive uses) and stop reporting one-off plain local initializer literals.
- Added `NOIR101` (`warn`, `low` confidence) to report repeated plain local-initializer literals within the same function/module scope, reducing noise while still surfacing copy-pasted unnamed constants.
- Fixed noisy local-init behavior so single literals such as `let _unused = 9;` no longer trigger magic-number warnings.
- Added regression coverage and fixture updates for the new `NOIR100`/`NOIR101` split, including explicit non-reporting of single local initializers and reporting of repeated initializer literals.
- Added catalog-driven lint reference generation in `aztec-lint-core` and a synchronization test that fails when `docs/lints-reference.md` drifts from `crates/aztec-lint-core/src/lints/mod.rs`.
- Updated lint catalog/docs confidence alignment (`NOIR100` high, `NOIR101` low) and refreshed CLI/text/SARIF golden outputs for deterministic snapshots.
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
- Added dedicated regression gate suites for core invariants/grouped-fix/output determinism and rules scoped lint-level precedence/suppression visibility.
- Added CLI determinism gates for SARIF and text output in `crates/aztec-lint-cli/tests/cli_golden.rs` (`check_sarif_output_is_deterministic`, `check_text_output_is_deterministic`) alongside existing JSON determinism coverage.
- Changed CI test workflow to run explicit lockfile-enforced (`--locked`) regression gate commands in `.github/workflows/ci-test.yml`.
- Updated Step 7 rollout documentation to keep legacy-field migration guidance without introducing a fixed compatibility-window removal schedule.

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
