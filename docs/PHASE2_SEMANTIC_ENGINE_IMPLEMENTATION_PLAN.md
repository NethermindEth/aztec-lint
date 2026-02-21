# Phase 2 Semantic Engine Upgrade Implementation Plan

Date: 2026-02-21  
Source: `docs/CLIPPY_GAP_ANALYSIS.md` (Phase 2 + Gap #3 "What to do")

## Phase 2 Requirements (Extracted)

1. Add a typed query API in `RuleContext` backed by the Noir semantic model.
2. Migrate current `noir_core` rules away from line-based heuristics.
3. Rebuild Aztec taint/soundness using AST/HIR + CFG/DFG dataflow, not substring sink detection.
4. Keep text heuristics only as fallback, never primary for correctness/soundness rules.

## Current Baseline (Concrete Evidence)

`RuleContext` currently exposes raw source/project accessors; no typed query surface:

```rust
// crates/aztec-lint-rules/src/engine/context.rs
pub fn project(&self) -> &ProjectModel { ... }
pub fn files(&self) -> &[SourceFile] { ... }
```

`NOIR020` is line and substring driven:

```rust
// crates/aztec-lint-rules/src/noir_core/noir020_bounds.rs
for line in file.text().lines() { ... }
if line.contains("assert(") && line.contains("len()") ...
```

Aztec taint sink detection is substring based:

```rust
// crates/aztec-lint-aztec/src/taint/graph.rs
if line.contains("emit(") || line.contains("return ") { ... }
if line.contains("hash(") || line.contains("serialize(") { ... }
```

Aztec semantic model building is also line/pattern based:

```rust
// crates/aztec-lint-aztec/src/model_builder.rs
if contains_public_sink(line) { ... }
if looks_like_enqueue(line, config) { ... }
```

`ProjectModel` lacks CFG/DFG/HIR facts today:

```rust
// crates/aztec-lint-core/src/model/project.rs
pub struct ProjectModel {
    pub ast_ids: Vec<String>,
    pub symbols: Vec<SymbolRef>,
    pub type_refs: Vec<TypeRef>,
    pub call_graph: Vec<CallEdge>,
    pub module_graph: Vec<ModuleEdge>,
}
```

Positive reuse point: HIR traversal already exists in `call_graph.rs` and can be generalized.

## Implementation Strategy

Implement in small, behavior-locked slices:

1. Lock baseline diagnostics with fixture parity tests.
2. Build semantic substrate in `aztec-lint-core`.
3. Expose typed queries via `RuleContext`.
4. Migrate `noir_core` rules in batches.
5. Replace Aztec taint pipeline with typed CFG/DFG flow.
6. Remove heuristic-primary paths and enforce with tests/CI checks.

## Detailed Steps

### Step 1: Freeze Baseline Behavior Before Refactors **COMPLETED**

Files:
- `crates/aztec-lint-rules/tests/noir_core_rules.rs`
- `crates/aztec-lint-rules/tests/aztec_foundation_rules.rs`
- `crates/aztec-lint-rules/tests/aztec_advanced_rules.rs`
- `fixtures/noir_core/rule_cases/*.nr`
- `fixtures/aztec/rule_cases/*.nr`

Change:
- Add missing edge-case fixtures (alias imports, nested scopes, range guards, branch + public-effect coupling, hash-with-guard ordering).
- Ensure every Phase 2-impacted rule has positive + negative + suppression (where supported).

Validation:
- `cargo test -p aztec-lint-rules noir_core_rules`
- `cargo test -p aztec-lint-rules aztec_foundation_rules`
- `cargo test -p aztec-lint-rules aztec_advanced_rules`

Failure modes:
- If fixtures are too weak, refactor regressions will ship unnoticed.
- If fixture expectations are unstable, stop and stabilize diagnostics ordering/spans first.

---

### Step 2: Introduce a First-Class Semantic Model in Core **COMPLETED**

Files:
- `crates/aztec-lint-core/src/model/mod.rs`
- `crates/aztec-lint-core/src/model/semantic.rs` (new)
- `crates/aztec-lint-core/src/model/project.rs`
- `docs/architecture.md`

Change:
- Add semantic data types extracted from Noir HIR:
  - typed function inventory
  - expression nodes with type/category
  - statement nodes
  - CFG blocks/edges
  - DFG def-use edges
  - call sites (resolved callee symbol ids)
  - guard/assert/constrain nodes
- Keep deterministic sort/dedup contracts identical to existing model code.

Validation:
- `cargo test -p aztec-lint-core`
- Add deterministic serialization/equality tests for semantic structures.

Failure modes:
- Memory blow-up from over-capturing raw compiler internals.
- Nondeterministic ordering from hash-iteration; enforce explicit sorting in builders.

---

### Step 3: Build Semantic Extractor From Noir Compiler Output **COMPLETED**

Files:
- `crates/aztec-lint-core/src/noir/project_builder.rs`
- `crates/aztec-lint-core/src/noir/call_graph.rs`
- `crates/aztec-lint-core/src/noir/semantic_builder.rs` (new)
- `crates/aztec-lint-core/src/noir/mod.rs`

Change:
- Add a semantic extraction pass using `NoirCheckedProject.context()` and def interner.
- Generalize existing HIR walking patterns from `call_graph.rs` to populate CFG/DFG/type facts.
- Introduce a bundle return path, e.g. `build_project_semantic_bundle(...)`, keeping `build_project_model(...)` for compatibility.

Validation:
- `cargo test -p aztec-lint-core --features noir-compiler`
- New fixture test proving semantic nodes exist for `fixtures/noir_core/minimal/src/main.nr`.

Failure modes:
- HIR node ids not stable across compiler versions.
- Feature-gating breakage; run `cargo test -p aztec-lint-core --no-default-features`.

---

### Step 4: Add Typed Query API to `RuleContext` **COMPLETED**

Files:
- `crates/aztec-lint-rules/src/engine/context.rs`
- `crates/aztec-lint-rules/src/engine/query.rs` (new)
- `crates/aztec-lint-rules/src/engine/mod.rs`

Change:
- Add semantic bundle storage in context (`set_semantic_model(...)`).
- Add typed query entrypoints, for example:
  - `ctx.query().functions()`
  - `ctx.query().locals_in_function(...)`
  - `ctx.query().index_accesses(...)`
  - `ctx.query().assertions(...)`
  - `ctx.query().cfg(...)`
  - `ctx.query().dfg(...)`
- Keep `files()` available for fallback-only rules.

Validation:
- New unit tests in `context.rs` for query availability and deterministic query ordering.
- `cargo test -p aztec-lint-rules engine::context`

Failure modes:
- Query API too narrow, forcing rules back to string matching.
- Query API leaking compiler internals instead of stable lint-facing abstractions.

---

### Step 5: Wire Semantic Model Through CLI Check Pipeline **COMPLETED**

Files:
- `crates/aztec-lint-cli/src/commands/check.rs`
- `crates/aztec-lint-rules/src/engine/context.rs`

Change:
- Replace single `build_project_model(...)` call with semantic bundle construction.
- Populate `RuleContext` with semantic model before rule execution.
- Preserve existing report-root path rebasing behavior.

Validation:
- `cargo test -p aztec-lint-cli cli_golden`
- `cargo test -p aztec-lint-cli`

Failure modes:
- Large projects regress in runtime from rebuilding semantic data per rule.
- Path rebasing bugs if semantic spans are absolute while diagnostics expect rebased relative.

---

### Step 6: Migrate `noir_core` Correctness Rules (Batch 1) **COMPLETED**

Files:
- `crates/aztec-lint-rules/src/noir_core/noir001_unused.rs`
- `crates/aztec-lint-rules/src/noir_core/noir002_shadowing.rs`
- `crates/aztec-lint-rules/src/noir_core/util.rs` (de-scope/reduce)

Change:
- `NOIR001`: compute unused locals/imports from symbol defs + references in semantic graph.
- `NOIR002`: use lexical scope tree from AST/HIR instead of brace-depth parser.

Validation:
- `cargo test -p aztec-lint-rules noir001_fixture_pair`
- `cargo test -p aztec-lint-rules noir002_fixture_pair`
- `cargo test -p aztec-lint-rules noir_core_rules`

Failure modes:
- False positives on pattern bindings/destructuring/import aliases.
- Span drift causing bad autofix placement for `NOIR001`.

---

### Step 7: Migrate `noir_core` Correctness Rules (Batch 2) **COMPLETED**

Files:
- `crates/aztec-lint-rules/src/noir_core/noir010_bool_not_asserted.rs`
- `crates/aztec-lint-rules/src/noir_core/noir020_bounds.rs`
- `crates/aztec-lint-rules/src/noir_core/noir030_unconstrained_influence.rs`

Change:
- `NOIR010`: detect boolean-typed bindings from HIR types and assertion consumption by use-def links.
- `NOIR020`: identify index expressions + guard dominance in CFG (assert/constrain before sink path).
- `NOIR030`: taint unconstrained-return values across intra-procedural DFG into constrain/assert sinks.

Validation:
- `cargo test -p aztec-lint-rules noir010_fixture_pair`
- `cargo test -p aztec-lint-rules noir020_fixture_pair`
- `cargo test -p aztec-lint-rules noir030_fixture_pair`

Failure modes:
- Guard logic over-approximates and suppresses real findings.
- Under-approximation misses transitive influence through temporary variables.

---

### Step 8: Migrate `noir_core` Maintainability Rules **COMPLETED**

Files:
- `crates/aztec-lint-rules/src/noir_core/noir100_magic_numbers.rs`
- `crates/aztec-lint-rules/src/noir_core/noir110_complexity.rs`
- `crates/aztec-lint-rules/src/noir_core/noir120_nesting.rs`

Change:
- `NOIR100`: inspect literal nodes and exclude constant declarations using AST context.
- `NOIR110`: compute complexity from CFG decision nodes.
- `NOIR120`: compute nesting from block/branch tree, not brace counting.

Validation:
- `cargo test -p aztec-lint-rules noir100_fixture_pair`
- `cargo test -p aztec-lint-rules noir110_fixture_pair`
- `cargo test -p aztec-lint-rules noir120_fixture_pair`

Failure modes:
- Behavior churn on expected maintainability thresholds; keep thresholds constant in this phase.
- Suggestions from `NOIR100` lose applicability fidelity if spans become expression-wide.

---

### Step 9: Replace Aztec Semantic Site Extraction With Typed Analysis

Files:
- `crates/aztec-lint-aztec/src/model_builder.rs`
- `crates/aztec-lint-aztec/src/patterns.rs`
- `crates/aztec-lint-core/src/model/aztec.rs`

Change:
- Build contracts/entrypoints/storage/sinks from semantic nodes (attributes, resolved calls, typed operations).
- Remove primary reliance on `contains_*` and `looks_like_*` substring helpers.
- Keep string helpers only as fallback when semantic data is unavailable.

Validation:
- `cargo test -p aztec-lint-aztec`
- `cargo test -p aztec-lint-rules aztec_foundation_rules`

Failure modes:
- Attribute parsing mismatch (`#[external("private")]`, custom-config names).
- Missed sink detection for method-call variants not covered in resolver.

---

### Step 10: Rebuild Aztec Taint Engine on CFG/DFG

Files:
- `crates/aztec-lint-aztec/src/taint/graph.rs`
- `crates/aztec-lint-aztec/src/taint/propagate.rs`
- `crates/aztec-lint-aztec/src/taint/mod.rs`
- `crates/aztec-lint-core/src/model/semantic.rs`

Change:
- Replace line records and substring sink/source detection with typed node ids and dataflow edges.
- Compute sources from semantic facts (private params, note reads, secret state reads, unconstrained-call returns).
- Compute sinks from resolved op/call categories (public output, storage writes, nullifier/commitment, hash/serialize, branch condition, merkle witness).
- Implement guard-before-sink reasoning via CFG dominance rather than line offsets.

Validation:
- `cargo test -p aztec-lint-aztec taint::graph`
- `cargo test -p aztec-lint-aztec taint::propagate`
- Add performance smoke test equivalent to current 200-assignment chain.

Failure modes:
- Graph explosion from per-expression node granularity.
- Dominance bugs causing AZTEC021 false negatives/positives.

---

### Step 11: Migrate Aztec Rules to Typed Taint Outputs

Files:
- `crates/aztec-lint-rules/src/aztec/aztec001_privacy_leak.rs`
- `crates/aztec-lint-rules/src/aztec/aztec002_secret_branching.rs`
- `crates/aztec-lint-rules/src/aztec/aztec003_private_debug_log.rs`
- `crates/aztec-lint-rules/src/aztec/aztec020_unconstrained_influence.rs`
- `crates/aztec-lint-rules/src/aztec/aztec021_range_before_hash.rs`
- `crates/aztec-lint-rules/src/aztec/aztec022_merkle_witness.rs`

Change:
- Keep rule IDs/messages/policies stable.
- Switch source/sink/filter conditions to typed taint facts only.
- Remove dependence on `SinkSite.line` textual checks for verification heuristics; use semantic call/assert facts.

Validation:
- `cargo test -p aztec-lint-rules aztec_foundation_rules`
- `cargo test -p aztec-lint-rules aztec_advanced_rules`
- `cargo test -p aztec-lint-cli cli_golden`

Failure modes:
- Rule behavior drift from existing fixture contract.
- Lost suppression behavior if spans move to parent nodes; verify suppression fixtures remain green.

---

### Step 12: Enforce "No Heuristic-Primary" Policy and Clean Up

Files:
- `crates/aztec-lint-rules/src/noir_core/util.rs`
- `crates/aztec-lint-aztec/src/patterns.rs`
- `docs/rule-authoring.md`
- `docs/architecture.md`

Change:
- Remove or isolate line-parser helpers so correctness/soundness rules cannot use them as primary signal.
- Update authoring guide with required typed-query usage for correctness/soundness.
- Keep fallback hooks explicit and gated.

Validation:
- `rg --line-number 'contains\\(\"|\\.lines\\(\\)|find_let_bindings|find_function_scopes' crates/aztec-lint-rules/src/noir_core crates/aztec-lint-rules/src/aztec`
- `cargo test --workspace`

Failure modes:
- Accidental deletion of fallback paths needed for `--no-default-features`.
- Documentation drift from implemented contracts.

## Definition of Done (Phase 2)

1. `RuleContext` has stable typed query API backed by Noir semantic data.
2. All `noir_core` rules (`NOIR001/2/10/20/30/100/110/120`) run without line-based primary heuristics.
3. Aztec taint and soundness/privacy rules are CFG/DFG-driven.
4. Existing fixture suites pass, and added Phase 2 edge-case fixtures pass.
5. Workspace tests pass with and without default features where applicable.

## Rollout Guardrails

1. Merge in step order; do not batch Steps 6-11 in one PR.
2. Require green targeted tests for each step before continuing.
3. If correctness/soundness false-positive rate increases on existing fixtures, stop and fix before next migration step.
4. Preserve diagnostic ordering and deterministic outputs at every step.
