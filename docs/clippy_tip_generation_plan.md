# Clippy-Style Tip/Suggestion Generation Plan

## Goal
Implement Clippy-equivalent lint tips in `aztec-lint`:
- lint messages can carry `note`, `help`, and structured code suggestions
- suggestions include applicability (safe vs needs review)
- text output shows tips in Clippy style
- `fix` can consume only machine-applicable suggestions by default

## What Clippy Does (source-backed)

### 1. Suggestion generation is lint-authored, not centralized
Clippy does not have one global “tip engine” that invents fixes. Each lint decides:
- whether to emit no tip / note / help / suggestion
- what replacement text to emit
- how safe that replacement is

References:
- `/home/ametel/source/rust-clippy/clippy_utils/src/diagnostics.rs:388` (`span_lint_and_sugg`)
- `/home/ametel/source/rust-clippy/clippy_lints/src/assertions_on_result_states.rs:80`

### 2. Emission API is standardized
Clippy wraps rustc diagnostics with helper APIs:
- `span_lint`
- `span_lint_and_help`
- `span_lint_and_note`
- `span_lint_and_sugg`
- `span_lint_and_then` for custom composition

Reference:
- `/home/ametel/source/rust-clippy/clippy_utils/src/diagnostics.rs:74`
- `/home/ametel/source/rust-clippy/book/src/development/emitting_lints.md:43`

### 3. Applicability drives autofix safety
Clippy uses `Applicability` to label suggestions:
- `MachineApplicable`
- `MaybeIncorrect`
- `HasPlaceholders`
- `Unspecified`

Only high-confidence suggestions are auto-applied via rustfix.

References:
- `/home/ametel/source/rust-clippy/clippy_utils/src/diagnostics.rs:11`
- `/home/ametel/source/rust-clippy/book/src/development/emitting_lints.md:91`

### 4. Suggestion text is snippet-aware and conservative
Suggestion builders use source snippets and downgrade applicability when confidence drops.

Reference:
- `/home/ametel/source/rust-clippy/clippy_utils/src/sugg.rs:1`
- `/home/ametel/source/rust-clippy/clippy_utils/src/sugg.rs:71`
- `/home/ametel/source/rust-clippy/clippy_lints/src/unit_types/unit_arg.rs:68`

### 5. Quality gate: output + fix tests
Clippy validates lint text via `.stderr` golden files and fixability via `.fixed` (rustfix).

Reference:
- `/home/ametel/source/rust-clippy/book/src/development/adding_lints.md:133`
- `/home/ametel/source/rust-clippy/book/src/development/adding_lints.md:173`

## Current `aztec-lint` Baseline

- `Diagnostic` already has `suggestions: Vec<String>` and `fixes: Vec<Fix>` but no structured applicability on suggestions:
  - `crates/aztec-lint-core/src/diagnostics/types.rs:35`
- Text renderer prints generic `help:` lines from `suggestions`, not structured span suggestions:
  - `crates/aztec-lint-core/src/output/text.rs:141`
- Fix engine applies `Fix` entries gated by `FixSafety::Safe`:
  - `crates/aztec-lint-core/src/fix/apply.rs:145`
- Rule emission API only builds base diagnostics; no first-class helper/note/suggestion builders:
  - `crates/aztec-lint-rules/src/engine/context.rs:174`

## Implementation Plan

### Phase 1: Diagnostic model parity with Clippy concepts **COMPLETE**
1. Add `Applicability` enum to core diagnostics.
2. Add structured suggestion type:
   - span
   - message (`help` text)
   - replacement
   - applicability
3. Keep `Fix` for backward compatibility, but define canonical mapping:
   - `MachineApplicable` -> `FixSafety::Safe`
   - everything else -> `FixSafety::NeedsReview`
4. Add optional diagnostic notes/help entries as structured fields (not just raw message strings).

Target files:
- `crates/aztec-lint-core/src/diagnostics/types.rs`
- `crates/aztec-lint-core/src/diagnostics/mod.rs`

### Phase 2: Rule authoring API (`clippy_utils::diagnostics` equivalent) **COMPLETE**
1. Add builder helpers in `RuleContext` or a small `DiagnosticBuilder`:
   - `ctx.diagnostic(...)`
   - `.note(...)`
   - `.help(...)`
   - `.span_suggestion(span, msg, replacement, applicability)`
   - `.multipart_suggestion(...)` (optional in v1, but design now)
2. Keep existing `ctx.diagnostic(...)` call sites valid.
3. Add lint metadata checks to ensure rule IDs + policy remain deterministic.

Target files:
- `crates/aztec-lint-rules/src/engine/context.rs`
- `crates/aztec-lint-rules/src/engine/mod.rs`

### Phase 3: Text output parity (Clippy-like presentation)
1. Render suggestion lines with `help:` and replacement text tied to span.
2. If suggestion span equals primary span, render inline marker-style help.
3. Render additional notes/help blocks in deterministic order.
4. Keep current colorized severity/labels and ensure suggestion/help colors are consistent.

Target files:
- `crates/aztec-lint-core/src/output/text.rs`
- `crates/aztec-lint-core/src/output/ansi.rs`

### Phase 4: JSON/SARIF schema extension
1. JSON output should include structured suggestions + applicability.
2. SARIF output should map suggestions to `fixes`/`artifacts` where possible.
3. Preserve existing fields for backward compatibility for one release.

Target files:
- `crates/aztec-lint-core/src/output/json.rs`
- `crates/aztec-lint-core/src/output/sarif.rs`

### Phase 5: Fix pipeline alignment
1. Convert machine-applicable suggestions into fix candidates automatically.
2. Keep explicit `Fix` support for rules that construct edits directly.
3. Resolve overlap deterministically (already present), but include suggestion-source provenance.
4. Add a flag later for opt-in `MaybeIncorrect` application (default off).

Target files:
- `crates/aztec-lint-core/src/fix/apply.rs`
- `crates/aztec-lint-cli/src/commands/fix.rs`

### Phase 6: Rule migration and quality controls
1. Start with a small pilot set (`NOIR001`, `NOIR100`, `AZTEC021`) to emit structured suggestions.
2. Add per-rule tests for:
   - no suggestion when confidence is insufficient
   - `MachineApplicable` suggestions generate valid fixed output
   - `MaybeIncorrect` suggestions are printed but not auto-applied
3. Add golden tests for text format with suggestions.

Target files:
- `crates/aztec-lint-rules/src/noir_core/*.rs`
- `crates/aztec-lint-rules/src/aztec/*.rs`
- `crates/aztec-lint-cli/tests/cli_golden.rs`

## Design Decisions Required Before Coding
1. Keep both `suggestions` and `fixes` long-term, or unify under one canonical edit type?
2. Should `check --format text` always show non-machine suggestions, or gate with a verbosity flag?
3. Should `fix` accept `--allow-needs-review` for `MaybeIncorrect` equivalent suggestions?
4. Do we want multipart suggestions in v1 or postpone to v2?

## Proposed Delivery Order (low-risk)
1. Phase 1 + Phase 2 (data model + builder API)
2. Phase 3 (text rendering)
3. Phase 5 (machine-applicable fix consumption)
4. Phase 4 (JSON/SARIF extension)
5. Phase 6 (incremental rule migration)

## Acceptance Criteria
1. At least 3 rules emit structured suggestions with applicability.
2. Text output includes Clippy-style `help`/suggestion sections with source spans.
3. `fix` applies only machine-applicable suggestions by default.
4. Golden tests cover suggestion rendering and fix behavior.
5. Existing diagnostics without suggestions remain unchanged.
