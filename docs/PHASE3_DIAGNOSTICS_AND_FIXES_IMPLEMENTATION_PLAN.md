# Diagnostics and Fixes Implementation Plan

Date: 2026-02-21
Scope: `## Diagnostics and fixes (medium-term)` from `docs/CLIPPY_GAP_ANALYSIS.md`

## Required outcomes (source of truth)

From `docs/CLIPPY_GAP_ANALYSIS.md:230-233`:
1. Introduce strict diagnostic/fix invariants and validation tests.
2. Add richer machine-applicable suggestion model and grouped edits.
3. Improve suppression scoping + lint-level semantics.

Related constraints:
- `docs/CLIPPY_GAP_ANALYSIS.md:157-172` requires stronger diagnostics metadata guarantees, grouped multipart fixes, transaction semantics, and better fix provenance.
- Current suppression contract is item-local only (`docs/suppression.md:23-25`, `docs/decisions/0002-suppression-semantics.md:23-26`).
- Current fix ADR v0 disallows multi-location coordinated edits (`docs/decisions/0004-fix-safety-policy.md:23-28`).

## Baseline in current code (gap map)

### Diagnostics and suggestions
- `Diagnostic` stores `structured_suggestions: Vec<StructuredSuggestion>` and `fixes: Vec<Fix>` with no first-class grouped edit object (`crates/aztec-lint-core/src/diagnostics/types.rs:177-196`).
- `multipart_suggestion(...)` currently flattens each part into independent `StructuredSuggestion` entries (`crates/aztec-lint-core/src/diagnostics/types.rs:151-166`).
- JSON/SARIF emit and sort flattened suggestions/fixes only (`crates/aztec-lint-core/src/output/json.rs:60-83`, `crates/aztec-lint-core/src/output/sarif.rs:173-262`).

### Fix engine
- Fix candidate extraction only considers explicit `fixes` plus machine-applicable flattened suggestions (`crates/aztec-lint-core/src/fix/apply.rs:132-158`).
- Conflict resolution and application are per single edit, not transactional group apply (`crates/aztec-lint-core/src/fix/apply.rs:221-265`, `crates/aztec-lint-core/src/fix/apply.rs:268-374`).

### Suppression and lint levels
- Suppressions are parsed as `#[allow(...)]` only, with item-local ranges (`crates/aztec-lint-rules/src/engine/context.rs:223-320`).
- Engine applies profile/CLI level globally per rule, then applies suppression as a boolean marker (`crates/aztec-lint-rules/src/engine/mod.rs:62-87`).
- CLI filter/exit-code logic treats suppressed diagnostics as non-blocking and applies thresholds afterward (`crates/aztec-lint-cli/src/commands/check.rs:453-493`, `crates/aztec-lint-cli/src/commands/fix.rs:96-126`).

### Existing tests
- Strong unit coverage exists for single-edit fix behavior and overlap ranking (`crates/aztec-lint-core/src/fix/apply.rs:466-734`).
- Suppression tests cover short/scoped allow and item-level behavior only (`crates/aztec-lint-rules/src/engine/context.rs:520-563`, `crates/aztec-lint-rules/tests/noir_core_rules.rs:139-169`).

## Implementation sequence

## Step 0: Align ADR/doc contracts before code changes **COMPLETED**

Files:
- `docs/decisions/0002-suppression-semantics.md`
- `docs/decisions/0004-fix-safety-policy.md`
- `docs/suppression.md`
- `docs/rule-authoring.md`
- New: `docs/decisions/0005-diagnostic-invariants-and-suggestion-groups.md`

Changes:
1. Amend suppression ADR to include file/module scopes and scoped lint-level directives (`allow/warn/deny`) precedence.
2. Amend fix safety ADR to permit same-file multi-location grouped edits with atomic apply semantics.
3. Define compatibility policy for diagnostic JSON/SARIF fields (legacy + new fields during migration window).

Validation:
- `rg -n "file-level|module-level|grouped|transaction|allow|warn|deny" docs/decisions docs/suppression.md docs/rule-authoring.md`
- Manual checklist in ADRs updated and signed.

Failure modes to watch:
- Implementing grouped edits or broader scoping while docs/ADRs still declare them invalid.

## Step 1: Add strict diagnostic and fix invariant validation layer **COMPLETED**

Files:
- New: `crates/aztec-lint-core/src/diagnostics/validate.rs`
- `crates/aztec-lint-core/src/diagnostics/mod.rs`
- `crates/aztec-lint-rules/src/engine/mod.rs`
- `crates/aztec-lint-cli/src/commands/check.rs`
- `crates/aztec-lint-cli/src/commands/fix.rs`

Changes:
1. Add `validate_diagnostic` / `validate_diagnostics` API with deterministic violation enum.
2. Enforce invariants at engine boundary before sorting/output:
- non-empty `rule_id`, `policy`, `message`
- `primary_span.start <= end`
- no invalid span in notes/helps/suggestions/fixes
- `suppressed => suppression_reason.is_some()`
- no overlapping edits inside a single future suggestion group
3. Convert violations into internal error path (`exit code 2`) with actionable message including rule id and span.

Validation:
- Add unit tests in new `validate.rs` for pass/fail cases.
- `cargo test -p aztec-lint-core diagnostics::`
- `cargo test -p aztec-lint-rules engine::mod::tests`

Failure modes to watch:
- Overly strict validation blocks existing legitimate diagnostics.
- Nondeterministic violation ordering causes flaky tests.

## Step 2: Introduce suggestion model v2 with grouped edits **COMPLETED**

Files:
- `crates/aztec-lint-core/src/diagnostics/types.rs`
- `crates/aztec-lint-core/src/diagnostics/mod.rs`
- `crates/aztec-lint-core/src/output/json.rs`
- `crates/aztec-lint-core/src/output/sarif.rs`
- `crates/aztec-lint-core/src/output/text.rs`

Changes:
1. Add first-class model (names can vary, shape should be equivalent):
- `TextEdit { span, replacement }`
- `SuggestionGroup { id, message, applicability, edits, provenance }`
2. Add `Diagnostic.suggestion_groups` with serde defaults.
3. Rework helper methods:
- `span_suggestion` creates one group with one edit.
- `multipart_suggestion` creates one group with N edits (not flattened).
4. Keep legacy fields (`structured_suggestions`, `fixes`, `suggestions`) for compatibility during migration; derive them from group model where needed.

Validation:
- Extend `diagnostics/types.rs` tests for serialization compatibility and helper behavior.
- Ensure old JSON shape still deserializes (`legacy_diagnostic_json_deserializes_with_defaults`).
- `cargo test -p aztec-lint-core diagnostics::types::tests`

Failure modes to watch:
- Breaking downstream consumers by removing/renaming legacy fields too early.
- Duplication drift when legacy and v2 fields disagree.

## Step 3: Upgrade fix engine to transactional grouped edit application **COMPLETED**

Files:
- `crates/aztec-lint-core/src/fix/apply.rs`
- `crates/aztec-lint-core/src/fix/mod.rs`
- `crates/aztec-lint-cli/src/commands/fix.rs`

Changes:
1. Replace single-edit candidate pipeline with group candidates.
2. Add group-level skip reasons (e.g. `InvalidGroupSpan`, `GroupOverlap`, `MixedFileGroup`, `GroupNoop`).
3. Apply all edits in a selected group atomically (all-or-none), reverse by span start within the group.
4. Preserve deterministic ranking policy from current engine (`confidence`, `rule_id`, source, ordinal) but apply at group level.
5. Extend fix report with group provenance and why-not-fixed reasons suitable for CI/editor output.

Validation:
- Add tests in `fix/apply.rs`:
- atomic rollback when one edit in group fails
- same-group overlap rejection
- overlapping groups deterministic winner selection
- idempotence with grouped edits
- `cargo test -p aztec-lint-core fix::apply::tests`

Failure modes to watch:
- Partial group writes leading to corrupted source.
- Group overlap detection missing zero-length insertion edge cases currently handled in `ranges_overlap`.

## Step 4: Migrate rule emitters to suggestion groups **COMPLETED**

Files:
- `crates/aztec-lint-rules/src/noir_core/noir001_unused.rs`
- `crates/aztec-lint-rules/src/noir_core/noir100_magic_numbers.rs`
- `crates/aztec-lint-rules/src/aztec/aztec021_range_before_hash.rs`
- `crates/aztec-lint-rules/tests/noir_core_rules.rs`
- `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`

Changes:
1. Emit group-backed suggestions only.
2. For rules that truly need multi-edit outcomes, emit one `multipart_suggestion` group.
3. Keep applicability discipline:
- `MachineApplicable` only when edits are deterministic and safe.
- exploratory guidance remains `MaybeIncorrect`.

Validation:
- Update/extend rule tests asserting applicability and suggestion-group structure.
- `cargo test -p aztec-lint-rules`

Failure modes to watch:
- Semantic and text-fallback paths producing different edit sets for same finding.
- Machine-applicable mislabeling causing unsafe auto-fix application.

## Step 5: Implement scoped lint-level semantics (allow/warn/deny) and broader suppression scopes **COMPLETED**

Files:
- `crates/aztec-lint-rules/src/engine/context.rs`
- `crates/aztec-lint-rules/src/engine/mod.rs`
- `crates/aztec-lint-core/src/config/types.rs`
- `crates/aztec-lint-cli/src/commands/check.rs`
- `crates/aztec-lint-cli/src/commands/fix.rs`

Changes:
1. Generalize current suppression parsing into directive parsing for `#[allow]`, `#[warn]`, `#[deny]`.
2. Add scope kinds: file, module, item; resolve effective level at diagnostic span using nearest-scope precedence.
3. Apply scoped level in engine:
- `allow` => suppressed (with precise reason)
- `warn`/`deny` => override severity at emission time
4. Keep global profile/CLI levels as baseline; scoped directives are local overrides.

Validation:
- New `context.rs` tests for precedence matrix (file vs module vs item).
- New `engine/mod.rs` tests for severity override behavior.
- CLI golden tests for suppression visibility and exit-code behavior with scoped directives.
- `cargo test -p aztec-lint-rules engine::context::tests`
- `cargo test -p aztec-lint-cli --test cli_golden`

Failure modes to watch:
- Precedence bugs silently downgrading critical diagnostics.
- Module/file scope parser mismatching Noir syntax edge cases.

## Step 6: Output contract and determinism hardening for grouped suggestions **COMPLETED**

Files:
- `crates/aztec-lint-core/src/output/json.rs`
- `crates/aztec-lint-core/src/output/sarif.rs`
- `crates/aztec-lint-core/src/output/text.rs`
- `fixtures/text/noir_core_minimal_with_suggestions.txt`
- `fixtures/sarif/noir_core_minimal.sarif.json`

Changes:
1. JSON: emit sorted `suggestion_groups` (plus legacy compatibility fields).
2. SARIF: map each suggestion group to one SARIF fix with multiple replacements where applicable.
3. Text: render grouped suggestions clearly, including applicability and grouped edit count.
4. Ensure deterministic ordering keys include group id + edit spans.

Validation:
- `cargo test -p aztec-lint-core output::json::tests`
- `cargo test -p aztec-lint-core output::sarif::tests`
- `cargo test -p aztec-lint-core output::text::tests`
- `cargo test -p aztec-lint-cli --test cli_golden`

Failure modes to watch:
- Snapshot churn from unstable ordering.
- SARIF consumers misreading grouped replacement structure.

## Step 7: Add regression gates and rollout criteria **COMPLETED**

Files:
- `crates/aztec-lint-core/src/diagnostics/*tests*`
- `crates/aztec-lint-core/src/fix/*tests*`
- `crates/aztec-lint-rules/tests/*`
- `crates/aztec-lint-cli/tests/cli_golden.rs`
- Optional CI workflow updates in `.github/workflows/*`

Changes:
1. Add dedicated test groups:
- invariants (positive + negative)
- grouped fix atomicity + conflict behavior
- scoped lint-level precedence + suppression visibility
2. Add deterministic round-trip checks for JSON/SARIF/text after grouped model migration.
3. Document legacy-field migration guidance without fixed deprecation/removal dates.

Validation:
- `cargo test --workspace --locked`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo fmt --all --check`

Failure modes to watch:
- Partial rollout where model changes land without output/fixer updates.
- Legacy-field removal before downstream tooling migrates.

## Definition of done for diagnostics and fixes track

1. Diagnostics/fixes have enforced invariant checks with failing tests for bad inputs.
2. Grouped machine-applicable suggestions are represented and applied atomically.
3. Suppression and lint-level semantics support broader scoping and precedence rules.
4. JSON/SARIF/text outputs remain deterministic and include grouped suggestion metadata.
5. CLI and engine tests cover new semantics and regression edge cases.
