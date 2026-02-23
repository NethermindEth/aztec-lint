# Second-Wave Implementation Plan (`AZTEC036`-`AZTEC041`)

This plan is intentionally linear. Execute steps in order. Do not skip validation gates.

## Scope

Rules to implement for `0.6.0`:
- `AZTEC036` (`privacy`)
- `AZTEC037` (`privacy`)
- `AZTEC038` (`correctness`)
- `AZTEC039` (`correctness`)
- `AZTEC040` (`protocol`)
- `AZTEC041` (`correctness`)

Required matrix for each rule:
- `positive`
- `negative`
- `suppressed`
- `false_positive_guard`

## Step 1: Baseline Safety Check **COMPLETED**

Action:
- Confirm repo builds/tests before changes.

Files touched:
- none

Validation:
1. `cargo check --workspace --locked`
2. `cargo test -p aztec-lint-rules full_registry_matches_canonical_lint_catalog --locked`

Failure mode:
- If baseline is already red, stop and fix baseline first; do not stack second-wave changes on top.

## Step 2: Add Rule Modules and Registry Wiring **COMPLETED**

Action:
- Add module exports for `aztec036`..`aztec041`.
- Add registry imports and `register(Box::new(...))` entries.

Files touched:
- `crates/aztec-lint-rules/src/aztec/mod.rs`
- `crates/aztec-lint-rules/src/engine/registry.rs`

Validation:
1. `cargo check -p aztec-lint-rules --locked`
2. `cargo test -p aztec-lint-rules full_registry_matches_canonical_lint_catalog --locked`

Failure mode:
- Registry panic or unresolved imports means wiring is incomplete.

## Step 3: Add Canonical Catalog Entries (`AZTEC036`..`AZTEC041`) **COMPLETED**

Action:
- Add six `LintSpec` entries with full docs fields.
- Use `introduced_in: "0.6.0"`, `maturity: Preview`, roadmap-aligned category/policy.

Files touched:
- `crates/aztec-lint-core/src/lints/mod.rs`

Validation:
1. `cargo test -p aztec-lint-core lint_catalog_invariants_hold --locked`
2. `cargo test -p aztec-lint-core lints_reference_doc_matches_catalog --locked`
3. `cargo test -p aztec-lint-rules full_registry_matches_canonical_lint_catalog --locked`

Failure mode:
- Catalog invariant errors (missing docs, bad policy, bad semver, registry/catalog divergence).

## Step 4: Implement `AZTEC036` End-to-End **COMPLETED**

Action:
- Implement `AZTEC036` rule logic based on `AZTEC002` with stricter sink scope.
- Trigger only when secret branch influences enqueue behavior.
- Add matrix fixtures and advanced-rule test.
- Replace scaffolded UI accepted fixtures.

Files touched:
- `crates/aztec-lint-rules/src/aztec/aztec036_secret_branch_affects_enqueue.rs`
- `fixtures/aztec/rule_cases/aztec036_positive.nr`
- `fixtures/aztec/rule_cases/aztec036_negative.nr`
- `fixtures/aztec/rule_cases/aztec036_suppressed.nr`
- `fixtures/aztec/rule_cases/aztec036_false_positive_guard.nr`
- `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`
- `fixtures/ui/accepted/AZTEC036/positive.nr`
- `fixtures/ui/accepted/AZTEC036/negative.nr`
- `fixtures/ui/accepted/AZTEC036/suppressed.nr`
- `fixtures/ui/accepted/AZTEC036/false_positive_guard.nr`

Validation:
1. `cargo test -p aztec-lint-rules aztec036_fixture_matrix --locked`
2. `cargo test -p aztec-lint-cli --test ui_matrix accepted_lints_have_required_ui_fixture_pack --locked`

Failure mode:
- False positives from secret branches that do not alter enqueue shape.

## Step 5: Implement `AZTEC037` End-to-End

Action:
- Add/extend taint sink coverage for delivery-count patterns (`deliver(...)` style).
- Implement `AZTEC037` to report secret branch influence on number/presence of deliveries.
- Add matrix fixtures and advanced-rule test.
- Replace scaffolded UI accepted fixtures.

Files touched:
- `crates/aztec-lint-aztec/src/taint/graph.rs`
- `crates/aztec-lint-aztec/src/taint/propagate.rs` (if sink propagation needs updates)
- `crates/aztec-lint-rules/src/aztec/aztec037_secret_branch_affects_delivery_count.rs`
- `fixtures/aztec/rule_cases/aztec037_positive.nr`
- `fixtures/aztec/rule_cases/aztec037_negative.nr`
- `fixtures/aztec/rule_cases/aztec037_suppressed.nr`
- `fixtures/aztec/rule_cases/aztec037_false_positive_guard.nr`
- `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`
- `fixtures/ui/accepted/AZTEC037/positive.nr`
- `fixtures/ui/accepted/AZTEC037/negative.nr`
- `fixtures/ui/accepted/AZTEC037/suppressed.nr`
- `fixtures/ui/accepted/AZTEC037/false_positive_guard.nr`

Validation:
1. `cargo test -p aztec-lint-aztec --locked`
2. `cargo test -p aztec-lint-rules aztec037_fixture_matrix --locked`

Failure mode:
- Over-reporting when branch exists but delivery count remains constant.

## Step 6: Implement `AZTEC038` End-to-End

Action:
- Implement change-note randomness freshness checks.
- Flag reused randomness and deterministic derivations missing uniqueness context.
- Add matrix fixtures and advanced-rule test.
- Replace scaffolded UI accepted fixtures.

Files touched:
- `crates/aztec-lint-rules/src/aztec/aztec038_change_note_missing_fresh_randomness.rs`
- `crates/aztec-lint-rules/src/aztec/text_scan.rs` (only if helper extraction is needed)
- `fixtures/aztec/rule_cases/aztec038_positive.nr`
- `fixtures/aztec/rule_cases/aztec038_negative.nr`
- `fixtures/aztec/rule_cases/aztec038_suppressed.nr`
- `fixtures/aztec/rule_cases/aztec038_false_positive_guard.nr`
- `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`
- `fixtures/ui/accepted/AZTEC038/positive.nr`
- `fixtures/ui/accepted/AZTEC038/negative.nr`
- `fixtures/ui/accepted/AZTEC038/suppressed.nr`
- `fixtures/ui/accepted/AZTEC038/false_positive_guard.nr`

Validation:
1. `cargo test -p aztec-lint-rules aztec038_fixture_matrix --locked`

Failure mode:
- Noise due to constructor naming variance.

## Step 7: Implement `AZTEC039` End-to-End

Action:
- Implement partial-spend balance invariant checks.
- Detect high-signal underflow/reconciliation anti-patterns.
- Add matrix fixtures and advanced-rule test.
- Replace scaffolded UI accepted fixtures.

Files touched:
- `crates/aztec-lint-rules/src/aztec/aztec039_partial_spend_not_balanced.rs`
- `crates/aztec-lint-rules/src/aztec/text_scan.rs` (if parsing helpers are needed)
- `fixtures/aztec/rule_cases/aztec039_positive.nr`
- `fixtures/aztec/rule_cases/aztec039_negative.nr`
- `fixtures/aztec/rule_cases/aztec039_suppressed.nr`
- `fixtures/aztec/rule_cases/aztec039_false_positive_guard.nr`
- `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`
- `fixtures/ui/accepted/AZTEC039/positive.nr`
- `fixtures/ui/accepted/AZTEC039/negative.nr`
- `fixtures/ui/accepted/AZTEC039/suppressed.nr`
- `fixtures/ui/accepted/AZTEC039/false_positive_guard.nr`

Validation:
1. `cargo test -p aztec-lint-rules aztec039_fixture_matrix --locked`

Failure mode:
- Missed equivalent arithmetic forms or noisy detections on safe code.

## Step 8: Implement `AZTEC040` End-to-End

Action:
- Implement initializer misuse check: initializer without `#[only_self]`.
- Prefer semantic entrypoint kinds (`Initializer`/`OnlySelf`) from Aztec model.
- Add matrix fixtures and advanced-rule test.
- Replace scaffolded UI accepted fixtures.

Files touched:
- `crates/aztec-lint-rules/src/aztec/aztec040_initializer_not_only_self.rs`
- `fixtures/aztec/rule_cases/aztec040_positive.nr`
- `fixtures/aztec/rule_cases/aztec040_negative.nr`
- `fixtures/aztec/rule_cases/aztec040_suppressed.nr`
- `fixtures/aztec/rule_cases/aztec040_false_positive_guard.nr`
- `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`
- `fixtures/ui/accepted/AZTEC040/positive.nr`
- `fixtures/ui/accepted/AZTEC040/negative.nr`
- `fixtures/ui/accepted/AZTEC040/suppressed.nr`
- `fixtures/ui/accepted/AZTEC040/false_positive_guard.nr`

Validation:
1. `cargo test -p aztec-lint-rules aztec040_fixture_matrix --locked`

Failure mode:
- Framework-level initializer guards not expressed as `#[only_self]`.

## Step 9: Implement `AZTEC041` End-to-End

Action:
- Implement cast truncation risk checks for Field<->integer conversions.
- Reuse or extend range-guard detection utilities from `AZTEC034`.
- Add matrix fixtures and advanced-rule test.
- Replace scaffolded UI accepted fixtures.

Files touched:
- `crates/aztec-lint-rules/src/aztec/aztec041_cast_truncation_risk.rs`
- `crates/aztec-lint-rules/src/aztec/aztec034_hash_input_not_range_constrained.rs` (shared helper extraction if needed)
- `fixtures/aztec/rule_cases/aztec041_positive.nr`
- `fixtures/aztec/rule_cases/aztec041_negative.nr`
- `fixtures/aztec/rule_cases/aztec041_suppressed.nr`
- `fixtures/aztec/rule_cases/aztec041_false_positive_guard.nr`
- `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`
- `fixtures/ui/accepted/AZTEC041/positive.nr`
- `fixtures/ui/accepted/AZTEC041/negative.nr`
- `fixtures/ui/accepted/AZTEC041/suppressed.nr`
- `fixtures/ui/accepted/AZTEC041/false_positive_guard.nr`

Validation:
1. `cargo test -p aztec-lint-rules aztec041_fixture_matrix --locked`

Failure mode:
- Safe helper conversions not recognized, causing false positives.

## Step 10: Update Rule Test Matrix Aggregation

Action:
- Ensure `aztec_advanced_rules` includes fixture matrix tests for all six new rules.

Files touched:
- `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`

Validation:
1. `cargo test -p aztec-lint-rules aztec_advanced_rules --locked`

Failure mode:
- Missing per-rule matrix test allows regressions to ship.

## Step 11: Update CLI Golden Contracts and Rule Counts

Action:
- Update hardcoded `rules` output golden to include `AZTEC036`..`AZTEC041`.
- Update profile active rule counts.
Expected count changes:
- `aztec_pack`: `13 -> 19`
- `aztec` profile total: `22 -> 28`

Files touched:
- `crates/aztec-lint-cli/tests/cli_golden.rs`
- `fixtures/text/noir_core_minimal_with_suggestions.txt`
- `fixtures/ui/cases/noir100_magic_array_len.text.v0.5.0.snap`
- `fixtures/ui/cases/noir100_magic_array_len.text.v0.4.0.snap` (if still asserted)

Validation:
1. `cargo test -p aztec-lint-cli --test cli_golden --locked`
2. `cargo test -p aztec-lint-cli --test ui_matrix --locked`

Failure mode:
- Snapshot/header drift because rule count changes were not propagated.

## Step 12: Regenerate and Verify Lint Docs Artifacts

Action:
- Sync generated lint docs from canonical catalog.

Files touched (generated):
- `docs/lints-reference.md`
- `docs/portal/index.md`
- `docs/portal/search-index.json`
- `docs/portal/lints/aztec036.md`
- `docs/portal/lints/aztec037.md`
- `docs/portal/lints/aztec038.md`
- `docs/portal/lints/aztec039.md`
- `docs/portal/lints/aztec040.md`
- `docs/portal/lints/aztec041.md`

Validation:
1. `cargo xtask update-lints --check --locked`
2. `cargo xtask docs-portal --check`

Failure mode:
- Generated artifact drift blocks CI.

## Step 13: Close Roadmap/Intake Status Loop

Action:
- Move second-wave entries from planned/accepted to implemented states.

Files touched:
- `docs/NEW_LINTS.md`
- `docs/rule-roadmap.md`
- `docs/portal/roadmap/index.md`
- `docs/portal/roadmap/accepted.md`
- `docs/portal/roadmap/covered.md`
- `docs/portal/roadmap/deferred.md`
- `docs/portal/roadmap/rejected.md`

Validation:
1. `cargo xtask lint-intake --source docs/NEW_LINTS.md --check`
2. `cargo xtask docs-portal --check`

Failure mode:
- Intake table and generated roadmap pages diverge.

## Step 14: Update Changelog Rule Growth

Action:
- Update `Rule Growth by Category` for second-wave additions.
Expected deltas:
- `privacy +2`
- `correctness +3`
- `protocol +1`

Files touched:
- `CHANGELOG.md`

Validation:
1. Manual cross-check against `aztec-lint rules` output.

Failure mode:
- Release notes disagree with actual catalog contents.

## Step 15: Final Gate (Must Pass Before Merge)

Run in order:
1. `cargo test --workspace --locked`
2. `cargo test -p aztec-lint-cli --test ui_matrix --locked`
3. `cargo test -p aztec-lint-cli --test fix_matrix --locked`
4. `cargo test -p aztec-lint-cli --test corpus_matrix --locked`
5. `cargo xtask update-lints --check --locked`
6. `cargo xtask docs-portal --check`
7. `cargo xtask lint-intake --source docs/NEW_LINTS.md --check`

Merge checklist:
- `AZTEC036`..`AZTEC041` are active in catalog and registry.
- All six matrix suites pass in `aztec_advanced_rules`.
- No TODO scaffold text remains in `fixtures/ui/accepted/AZTEC036`..`fixtures/ui/accepted/AZTEC041`.
- Generated docs and roadmap artifacts are clean.
