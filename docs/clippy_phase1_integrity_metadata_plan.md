# Clippy Gap Phase 1 Implementation Plan

Source: `CLIPPY_GAP_ANALYSIS.md` ("Phase 1: Integrity and metadata foundation")
Date: 2026-02-20

## Scope
Deliver these four outcomes:
1. Unify rule source of truth (registry/catalog/config generation).
2. Add unknown-rule validation and fail-fast behavior.
3. Add lint metadata model: category, introduced version, lifecycle state, docs content.
4. Expand `explain` to full lint docs.

This phase is metadata/integrity work only. No new rule semantics are added here.

## Current Gaps (Repo-Specific)
- Rule lists are duplicated and drift-prone:
  - Runtime registry: `crates/aztec-lint-rules/src/engine/registry.rs`
  - CLI catalog docs: `crates/aztec-lint-cli/src/commands/catalog.rs`
  - Config defaults/rulesets: `crates/aztec-lint-core/src/config/types.rs`
- Unknown rule IDs from config/CLI overrides are not rejected early.
- Rule metadata is minimal (`id`, `pack`, `policy`, `default_level`, `confidence`), no lifecycle/doc model.
- `explain` prints only summary fields.

## Design Principles
- Single canonical metadata source, all other surfaces generated from it.
- Fail fast on invalid rule IDs before running lints.
- Keep outputs deterministic and stable for CI.
- Backward compatibility: existing rule IDs keep behavior unless explicitly deprecated/renamed.

## Discrete Testable Phases

### Phase 1: Canonical Metadata Types + Catalog Seed **COMPLETE**
Objective:
- Introduce one canonical lint metadata model and seed it with current implemented rules.

Implementation:
- Add:
  - `crates/aztec-lint-core/src/lints/mod.rs`
  - `crates/aztec-lint-core/src/lints/types.rs`
- Define:
  - `LintCategory`
  - `LintLifecycleState`
  - `LintDocs`
  - `LintSpec`
- Publish lookup APIs:
  - `all_lints()`
  - `find_lint(rule_id)`

Tests:
- Unit tests for catalog invariants:
  - unique IDs
  - canonical uppercase ID format
  - required docs fields present for active lints

Validation commands:
- `cargo test -p aztec-lint-core lints::`
- `cargo test -p aztec-lint-core`

Exit criteria:
- Canonical lint metadata exists and passes invariant tests.

---

### Phase 2: Migrate Config and Catalog to Canonical Source **COMPLETE**
Objective:
- Eliminate duplicated rule ID/default metadata in config and CLI catalog.

Implementation:
- Refactor `crates/aztec-lint-core/src/config/types.rs`:
  - derive ruleset defaults from canonical catalog instead of static arrays.
- Refactor `crates/aztec-lint-cli/src/commands/catalog.rs`:
  - render `rules`/`find_rule` from canonical metadata.

Tests:
- Update/add tests that assert:
  - ruleset resolution still yields expected active rules per profile
  - `rules` command output still includes all expected implemented rules

Validation commands:
- `cargo test -p aztec-lint-core config::types::tests::`
- `cargo test -p aztec-lint-cli rules_command_matches_golden_output -- --nocapture`

Exit criteria:
- Config defaults and CLI catalog have no independent hardcoded rule tables.

---

### Phase 3: Unknown Rule Validation + Fail-Fast **COMPLETE**
Objective:
- Reject unknown rule IDs early across CLI overrides and config-resolved levels.

Implementation:
- Add `ConfigError::UnknownRuleId { rule_id, source }` (or equivalent).
- Validate rule IDs in:
  - `--allow/--warn/--deny`
  - any resolved profile/ruleset overrides
- Standardize error messaging with actionable hint (and replacement when lifecycle says renamed).

Tests:
- CLI/integration tests:
  - `aztec-lint --deny DOES_NOT_EXIST` fails with clear error and non-zero exit.
  - unknown rule in config profile fails before analysis starts.

Validation commands:
- `cargo test -p aztec-lint-core config::types::tests::`
- `cargo test -p aztec-lint-cli invalid_flag_combination_returns_exit_code_two -- --nocapture`
- add and run dedicated unknown-rule golden tests in `crates/aztec-lint-cli/tests/cli_golden.rs`

Exit criteria:
- Unknown rule IDs cannot silently pass into a lint run.

---

### Phase 4: Lifecycle + Rich Docs Metadata Completion **COMPLETE**
Objective:
- Enrich lint metadata to include Clippy-like lifecycle and documentation payload.

Implementation:
- Extend `LintSpec` with:
  - `category`
  - `introduced_in`
  - `lifecycle` (`Active`, `Deprecated`, `Renamed`, `Removed`)
  - rich docs fields (`summary`, `what_it_does`, `why_this_matters`, `known_limitations`, `how_to_fix`, `examples`, `references`)
- Populate metadata entries for all implemented lints.

Tests:
- Unit tests for lifecycle integrity:
  - renamed lints map to valid targets
  - removed/deprecated states carry required metadata
- Snapshot stability tests for serialized metadata views if exposed.

Validation commands:
- `cargo test -p aztec-lint-core lints::`

Exit criteria:
- Every implemented lint has complete lifecycle + docs metadata.

---

### Phase 5: Full `explain` Output + Drift Guardrails
Objective:
- Upgrade `explain` to full docs output and enforce metadata/registry integrity in CI.

Implementation:
- Refactor `crates/aztec-lint-cli/src/commands/explain.rs` to render:
  - title + identifiers
  - category/pack/policy/confidence/default level
  - introduced version + lifecycle
  - rationale and limitation sections
  - fix guidance + examples + references
- Add integrity check between runtime registry and canonical metadata:
  - registry IDs must all exist in canonical catalog
  - optional: canonical entries marked implemented must exist in registry

Tests:
- Golden tests for `explain` output (full-section rendering).
- Engine/bootstrap tests for metadata/registry mismatch failure.

Validation commands:
- `cargo test -p aztec-lint-cli explain_command_matches_golden_output -- --nocapture`
- `cargo test -p aztec-lint-rules engine::tests::`
- `cargo test -p aztec-lint-cli -p aztec-lint-core -p aztec-lint-rules`

Exit criteria:
- `explain` is full-doc quality.
- CI blocks metadata drift between canonical catalog and runtime registry.

## Final Acceptance Criteria
- Exactly one canonical lint metadata source exists.
- Catalog/config/engine no longer maintain independent rule ID lists.
- Unknown rule IDs are rejected before rule execution.
- Lint metadata includes category, introduced version, lifecycle state, and rich docs content.
- `explain` prints full lint docs, not only summary.
- CI fails on metadata drift between canonical catalog and runtime registry.

## Suggested Commit Breakdown
1. `core: add canonical lint metadata model and catalog`
2. `core/config: derive ruleset defaults from canonical catalog`
3. `cli/catalog: read rule listing from canonical metadata`
4. `core/config: fail fast on unknown rule overrides`
5. `core/lints: add lifecycle and rich docs metadata`
6. `cli/explain: render full lint documentation`
7. `rules/tests: add registry-catalog drift guard`
