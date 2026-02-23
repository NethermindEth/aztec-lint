# aztec-lint vs rust-clippy: Gap Analysis

Date: 2026-02-23  
Compared repos:
- `aztec-lint`: `/home/ametel/source/aztec-lint`
- `rust-clippy`: `/home/ametel/source/rust-clippy`

## Executive summary

`aztec-lint` has materially improved since the previous analysis. Several foundational parity gaps are now closed: canonical lint metadata is enforced at runtime, unknown-rule overrides fail fast with replacement hints, explain output is rich, suppression supports file/module/item scope, diagnostic invariants are validated, and authoring/update `xtask` flows exist.

Remaining distance to Clippy is now concentrated in four areas:

1. **Scale** (22 active rules vs 806 declared Clippy lints).
2. **Semantic depth** (many checks still depend on source-text heuristics/fallbacks).
3. **Config + group sophistication** (no Clippy-style lint groups/per-lint config surface).
4. **Toolchain/test ecosystem parity** (`cargo clippy`/`clippy-driver` integration model and compile-test scale are still far ahead).

---

## Snapshot metrics (current)

- Clippy declared lints (`_INFO` entries): **806** (`clippy_lints/src/declared_lints.rs`)
- aztec-lint active catalog lints: **22** (`crates/aztec-lint-core/src/lints/mod.rs`)
- aztec-lint runtime registry rules: **22** (`crates/aztec-lint-rules/src/engine/registry.rs:33`)

- Clippy UI tests (`tests/ui/*.rs`): **1187**
- Clippy config UI tests (`tests/ui-toml/*.rs`): **124**
- Clippy cargo-project tests (`tests/ui-cargo/**/Cargo.toml`): **45**
- aztec-lint Noir fixtures (`fixtures/**/*.nr`): **141**
- aztec-lint test files under `crates/*/tests`: **12**

- Clippy config keys (estimated from `define_Conf!`): **94** (`clippy_config/src/conf.rs:344`)
- aztec-lint ruleset selectors: pack + tier (`pack`, `tier:*`, `pack@tier`) (`crates/aztec-lint-core/src/config/types.rs:472`)

---

## Closed since previous revision (removed from active gap backlog)

### 1) Catalog/config/registry drift safeguards
- Runtime registry now panics if a rule is missing from canonical metadata (`crates/aztec-lint-rules/src/engine/registry.rs:60`).
- Engine validates full runtime/catalog integrity both directions (`crates/aztec-lint-rules/src/engine/mod.rs:199`).

### 2) Unknown rule ID validation and replacement hints
- CLI/profile overrides now validate rule IDs and return `UnknownRuleId` with optional replacement suggestions (`crates/aztec-lint-core/src/config/types.rs:406`, `crates/aztec-lint-core/src/config/mod.rs:93`).

### 3) Rich lint metadata + lifecycle model
- Canonical `LintSpec` includes category, maturity, introduced version, lifecycle, and docs (`crates/aztec-lint-core/src/lints/types.rs:118`).
- Catalog integrity checks enforce lifecycle and docs invariants (`crates/aztec-lint-core/src/lints/mod.rs:596`).

### 4) Explain output depth
- `aztec-lint explain` now prints lifecycle, full rationale sections, examples, and references (`crates/aztec-lint-cli/src/commands/explain.rs:19`).

### 5) Suppression semantics scope parity improvements
- File/module/item scope and deterministic precedence are documented and implemented (`docs/suppression.md:31`, `crates/aztec-lint-rules/src/engine/context.rs`).

### 6) Diagnostics + grouped suggestion validation baseline
- Engine validates diagnostics post-run (`crates/aztec-lint-rules/src/engine/mod.rs:142`).
- Validation covers grouped edits overlap/span constraints (`crates/aztec-lint-core/src/diagnostics/validate.rs:256`).

### 7) Authoring/update automation exists
- `cargo xtask new-lint` scaffolding exists (`crates/xtask/src/new_lint.rs:13`).
- `cargo xtask update-lints` checks docs/registry sync (`crates/xtask/src/update_lints.rs:12`).
- Performance budget gate exists (`crates/xtask/src/perf_gate.rs:54`).

---

## Remaining parity gaps (re-audited)

## 1) Coverage scale gap
### Gap
Clippy-scale utility is still dominated by rule-count and surface-area differences.

### Evidence
- aztec-lint active rule corpus is 22 (`crates/aztec-lint-rules/src/engine/registry.rs:33`).
- Clippy has 806 declared lints (`clippy_lints/src/declared_lints.rs`).
- `AZTEC036`..`AZTEC041` are accepted/planned but not active yet (`docs/rule-roadmap.md`, `docs/NEW_LINTS.md`).

### Needed work
- Land second-wave accepted rules first (`AZTEC036`..`AZTEC041`).
- Expand into deferred performance lints (`AZTEC050`, `AZTEC051`) as opt-in tier.
- Set explicit medium-term corpus target (for example 60-100 active lints).

## 2) Semantic precision gap (typed queries vs heuristic/text dependence)
### Gap
Semantic model extraction exists, but many rules still rely on source slicing and fallback string heuristics.

### Evidence
- Semantic extraction from Noir compiler HIR exists (`crates/aztec-lint-core/src/noir/semantic_builder.rs:35`).
- Rule query API exists but is not currently used by rules (`crates/aztec-lint-rules/src/engine/query.rs:27`; `rg ctx.query` = 0 hits).
- Noir rules retain `run_text_fallback` paths broadly (`crates/aztec-lint-rules/src/noir_core/noir001_unused.rs:239`, `crates/aztec-lint-rules/src/noir_core/noir020_bounds.rs:241`, and peers).
- Aztec model builder still accumulates fallback sink detections (`crates/aztec-lint-aztec/src/model_builder.rs:49`, `crates/aztec-lint-aztec/src/model_builder.rs:156`).

### Needed work
- Make semantic path mandatory in normal mode; move text fallback behind explicit degraded mode.
- Port high-impact rules to typed `RuleQuery` and semantic IDs end-to-end.
- Reduce string pattern matching in Aztec model construction for critical protocol/soundness rules.

## 3) Lint group model gap
### Gap
Clippy exposes first-class lint groups (`clippy::all`, `clippy::style`, `clippy::perf`, etc.); aztec-lint currently exposes pack/tier selectors only.

### Evidence
- Clippy group registration is explicit (`declare_clippy_lint/src/lib.rs:51`).
- aztec-lint ruleset selectors are limited to pack and maturity tier (`crates/aztec-lint-core/src/config/types.rs:485`).

### Needed work
- Add category-based rulesets (correctness/soundness/protocol/privacy/maintainability/performance).
- Add named composite groups akin `all`, `pedantic`, `nursery`, `restriction`.
- Allow group-level allow/warn/deny in config and CLI.

## 4) Config sophistication gap
### Gap
aztec-lint lacks Clippy-scale per-lint configurable knobs and unknown-field UX.

### Evidence
- Clippy central config surface is large (`clippy_config/src/conf.rs:344`) and unknown fields are diagnosed with suggestions (`clippy_config/src/conf.rs:1157`).
- Clippy enforces config-test coverage in `ui-toml` (`clippy_config/src/conf.rs:1232`).
- aztec-lint config is profile/ruleset/override + Aztec naming/domain knobs, with no per-lint schema layer (`crates/aztec-lint-core/src/config/types.rs`).
- Config file parsing currently deserializes `RawConfig` directly (`crates/aztec-lint-core/src/config/loader.rs:51`), without Clippy-style unknown-key suggestion handling.

### Needed work
- Introduce per-lint typed config schema and validation.
- Add unknown-config-key diagnostics with nearest-key suggestion.
- Add config-contract tests mirroring Clippy’s “every config key is tested” discipline.

## 5) Toolchain integration parity gap
### Gap
Clippy behaves as both `cargo clippy` wrapper and `clippy-driver`; aztec-lint remains standalone CLI.

### Evidence
- Clippy cargo wrapper behavior and `RUSTC_WORKSPACE_WRAPPER` pathing (`src/main.rs:112`, `src/main.rs:122`).
- Clippy rustc callback integration and lint-pass registration (`src/driver.rs:137`, `src/driver.rs:166`).
- aztec-lint runs its own check/fix pipeline (`crates/aztec-lint-cli/src/commands/check.rs:137`) without a package-manager-native wrapper mode analogous to `cargo clippy`.

### Needed work
- Add `nargo`-native wrapper UX (`nargo lint`/`nargo clippy`-style entrypoint).
- Support workspace/package target semantics with parity to package-manager workflows.
- Stabilize machine-output contracts for CI/editor tooling in wrapper mode.

## 6) Diagnostics/fix applicability parity gap
### Gap
Diagnostic invariants are solid, but auto-fix remains intentionally conservative (safe-only).

### Evidence
- aztec-lint skips non-safe fixes (`crates/aztec-lint-core/src/fix/apply.rs:388`).
- Clippy compile-test harness tracks rustfix/applicability modes broadly (`tests/compile-test.rs:195`, `tests/compile-test.rs:540`).

### Needed work
- Add configurable fix modes (safe-only default, optional assisted mode for lower applicability).
- Add stronger per-rule applicability contracts and coverage checks.
- Add richer “why not fixed” structured output for editors/CI bots.

## 7) Test ecosystem parity gap
### Gap
aztec-lint has meaningful matrix coverage, but Clippy’s compile-test ecosystem remains much larger and more automated.

### Evidence
- aztec-lint has deterministic UI/corpus/fix matrix tests (`crates/aztec-lint-cli/tests/ui_matrix.rs`, `crates/aztec-lint-cli/tests/corpus_matrix.rs`, `crates/aztec-lint-cli/tests/fix_matrix.rs`).
- Clippy runs large compile-test suites with dedicated `ui`, `ui-toml`, `ui-cargo`, and rustfix validation (`tests/compile-test.rs:234`, `tests/compile-test.rs:271`, `tests/compile-test.rs:300`).

### Needed work
- Expand project-level fixture corpus and edge-case density, especially for semantic false-positive boundaries.
- Add compile-test style harness features: applicability summary gates, per-lint config coverage, and cargo-workspace scenario breadth.

## 8) Plugin runtime integration gap
### Gap
Plugin API exists but is not executed in `check`/`fix`.

### Evidence
- Plugin API doc explicitly states non-execution in command flow (`docs/plugin-api-v0.md:71`).

### Needed work
- Decide scope: either fully integrate plugin execution (ordering/config/suppression/fix/output), or keep explicitly out-of-scope for Clippy parity target.

---

## Improvement roadmap (toward Clippy-closest state)

## Phase A: 0.6.0 parity consolidation (short-term)
- Ship planned second-wave rules (`AZTEC036`..`AZTEC041`) and close accepted backlog.
- Add category group selectors and CLI controls.
- Add strict config-key validation with suggestion diagnostics.
- Exit criteria:
  - Active lint count >= 28.
  - Accepted roadmap IDs have runtime implementations (not only fixture placeholders).

## Phase B: Semantic-first engine hardening (short/medium)
- Require semantic model in normal mode; gate text fallback behind explicit degraded option.
- Refactor top noisy rules to typed query flow; target `NOIR001/2/10/20/30` and `AZTEC020/21/22/30/31/32/33/34` first.
- Exit criteria:
  - `ctx.query()` adopted by core rule set.
  - Substantial reduction of source-slice parsing in semantic path.

## Phase C: Config and lifecycle maturity (medium)
- Introduce per-lint config schema.
- Add unknown-field diagnostics with typo suggestions.
- Add compiler-version-aware rule gating equivalent to MSRV strategy.
- Exit criteria:
  - Per-lint config keys documented and tested.
  - Unknown config keys fail with actionable suggestions.

## Phase D: Toolchain + fix UX parity (medium)
- Build `nargo` wrapper integration analogous to `cargo clippy`.
- Add fix modes beyond safe-only while preserving deterministic default.
- Exit criteria:
  - Wrapper command supports workspace/package targeting.
  - CI/editor consumers can rely on stable structured output for diagnostics and fix decisions.

## Phase E: Scale + compile-test growth (long-term)
- Grow toward 60-100 active lints with clear group semantics.
- Expand compile-test style suites (ui/config/cargo-like scenarios) and performance baselines.
- Exit criteria:
  - Rule corpus and test breadth support “Clippy-like” utility claims for Noir/Aztec ecosystems.

---

## Bottom line

`aztec-lint` has moved from “foundational platform gap” to “scale + semantic precision + ecosystem parity” gap.  
The highest leverage next steps are:

1. Implement accepted second-wave rules (`AZTEC036`..`AZTEC041`).
2. Shift remaining heuristic-heavy checks to semantic-first query execution.
3. Add Clippy-style config/group UX and wrapper integration.
