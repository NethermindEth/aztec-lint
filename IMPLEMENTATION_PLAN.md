# AZTEC-LINT Implementation Plan (Phased, Executable)

Date: 2026-02-19
Inputs: `SPEC.md`, `RESEARCH.md`

## 1. Plan Objectives

This plan implements the application defined in `SPEC.md` and refined by `RESEARCH.md`.

Constraints pulled from spec and research:
- Deterministic, CI-friendly, machine-readable output (`SPEC.md:29`, `SPEC.md:31`, `SPEC.md:33`, `SPEC.md:348`)
- Rust + Noir compiler crates (not syntax-only parsing) (`SPEC.md:70`, `SPEC.md:71`, `RESEARCH.md:87`)
- Layered architecture: Noir frontend -> generic model -> Aztec model -> rule engine (`SPEC.md:61`)
- Rule packs, config, CLI, SARIF, suppressions (`SPEC.md:157`, `SPEC.md:261`, `SPEC.md:309`, `SPEC.md:366`, `SPEC.md:423`)
- Risk items to resolve explicitly before rule coding (`RESEARCH.md:114`)

## 2. Non-Negotiable Sequencing Rules

- Every phase depends on previous phase artifacts; no parallel implementation across phases.
- No rule implementation before parser/model contracts are frozen.
- No SARIF output until diagnostic schema is frozen.
- No `fix` edits until safe-edit policy is defined and validated.

## 3. Repository Target Layout

Final target layout (from `SPEC.md:370`) with additional supporting files:

```text
Cargo.toml
rust-toolchain.toml
clippy.toml
crates/
  aztec-lint-cli/
  aztec-lint-core/
  aztec-lint-rules/
  aztec-lint-aztec/
  aztec-lint-sdk/              # phase 8+ skeleton
fixtures/
  noir_core/
  aztec/
docs/
  architecture.md
  rule-authoring.md
  suppression.md
```

## 4. Phase 0 - Decision Freeze and Contracts **COMPLETED**

### Objective
Resolve ambiguity before coding to prevent rework and false positives.

### Files
- Create `docs/decisions/0001-aztec010-scope.md`
- Create `docs/decisions/0002-suppression-semantics.md`
- Create `docs/decisions/0003-confidence-model.md`
- Create `docs/decisions/0004-fix-safety-policy.md`

### Steps
1. Decide AZTEC010 scope:
- Enforce `#[only_self]` for same-contract private->public bridges only.
- Do not flag valid cross-contract enqueue (`RESEARCH.md:116`, `RESEARCH.md:118`).

2. Define suppression contract:
- Primary syntax: `#[allow(AZTEC010)]`, `#[allow(noir_core::NOIR100)]` (`SPEC.md:425`, `SPEC.md:431`).
- Attachment scope: function/item only (`SPEC.md:437`).
- Output must expose suppression metadata (`SPEC.md:440`).

3. Define confidence rubric:
- Deterministic mapping rule-by-rule: `high|medium|low`.
- `--min-confidence` filtering happens after rule execution, before formatting.

4. Define safe-fix policy:
- v0 fixes allowed only when span-local and semantics-preserving.
- No cross-file refactors or speculative transformations.

### Validation
- Manual review checklist in each ADR with final status `Accepted`.
- Team sign-off gate: no phase 1 work until all four ADRs accepted.

### Failure Modes / Mitigation
- Failure: ambiguous AZTEC010 interpretation produces noisy results.
- Mitigation: hard gate on ADR acceptance.

### Exit Criteria
- All 4 ADR files merged and referenced from `docs/architecture.md`.

## 5. Phase 1 - Workspace and Build Baseline

### Objective
Stand up deterministic Rust workspace and crate boundaries.

### Files
- `Cargo.toml`
- `rust-toolchain.toml`
- `clippy.toml`
- `crates/aztec-lint-cli/Cargo.toml`
- `crates/aztec-lint-cli/src/main.rs`
- `crates/aztec-lint-core/Cargo.toml`
- `crates/aztec-lint-core/src/lib.rs`
- `crates/aztec-lint-rules/Cargo.toml`
- `crates/aztec-lint-rules/src/lib.rs`
- `crates/aztec-lint-aztec/Cargo.toml`
- `crates/aztec-lint-aztec/src/lib.rs`

### Steps
1. Initialize workspace members matching `SPEC.md:373`.
2. Pin toolchain in `rust-toolchain.toml`.
3. Add `--locked` CI commands in workspace README or Makefile.
4. Add strict lint settings:
- deny warnings in CI
- forbid `unsafe`

### Validation
- `cargo check --workspace --locked`
- `cargo test --workspace --locked`
- `cargo clippy --workspace --all-targets -- -D warnings`

Expected:
- all commands pass
- no network access required at runtime

### Failure Modes / Mitigation
- Failure: dependency drift from unpinned crate versions.
- Mitigation: lockfile committed + CI `--locked`.

### Exit Criteria
- Clean baseline CI pass with zero tests failing.

## 6. Phase 2 - Core Domain Model + Diagnostics Contract

### Objective
Define stable core data types before implementing analyzers or rules.

### Files
- `crates/aztec-lint-core/src/model/mod.rs`
- `crates/aztec-lint-core/src/model/span.rs`
- `crates/aztec-lint-core/src/model/project.rs`
- `crates/aztec-lint-core/src/model/aztec.rs`
- `crates/aztec-lint-core/src/diagnostics/mod.rs`
- `crates/aztec-lint-core/src/diagnostics/types.rs`
- `crates/aztec-lint-core/src/diagnostics/fingerprint.rs`
- `crates/aztec-lint-core/src/policy/mod.rs`

### Required structs (minimum)

```rust
pub struct Span { pub file: String, pub start: u32, pub end: u32, pub line: u32, pub col: u32 }
pub struct ProjectModel { /* ast ids, symbols, type refs, call graph, module graph */ }
pub struct AztecModel { /* contracts, entrypoints, notes, nullifiers, sinks, enqueue sites */ }

pub enum Severity { Warning, Error }
pub enum Confidence { Low, Medium, High }

pub struct Diagnostic {
  pub rule_id: String,
  pub severity: Severity,
  pub confidence: Confidence,
  pub policy: String,
  pub message: String,
  pub primary_span: Span,
  pub secondary_spans: Vec<Span>,
  pub suggestions: Vec<String>,
  pub fixes: Vec<Fix>,
  pub suppressed: bool,
  pub suppression_reason: Option<String>,
}
```

### Steps
1. Implement model modules and serialization derives.
2. Define deterministic sort key for diagnostics:
- `(file, start, end, rule_id, message_hash)`.
3. Implement stable fingerprint utility based on normalized span + rule id.

### Validation
- Unit tests for sort determinism.
- Unit tests for fingerprint stability across process restarts.
- Snapshot tests for JSON serialization shape against `SPEC.md:352` schema.

### Failure Modes / Mitigation
- Failure: schema churn later breaks formatters and SARIF.
- Mitigation: freeze struct names/fields at phase exit.

### Exit Criteria
- Public core API marked stable in docs.

## 7. Phase 3 - Config System + CLI Surface

### Objective
Implement command/flag/config behavior independent of actual rule logic.

### Files
- `crates/aztec-lint-core/src/config/mod.rs`
- `crates/aztec-lint-core/src/config/types.rs`
- `crates/aztec-lint-core/src/config/loader.rs`
- `crates/aztec-lint-cli/src/cli.rs`
- `crates/aztec-lint-cli/src/commands/check.rs`
- `crates/aztec-lint-cli/src/commands/fix.rs`
- `crates/aztec-lint-cli/src/commands/rules.rs`
- `crates/aztec-lint-cli/src/commands/explain.rs`
- `crates/aztec-lint-cli/src/commands/aztec_scan.rs`

### Steps
1. Parse config files `aztec-lint.toml` and fallback `noir-lint.toml` (`SPEC.md:265`, `SPEC.md:272`).
2. Implement profile inheritance (`profile.aztec extends default`) (`SPEC.md:283`).
3. Implement CLI per `SPEC.md:313` and flags per `SPEC.md:325`.
4. Implement override precedence:
- CLI `--deny/--warn/--allow`
- profile ruleset defaults

### Validation
- CLI golden tests:
- `aztec-lint rules`
- `aztec-lint explain AZTEC001`
- invalid flag combinations return exit code 2
- Config parsing tests for sample config from `SPEC.md:279`.

### Failure Modes / Mitigation
- Failure: profile inheritance cycles.
- Mitigation: detect and error with cycle path.

### Exit Criteria
- CLI fully parses all spec-defined commands/flags.

## 8. Phase 4 - Noir Frontend Integration (Generic Model Builder)

### Objective
Build `ProjectModel` from Noir compiler crates with accurate symbols/types/spans.

### Files
- `crates/aztec-lint-core/src/noir/mod.rs`
- `crates/aztec-lint-core/src/noir/driver.rs`
- `crates/aztec-lint-core/src/noir/project_builder.rs`
- `crates/aztec-lint-core/src/noir/span_mapper.rs`
- `crates/aztec-lint-core/src/noir/call_graph.rs`

### Steps
1. Add Noir crate dependencies pinned to exact revision from ADR.
2. Use `noirc_driver` entrypoints to check/resolve crates (`RESEARCH.md:90`).
3. Extract and map:
- AST handles
- symbols
- type metadata
- module graph
- best-effort call graph (`SPEC.md:91`-`SPEC.md:96`)
4. Normalize spans into linter `Span` format.

### Validation
- Fixture compilation tests with minimal Noir package.
- Regression tests for span mapping (line/column correctness).
- Determinism test: two runs produce byte-identical serialized `ProjectModel`.

### Failure Modes / Mitigation
- Failure: upstream Noir API changes.
- Mitigation: adapter layer in `noir/driver.rs`; pin revision; compile-time feature guards.

### Exit Criteria
- `ProjectModel` populated from real Noir code in tests.

## 9. Phase 5 - Rule Engine + `noir_core` Pack

### Objective
Implement generic deterministic rule runner and all 8 `noir_core` rules from phase 0 spec (`SPEC.md:446`).

### Files
- `crates/aztec-lint-rules/src/engine/mod.rs`
- `crates/aztec-lint-rules/src/engine/context.rs`
- `crates/aztec-lint-rules/src/engine/registry.rs`
- `crates/aztec-lint-rules/src/noir_core/noir001_unused.rs`
- `crates/aztec-lint-rules/src/noir_core/noir002_shadowing.rs`
- `crates/aztec-lint-rules/src/noir_core/noir010_bool_not_asserted.rs`
- `crates/aztec-lint-rules/src/noir_core/noir020_bounds.rs`
- `crates/aztec-lint-rules/src/noir_core/noir030_unconstrained_influence.rs`
- `crates/aztec-lint-rules/src/noir_core/noir100_magic_numbers.rs`
- `crates/aztec-lint-rules/src/noir_core/noir110_complexity.rs`
- `crates/aztec-lint-rules/src/noir_core/noir120_nesting.rs`
- `fixtures/noir_core/*`

### Steps
1. Create `Rule` trait:

```rust
pub trait Rule {
  fn id(&self) -> &'static str;
  fn run(&self, ctx: &RuleContext, out: &mut Vec<Diagnostic>);
}
```

2. Build registry by pack and default enablement policy.
3. Implement severity defaults per spec tables (`SPEC.md:163`, `SPEC.md:175`, `SPEC.md:185`).
4. Add suppression lookup in engine (not per-rule).
5. Add confidence assignment per ADR.

### Validation
- Unit tests per rule with positive and negative fixture pairs.
- Suppression tests for `#[allow(RULE_ID)]` and scoped form.
- Command tests:
- `aztec-lint check fixtures/noir_core` returns exit code 1 when errors exist.
- `--severity-threshold warning|error` behavior verified.

### Failure Modes / Mitigation
- Failure: high false positives for default-on correctness rules.
- Mitigation: require fixture demonstrating true positive + false-positive guard before enabling by default.

### Exit Criteria
- 8 `noir_core` rules implemented and passing fixture suite.

## 10. Phase 6 - Aztec Semantic Augmentation + Initial Aztec Rules

### Objective
Implement Aztec model builder and phase-1 Aztec rules (`AZTEC001`, `AZTEC010`, `AZTEC020`) per `SPEC.md:452`.

### Files
- `crates/aztec-lint-aztec/src/detect.rs`
- `crates/aztec-lint-aztec/src/model_builder.rs`
- `crates/aztec-lint-aztec/src/patterns.rs`
- `crates/aztec-lint-rules/src/aztec/aztec001_privacy_leak.rs`
- `crates/aztec-lint-rules/src/aztec/aztec010_only_self_enqueue.rs`
- `crates/aztec-lint-rules/src/aztec/aztec020_unconstrained_influence.rs`
- `fixtures/aztec/*`

### Steps
1. Implement Aztec activation triggers (`SPEC.md:102`-`SPEC.md:106`).
2. Build `AztecModel` fields from `SPEC.md:111`-`SPEC.md:120`.
3. Recognize patterns validated in research:
- `self.enqueue(...)`
- `self.enqueue_self.*`
- `get_notes(...)` high-level and oracle-level
- `MessageDelivery.ONCHAIN_CONSTRAINED` style
4. Implement AZTEC010 using decision from phase 0.

### Validation
- Fixture tests for each Aztec rule including suppression cases.
- Differential tests against known Aztec contract snippets.
- Ensure `--profile default` excludes aztec pack; `--profile aztec` includes both packs.

### Failure Modes / Mitigation
- Failure: missing real-world Aztec patterns yields under-reporting.
- Mitigation: keep patterns configurable via `[aztec]` keys (`SPEC.md:287` onward).

### Exit Criteria
- Three phase-1 Aztec rules stable on fixtures and deterministic.

## 11. Phase 7 - Taint Engine v1 + Additional Aztec Coverage

### Objective
Implement taint propagation required by `SPEC.md:238` and enable remaining default-on privacy/soundness checks incrementally.

### Files
- `crates/aztec-lint-aztec/src/taint/mod.rs`
- `crates/aztec-lint-aztec/src/taint/graph.rs`
- `crates/aztec-lint-aztec/src/taint/propagate.rs`
- `crates/aztec-lint-rules/src/aztec/aztec002_secret_branching.rs`
- `crates/aztec-lint-rules/src/aztec/aztec003_private_debug_log.rs`
- `crates/aztec-lint-rules/src/aztec/aztec021_range_before_hash.rs`
- `crates/aztec-lint-rules/src/aztec/aztec022_merkle_witness.rs`

### Steps
1. Encode taint sources/sinks from `SPEC.md:240`-`SPEC.md:252`.
2. Implement intra-procedural def-use propagation first (`SPEC.md:256`).
3. Wire taint facts into affected rules.
4. Keep inter-procedural extension hooks but not enabled by default.

### Validation
- Targeted taint fixture suite:
- source-only (no sink) -> no issue
- source->sink flow -> issue
- sanitized/guarded flow -> no issue
- Performance check: run time on fixture corpus under threshold.

### Failure Modes / Mitigation
- Failure: path explosion or unstable ordering.
- Mitigation: bounded graph traversal + deterministic node ordering.

### Exit Criteria
- Intra-procedural taint engine merged with stable results.

## 12. Phase 8 - JSON/SARIF Output, Exit Codes, and CI Behavior

### Objective
Finish machine interfaces and CI ergonomics.

### Files
- `crates/aztec-lint-core/src/output/text.rs`
- `crates/aztec-lint-core/src/output/json.rs`
- `crates/aztec-lint-core/src/output/sarif.rs`
- `crates/aztec-lint-cli/src/exit_codes.rs`
- `fixtures/sarif/*`

### Steps
1. Implement JSON output matching `SPEC.md:352` schema.
2. Implement SARIF 2.1.0 emitter (`SPEC.md:366`, `RESEARCH.md:109`).
3. Add stable `partialFingerprints` and relative file URI normalization (`RESEARCH.md:111`).
4. Implement exit code behavior (`SPEC.md:340`).

### Validation
- JSON schema tests (exact field names/types).
- SARIF validator check in CI.
- Golden SARIF snapshots for deterministic output.
- Integration tests for threshold/exit code matrix.

### Failure Modes / Mitigation
- Failure: non-stable SARIF identifiers cause alert churn.
- Mitigation: normalized fingerprint algorithm with tests across reorderings.

### Exit Criteria
- `--format json|sarif` production-ready for CI pipelines.

## 13. Phase 9 - `fix`, `changed-only`, and Operational Hardening

### Objective
Complete remaining CLI behavior and production hardening.

### Files
- `crates/aztec-lint-core/src/fix/mod.rs`
- `crates/aztec-lint-core/src/fix/apply.rs`
- `crates/aztec-lint-core/src/vcs/changed_only.rs`
- `crates/aztec-lint-cli/src/commands/fix.rs`
- `docs/suppression.md`
- `docs/rule-authoring.md`

### Steps
1. Implement safe-fix executor with dry-run mode.
2. Implement `--changed-only` using git diff file set.
3. Expose suppression state in diagnostics as required (`SPEC.md:440`).
4. Add comprehensive docs and troubleshooting.

### Validation
- Fix idempotence tests (apply twice no further changes).
- Changed-only tests across staged/unstaged scenarios.
- End-to-end test matrix in CI.

### Failure Modes / Mitigation
- Failure: unsafe edits mutate semantics.
- Mitigation: restrict fixes to vetted rules only; per-fix safety classification.

### Exit Criteria
- `check`, `fix`, `rules`, `explain`, `aztec scan` all functional end-to-end.

## 14. Phase 10 - Extensibility Skeleton (Plugin-Ready)

### Objective
Prepare non-breaking path to WASM plugin system (`SPEC.md:484`) without shipping full plugin runtime yet.

### Files
- `crates/aztec-lint-sdk/Cargo.toml`
- `crates/aztec-lint-sdk/src/lib.rs`
- `crates/aztec-lint-core/src/plugin/api.rs`
- `docs/plugin-api-v0.md`

### Steps
1. Define versioned rule API traits and data contracts.
2. Add plugin loading interface behind feature flag.
3. Add sandbox policy placeholders.

### Validation
- Compile-time integration test with mock plugin crate.
- Version-compatibility contract tests.

### Failure Modes / Mitigation
- Failure: core APIs leak unstable internals.
- Mitigation: SDK types decoupled from internal compiler adapter structs.

### Exit Criteria
- Plugin API documented and versioned, runtime still optional.

## 15. Cross-Phase Validation Matrix

Run after each phase and in CI:
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --locked`
- `cargo test -p aztec-lint-cli --locked`
- `cargo run -p aztec-lint-cli -- check fixtures`

Additional gates:
- Determinism test: same input repo -> byte-identical JSON/SARIF outputs.
- No-network runtime test (except compile/download time).
- Performance budget smoke test on medium fixture corpus.

## 16. Failure Visibility Checklist (must be obvious)

Every phase must include:
- explicit command(s) that fail if phase is incomplete
- fixture(s) proving true positive and false-positive boundaries
- regression test linked to any bug fixed in phase
- documented rollback strategy (disable new rule or feature flag)

## 17. Milestone Mapping to SPEC Phases

- `SPEC Phase 0` (`SPEC.md:446`) maps to this plan phases 1-5.
- `SPEC Phase 1` (`SPEC.md:452`) maps to this plan phase 6.
- `SPEC Phase 2` (`SPEC.md:459`) maps to this plan phases 7-8.
- `SPEC Phase 3` (`SPEC.md:465`) maps to this plan phase 10.

## 18. Immediate Next Action

Start with Phase 0 decision ADRs, then execute Phase 1 workspace scaffold in one PR.
