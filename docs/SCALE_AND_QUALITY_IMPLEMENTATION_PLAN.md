# Scale and Quality Implementation Plan

Date: 2026-02-23
Scope: `## Track 4: Scale and quality bar (long-term)` from `docs/CLIPPY_GAP_ANALYSIS.md`

## Required outcomes (source of truth)

From `docs/CLIPPY_GAP_ANALYSIS.md:235-239`:
1. Expand rule count aggressively by category and maturity tier.
2. Build Clippy-style UI/regression/fix/corpus test matrix.
3. Add benchmark and performance gates.
4. Add lint-authoring automation (`xtask`) and generated docs portal.

Related hard requirements from the same analysis:
1. Multi-release rule roadmap with category coverage and maturity states (`docs/CLIPPY_GAP_ANALYSIS.md:44-47`).
2. UI-style test contract with true/false-positive guards, fix expectations, and cross-version snapshots (`docs/CLIPPY_GAP_ANALYSIS.md:183-188`).
3. Authoring workflow parity with generated metadata/docs/tests (`docs/CLIPPY_GAP_ANALYSIS.md:199-201`).
4. Integrate the high-signal Aztec lint candidates captured in `docs/NEW_LINTS.md`.

## Baseline in current repo (measured 2026-02-23)

1. Active lint specs: 16 in canonical catalog (`crates/aztec-lint-core/src/lints/mod.rs`).
2. Runtime registered rules: 16 manual `register(Box::new(...))` calls (`crates/aztec-lint-rules/src/engine/registry.rs:27-45`).
3. Rule fixtures: 54 `fixtures/*/rule_cases/*.nr` cases.
4. Existing tests are mostly rule-pair and CLI golden tests (`crates/aztec-lint-rules/tests/*.rs`, `crates/aztec-lint-cli/tests/cli_golden.rs`), not a matrix harness.
5. No benchmark corpus or dedicated perf CI workflow (`.github/workflows`).
6. No `xtask` crate in workspace (`Cargo.toml:1-9` members list).
7. Docs generation is markdown-only and pack-hardcoded:
   - fixed pack iteration in generator: `for pack in ["aztec_pack", "noir_core"]` (`crates/aztec-lint-core/src/lints/mod.rs:355`)
   - single generated file `docs/lints-reference.md` with consistency test (`crates/aztec-lint-core/src/lints/mod.rs:559-566`)

## Scale-and-quality exit criteria (definition of done)

1. Rule scale:
   - At least 48 active lints with category and maturity metadata.
   - Published roadmap to 100+ lints.
   - No category has fewer than 6 active lints.
2. Test matrix:
   - Per-lint UI + regression fixtures required in CI.
   - Fix snapshot coverage required for all machine-applicable fixes.
   - Corpus suite executes at least 5 real-world project fixtures.
3. Performance:
   - Baseline benchmark corpus committed.
   - CI gate blocks regressions above configured budget per scenario.
4. Automation and docs:
   - `cargo xtask new-lint` scaffolds rule + metadata + fixtures + tests.
   - `cargo xtask update-lints` regenerates derived artifacts and fails on drift.
   - Generated docs portal includes lint index, per-lint pages, and maturity/category filters.
5. `docs/NEW_LINTS.md` suggestions are triaged into:
   - mapped to existing lint IDs where duplicate,
   - accepted into roadmap with concrete rule IDs where new,
   - or explicitly rejected/deferred with rationale.

## Integrated lint intake from `docs/NEW_LINTS.md`

1. Already covered by existing rules (retain and tune):
   - `AZTEC010` covers enqueue + `#[only_self]` boundary contract.
   - `AZTEC001` partially covers private-to-public taint; extend with enqueue-argument sink precision.
   - `AZTEC003` covers debug logging in private entrypoints.
   - `AZTEC020` covers unconstrained influence to critical sinks.
2. Net-new high-priority rules to schedule first:
   - `AZTEC030_NOTE_CONSUMED_WITHOUT_NULLIFIER`
   - `AZTEC031_DOMAIN_SEP_NULLIFIER`
   - `AZTEC032_DOMAIN_SEP_COMMITMENT`
   - `AZTEC033_PUBLIC_MUTATES_PRIVATE_WITHOUT_ONLY_SELF`
   - `AZTEC034_HASH_INPUT_NOT_RANGE_CONSTRAINED`
   - `AZTEC035_STORAGE_KEY_SUSPICIOUS`
3. Net-new second-wave correctness/privacy rules:
   - `AZTEC036_SECRET_BRANCH_AFFECTS_ENQUEUE`
   - `AZTEC037_SECRET_BRANCH_AFFECTS_DELIVERY_COUNT`
   - `AZTEC038_CHANGE_NOTE_MISSING_FRESH_RANDOMNESS`
   - `AZTEC039_PARTIAL_SPEND_NOT_BALANCED`
   - `AZTEC040_INITIALIZER_MISUSE`
   - `AZTEC041_CAST_TRUNCATION_RISK`
4. Net-new opt-in cost/performance rules:
   - `AZTEC050_HASH_IN_LOOP`
   - `AZTEC051_MERKLE_VERIFY_IN_LOOP`
5. Profile integration from suggestions:
   - add `aztec_strict` for metadata-leak/control-flow-sensitive lints,
   - keep high-noise rules off `default` profile,
   - enforce confidence-based defaults (`deny` only for high-confidence protocol/soundness cases).

## Implementation sequence

## Step 0: Lock contracts and target metrics before code churn **COMPLETED**

Files:
1. `docs/CLIPPY_GAP_ANALYSIS.md`
2. New: `docs/decisions/0006-scale-quality-contract.md`
3. `docs/rule-authoring.md`
4. `README.md`
5. `docs/NEW_LINTS.md`

Changes:
1. Add an ADR that defines:
   - maturity tier model (for example `stable`, `preview`, `experimental`)
   - quantitative targets (48 short-term, 100+ long-term)
   - minimum test matrix obligations per tier
   - perf budget policy and allowed variance
2. Record lint-intake policy for external suggestions (including `docs/NEW_LINTS.md`) with statuses: `covered`, `accepted`, `deferred`, `rejected`.
3. Update `docs/rule-authoring.md` so manual registry editing is no longer the default path once `xtask` lands.
4. Add a scale-and-quality operator section in `README.md` for new required commands.

Validation:
1. `rg -n "maturity|test matrix|perf budget|xtask|docs portal|lint intake|covered|accepted|deferred|rejected" docs/decisions/0006-scale-quality-contract.md docs/rule-authoring.md README.md docs/NEW_LINTS.md`

Failure modes to watch:
1. Starting rule expansion before maturity/test/perf contracts are explicit causes inconsistent quality bars between new rules.
2. Suggestion intake decisions are undocumented, creating repeated churn and duplicate lint proposals.

## Step 1: Extend canonical lint metadata for maturity-tier planning

Files:
1. `crates/aztec-lint-core/src/lints/types.rs`
2. `crates/aztec-lint-core/src/lints/mod.rs`
3. `crates/aztec-lint-cli/src/commands/rules.rs`
4. `crates/aztec-lint-cli/src/commands/explain.rs`
5. `crates/aztec-lint-core/src/config/types.rs`
6. `crates/aztec-lint-rules/src/engine/mod.rs`

Changes:
1. Add `LintMaturityTier` to `LintSpec` and enforce metadata invariants in catalog validation.
2. Add tier-aware selectors in config resolution so rules can be enabled by pack and tier without hand-listing IDs.
3. Extend `rules` and `explain` output contracts to include maturity tier and category.
4. Replace hardcoded docs pack iteration path (`for pack in ["aztec_pack", "noir_core"]`) with metadata-driven grouping.
5. Resolve policy naming for suggested “cost” lints:
   - either adopt `cost` as first-class policy, or
   - standardize on existing `performance` and document aliasing in generated docs.
6. Add builtin `aztec_strict` profile for strict/low-noise-sensitive Aztec rules.

Validation:
1. `cargo test -p aztec-lint-core lints::tests::lint_catalog_invariants_hold --locked`
2. `cargo test -p aztec-lint-cli --test cli_golden rules_command_matches_golden_output --locked`
3. `cargo test -p aztec-lint-cli --test cli_golden explain_command_matches_golden_output --locked`
4. `cargo test -p aztec-lint-rules engine::mod::tests --locked`
5. `cargo test -p aztec-lint-core config::types::tests --locked`

Failure modes to watch:
1. Adding maturity without config/CLI support creates metadata that users cannot act on.
2. Output schema drift breaks existing automation consuming rules/explain output.
3. Policy taxonomy drift (`cost` vs `performance`) makes filters and docs inconsistent.

## Step 2: Add rule expansion roadmap and backlog structure

Files:
1. New: `docs/rule-roadmap.md`
2. `crates/aztec-lint-core/src/lints/mod.rs`
3. `crates/aztec-lint-rules/src/engine/registry.rs`
4. `CHANGELOG.md`
5. `docs/NEW_LINTS.md`

Changes:
1. Create a category x maturity matrix in `docs/rule-roadmap.md` with explicit rule IDs, owner, status, target release.
2. Add a “suggestion intake mapping” section to `docs/rule-roadmap.md` that maps each proposal from `docs/NEW_LINTS.md` to:
   - existing lint ID (if duplicate),
   - new accepted lint ID (if new),
   - or deferred/rejected with rationale.
3. Schedule first-wave implementation as the 6 ROI lints from `docs/NEW_LINTS.md` (the `AZTEC030` to `AZTEC035` set).
4. Add placeholder lint metadata entries only when they have implementation milestones and tests planned.
5. Gate release notes on net rule growth by category, not only raw count.

Validation:
1. `rg -n "Category|Maturity|Owner|Target Release|Status" docs/rule-roadmap.md`
2. `cargo test -p aztec-lint-rules full_registry_matches_canonical_lint_catalog --locked`
3. `rg -n "AZTEC030|AZTEC031|AZTEC032|AZTEC033|AZTEC034|AZTEC035|covered|accepted|deferred|rejected" docs/rule-roadmap.md docs/NEW_LINTS.md`

Failure modes to watch:
1. Inflating catalog IDs without runtime registration reintroduces drift and user confusion.
2. Rule count growth concentrated in one category fails scale-and-quality coverage intent.
3. New lint ideas are implemented ad hoc without de-dup mapping, creating overlapping noisy diagnostics.

## Step 3: Build Clippy-style UI/regression/fix/corpus matrix harness

Files:
1. New: `crates/aztec-lint-cli/tests/ui_matrix.rs`
2. New: `crates/aztec-lint-cli/tests/fix_matrix.rs`
3. New: `crates/aztec-lint-cli/tests/corpus_matrix.rs`
4. New directories:
   - `fixtures/ui/`
   - `fixtures/fix/`
   - `fixtures/corpus/`
5. Existing:
   - `crates/aztec-lint-rules/tests/noir_core_rules.rs`
   - `crates/aztec-lint-rules/tests/aztec_foundation_rules.rs`
   - `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`
   - existing core regression suite in `crates/aztec-lint-core/tests/`

Changes:
1. Introduce matrix runners with deterministic fixture discovery and strict naming contracts:
   - UI diagnostics: `*.nr` + expected text/json/sarif snapshots
   - fix tests: `before.nr` / `after.nr` / optional `report.json`
   - corpus tests: project-level expected summary counts and selected golden diagnostics
2. Keep current rule-pair tests as fast unit checks, but make matrix suites authoritative regression gates.
3. Add cross-version snapshots keyed by workspace version for stable contract monitoring.
4. For each accepted lint from `docs/NEW_LINTS.md`, require a minimum fixture pack:
   - positive,
   - negative,
   - suppression/override,
   - at least one false-positive guard tied to its stated caveat.

Validation:
1. `cargo test -p aztec-lint-cli --test ui_matrix --locked`
2. `cargo test -p aztec-lint-cli --test fix_matrix --locked`
3. `cargo test -p aztec-lint-cli --test corpus_matrix --locked`
4. `cargo test -p aztec-lint-rules --locked`

Failure modes to watch:
1. Snapshot churn from unstable ordering or nondeterministic paths.
2. Matrix runtime blow-up without fixture sharding and selective execution strategy.
3. Corpus fixtures depending on network or non-reproducible toolchain state.

## Step 4: Add benchmark corpus and enforce performance budgets

Files:
1. New: `benchmarks/scenarios.toml`
2. New: `benchmarks/budgets.toml`
3. New: `crates/aztec-lint-cli/tests/perf_gate.rs` or `crates/xtask/src/perf_gate.rs` (chosen in Step 5)
4. New corpus inputs under `fixtures/bench/`
5. Existing reference test to replace:
   - `crates/aztec-lint-aztec/src/taint/propagate.rs:293-314`

Changes:
1. Replace ad hoc single-test wall-clock assertions with scenario-driven benchmark runner.
2. Record baseline metrics (median and p95 wall time; optional RSS) per scenario.
3. Enforce regression budgets in CI with explicit allowlist for intentional slowdowns.
4. Include new-lint stress scenarios from `docs/NEW_LINTS.md` domains:
   - note-consumption/nullifier patterns,
   - domain-separation hash tuples,
   - looped hash/Merkle verification cases.

Validation:
1. `cargo test -p aztec-lint-aztec performance_smoke_stays_bounded --locked` (temporary until new gate lands)
2. `cargo run -p xtask -- perf-gate --check --locked` (post-Step 5)
3. CI job must fail on budget violations.

Failure modes to watch:
1. Flaky timing gates from noisy environments without warm-up and repeated runs.
2. Budgets too loose to catch regressions or too strict to permit valid improvements.

## Step 5: Add lint-authoring and maintenance automation via xtask

Files:
1. `Cargo.toml` (add workspace member)
2. New crate:
   - `crates/xtask/Cargo.toml`
   - `crates/xtask/src/main.rs`
   - `crates/xtask/src/new_lint.rs`
   - `crates/xtask/src/update_lints.rs`
   - `crates/xtask/src/docs_portal.rs`
   - `crates/xtask/src/perf_gate.rs`
3. New templates:
   - `crates/xtask/templates/rule.rs.tmpl`
   - `crates/xtask/templates/rule_fixture_positive.nr.tmpl`
   - `crates/xtask/templates/rule_fixture_negative.nr.tmpl`
   - `crates/xtask/templates/rule_fixture_suppressed.nr.tmpl`
4. Existing generated targets:
   - `crates/aztec-lint-core/src/lints/mod.rs`
   - `crates/aztec-lint-rules/src/engine/registry.rs`
   - `docs/lints-reference.md`

Changes:
1. `new-lint` command scaffolds:
   - rule module file
   - metadata entry
   - registry wiring
   - fixture triplet
   - baseline test stub
2. `update-lints` regenerates registry/catalog/docs artifacts from canonical metadata and fails if tree is dirty after generation.
3. Add deterministic ID normalization and uniqueness checks in generator path.
4. Add `xtask lint-intake` command that imports/updates suggestion status from `docs/NEW_LINTS.md` into `docs/rule-roadmap.md`.

Validation:
1. `cargo run -p xtask -- new-lint --id NOIR130 --pack noir_core --category maintainability --tier preview --dry-run`
2. `cargo run -p xtask -- update-lints --check`
3. `cargo run -p xtask -- lint-intake --source docs/NEW_LINTS.md --check`
4. `cargo test --workspace --locked`

Failure modes to watch:
1. Generator writes unstable ordering causing perpetual diff noise.
2. Template drift between generated files and evolving rule trait/contracts.
3. Partial scaffold generation that compiles but misses required fixtures/tests.
4. Intake tooling drifts from roadmap format and silently drops suggested lints.

## Step 6: Generate and publish a docs portal from canonical metadata

Files:
1. New portal content root:
   - `docs/portal/index.md`
   - `docs/portal/lints/`
   - `docs/portal/search-index.json` (generated)
2. `crates/xtask/src/docs_portal.rs`
3. `README.md`
4. `.github/workflows/release.yml` (if publishing with releases)
5. New: `.github/workflows/docs.yml` (if publishing continuously)

Changes:
1. Generate a Clippy-style lint index grouped by category, pack, and maturity tier.
2. Generate one page per lint with lifecycle state, examples, config knobs, and fix safety notes.
3. Publish generated portal artifact in CI and link it from `README.md`.
4. Add roadmap view pages by intake status (`covered`, `accepted`, `deferred`, `rejected`) so `docs/NEW_LINTS.md` decisions remain auditable.

Validation:
1. `cargo run -p xtask -- docs-portal --check`
2. `rg -n "AZTEC001|NOIR001|Maturity|Category" docs/portal/index.md docs/portal/lints`
3. CI docs job verifies no uncommitted generated changes.

Failure modes to watch:
1. Portal content diverging from canonical catalog because generation is optional.
2. Broken links and stale pages when lint IDs are renamed or removed.

## Step 7: Wire scale-and-quality CI gates

Files:
1. `.github/workflows/ci-test.yml`
2. `.github/workflows/ci-quality.yml`
3. New:
   - `.github/workflows/ci-matrix.yml`
   - `.github/workflows/ci-perf.yml`
   - `.github/workflows/ci-docs.yml`
4. `Makefile`

Changes:
1. Add dedicated jobs for:
   - UI/fix/corpus matrix
   - perf budget gate
   - generation drift check (`xtask update-lints --check`, `xtask docs-portal --check`)
2. Add local developer entry points in `Makefile`:
   - `make matrix`
   - `make perf`
   - `make generate`
3. Keep existing diagnostics/fix gates and make them a subset of new matrix checks.

Validation:
1. `make matrix`
2. `make perf`
3. `make generate`
4. GitHub Actions dry run via PR branch with all new jobs passing.

Failure modes to watch:
1. CI duration explosion without job partitioning/caching.
2. Duplicate checks between old and new workflows causing inconsistent pass/fail criteria.

## Rollout milestones (recommended)

1. Milestone A (metadata + automation): Steps 0, 1, 5.
2. Milestone B (matrix harness + first corpus): Step 3.
3. Milestone C (perf gates): Step 4 and CI integration from Step 7.
4. Milestone D (docs portal + rule growth): Step 6 plus roadmap execution from Step 2.

## Immediate execution checklist

1. Land Step 0 ADR and target metrics first.
2. Implement `xtask` skeleton with `update-lints --check` before adding many new rules.
3. Stand up matrix harness with at least one lint from each category.
4. Add perf gate with 3 initial scenarios, then expand corpus.
5. Turn on CI blocking only after one full green run on all new jobs.
