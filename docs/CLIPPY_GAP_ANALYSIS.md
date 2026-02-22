# aztec-lint vs rust-clippy: Gap Analysis

Date: 2026-02-20
Compared repos:
- `aztec-lint`: `/home/ametel/source/aztec-lint`
- `rust-clippy`: `/home/ametel/source/rust-clippy`

## Executive summary
`aztec-lint` already has a solid baseline (workspace split, deterministic sorting, SARIF/JSON/text output, suppression, fix pipeline, Noir frontend adapter), but it is still far from Clippy-equivalent in scale, semantic depth, lint metadata lifecycle, diagnostic sophistication, and test rigor.

If the goal is “logically equivalent to Clippy,” the biggest work is not adding a few more rules; it is building a full lint platform: compiler-integrated semantic analysis, rich lint metadata + groups + lifecycle management, robust diagnostics/fixes, and a large UI/regression suite.

## Snapshot metrics

### Lint inventory and scale
- Clippy declared lints: **804** (`clippy_lints/src/declared_lints.rs`, counted from `_INFO` entries)
- aztec-lint implemented rules in engine registry: **15** (`crates/aztec-lint-rules/src/engine/registry.rs:37`)
- aztec-lint catalog entries exposed to users: **20** (`crates/aztec-lint-cli/src/commands/catalog.rs:10`)

### Testing scale
- Clippy UI tests: **1185** Rust files in `tests/ui`
- Clippy config-focused tests: **122** Rust files in `tests/ui-toml`
- Clippy cargo-project tests: **45** manifests in `tests/ui-cargo`
- aztec-lint test files under `crates/*/tests`: **6**
- aztec-lint Noir fixtures (`*.nr`): **39**

### Config surface
- Clippy config keys: **95** (`clippy_config/src/conf.rs`, `define_Conf!` block)
- aztec-lint config is small and rule-pack based (`crates/aztec-lint-core/src/config/types.rs:8`)

---

## Gap list and required work

## 1) Lint coverage scale gap
### Gap
`aztec-lint` has 15 implemented rules, while Clippy has 800+ and broad category coverage.

### Evidence
- aztec registry only includes `NOIR001/2/10/20/30/100/110/120` + `AZTEC001/2/3/10/20/21/22` (`crates/aztec-lint-rules/src/engine/registry.rs:37`)
- Clippy tracks 800+ lints (`README.md:7`)

### What to do
- Create a multi-release rule roadmap by category (correctness, suspicious, style, complexity, perf, security/privacy/soundness for Noir/Aztec).
- Target >100 rules before claiming Clippy-like utility.
- Add per-rule maturity states similar to nursery/pedantic/restriction semantics.

## 2) Implemented-rules vs exposed-rules inconsistency
### Gap
User-facing catalog/config include rules that are not implemented in the runtime registry.

### Evidence
- Catalog/config include `AZTEC011`, `AZTEC012`, `AZTEC040`, `AZTEC041`, `NOIR200` (`crates/aztec-lint-cli/src/commands/catalog.rs:40`, `crates/aztec-lint-core/src/config/types.rs:17`)
- Engine registry does not register those IDs (`crates/aztec-lint-rules/src/engine/registry.rs:37`)

### What to do
- Add startup validation: `catalog_ids == config_ids == registry_ids`.
- Fail CI if any catalog/config rule is missing from registry.
- Either implement missing rules or remove them from catalog/config until ready.

## 3) Semantic analysis depth gap (typed-HIR/MIR equivalent)
### Gap
Clippy lints are deeply integrated with typed HIR/MIR and compiler internals; many aztec-lint rules are still line/text-pattern driven.

### Evidence
- Clippy registers large sets of early and late lint passes over compiler internals (`clippy_lints/src/lib.rs:440`)
- Clippy driver configures rustc callbacks and lint store directly (`src/driver.rs:137`)
- Noir core rules in aztec-lint mostly iterate raw file lines (`crates/aztec-lint-rules/src/noir_core/noir020_bounds.rs:22`)
- Aztec model/taint uses many string heuristics (`crates/aztec-lint-aztec/src/model_builder.rs:89`, `crates/aztec-lint-aztec/src/taint/graph.rs:444`)
- Rule code does not consume `ctx.project()` semantic model (`crates/aztec-lint-rules/src/engine/context.rs:135`)

### What to do
- Build typed query APIs in `RuleContext` (symbol resolution, type predicates, CFG/DFG queries, callgraph traversal).
- Port existing rules from string matching to typed AST/HIR operations.
- Add a MIR/IR-like analysis layer for dataflow-heavy rules (influence/taint/soundness).
- Keep text heuristics only as fallback, never as primary for correctness/soundness rules.

## 4) Driver/invocation parity gap
### Gap
Clippy behaves like a first-class compiler companion (`cargo clippy`, `clippy-driver`); aztec-lint is standalone and not deeply toolchain-integrated.

### Evidence
- Clippy cargo wrapper and rustc wrapper logic (`src/main.rs:56`, `src/driver.rs:196`)
- `aztec-lint` CLI supports `check/fix/rules/explain/aztec scan` only (`crates/aztec-lint-cli/src/cli.rs:93`)

### What to do
- Ship a `nargo`/Aztec-integrated wrapper command analogous to `cargo clippy`.
- Add package/workspace targeting semantics equivalent to package-manager UX.
- Add wrapper-mode behavior for CI and editor integration (stable machine-readable diagnostics stream).

## 5) Lint grouping and lifecycle metadata gap
### Gap
Clippy has first-class lint categories/groups and lifecycle metadata (including renamed/removed lint handling and version tags). aztec-lint currently has lightweight pack/policy metadata.

### Evidence
- Clippy registers groups (`clippy::all`, `clippy::correctness`, etc.) (`declare_clippy_lint/src/lib.rs:49`)
- Clippy handles renamed/removed lints at registration (`clippy_lints/src/lib.rs:441`)
- Clippy lint info includes explanation/location/version (`declare_clippy_lint/src/lib.rs:103`)
- aztec-lint rule metadata is limited to id/pack/policy/default/confidence (`crates/aztec-lint-rules/src/engine/registry.rs:40`)

### What to do
- Introduce lint categories and group-level CLI/config control.
- Add lint lifecycle metadata: introduced version, deprecated/renamed/replaced-by.
- Add compatibility layer that warns on renamed rules and maps them automatically.

## 6) Explain/documentation parity gap
### Gap
Clippy’s `--explain` returns full lint docs + config info; aztec-lint `explain` prints only a short summary block.

### Evidence
- aztec-lint explain is summary-only (`crates/aztec-lint-cli/src/commands/explain.rs:14`)
- Clippy explain prints full explanation and matching config metadata (`clippy_lints/src/lib.rs:420`)

### What to do
- Store full per-lint docs in source-of-truth metadata.
- Extend `aztec-lint explain RULE_ID` to include: rationale, examples, false-positive boundaries, fix safety notes, related config keys.
- Generate lint reference docs website (Clippy-style index).

## 7) Config sophistication gap
### Gap
Clippy has extensive lint-tunable configuration and validation discipline; aztec-lint has basic profiles/rulesets and Aztec naming knobs.

### Evidence
- Clippy `Conf` metadata and generated config docs (`clippy_config/src/conf.rs:314`)
- Clippy enforces that every config variable has tests (`clippy_config/src/conf.rs:1239`)
- aztec-lint config focuses on profile inheritance + rule levels + Aztec naming (`crates/aztec-lint-core/src/config/types.rs:8`)

### What to do
- Add per-lint configuration schema and binding.
- Add strict unknown-config and unknown-rule diagnostics.
- Add a test contract: every config key must have positive+negative tests.
- Add MSRV/min-version-aware lint gating equivalent for Noir/Aztec compiler versions.

## 8) Unknown lint/rule behavior gap
### Gap
Clippy/rustc tooling provides strong unknown-lint behavior; aztec-lint accepts overrides without validating that rule IDs are implemented.

### Evidence
- aztec-lint override registration only checks conflicts, not existence (`crates/aztec-lint-core/src/config/types.rs:286`)
- Effective levels can include IDs missing from engine registry (`crates/aztec-lint-cli/src/commands/check.rs:90`, `crates/aztec-lint-rules/src/engine/registry.rs:37`)

### What to do
- Validate all configured/CLI rule IDs against registry at run start.
- Emit actionable errors for unknown or unimplemented rules.
- Add `--list-unimplemented` for transparent roadmap visibility.

## 9) Diagnostic ergonomics parity gap
### Gap
Clippy diagnostics use rich rustc mechanisms (multi-span, applicability discipline, docs links, precise lint-level scoping via HIR IDs). aztec-lint diagnostics are improving but still less integrated and less constrained.

### Evidence
- Clippy diagnostic wrappers and validation (`clippy_utils/src/diagnostics.rs:1`, `clippy_utils/src/diagnostics.rs:37`)
- Clippy provides HIR-id-aware emission for correct allow/expect semantics (`clippy_utils/src/diagnostics.rs:82`)
- aztec-lint suppression is custom and item-local only (`docs/suppression.md:23`)

### What to do
- Introduce richer structured diagnostics contract (primary + labeled secondary spans + machine-fix diagnostics metadata with stronger guarantees).
- Add diagnostic quality gates (no overlapping replacements, stable spans, applicability correctness).
- Expand suppression semantics to parity with module/file scopes where appropriate.

## 10) Fix application parity gap
### Gap
Clippy uses rustfix through compiler diagnostics/applicability ecosystem; aztec-lint has an internal fix engine that only applies explicitly safe edits.

### Evidence
- aztec fix engine applies only `FixSafety::Safe` and skips unsafe candidates (`crates/aztec-lint-core/src/fix/apply.rs:145`)
- Clippy fix workflow is integrated in standard command path (`README.md:93`, `src/main.rs:73`)

### What to do
- Add end-to-end applicability model and rustfix-style provenance for each suggestion.
- Support grouped multipart fixes with transaction semantics and better conflict resolution policy.
- Add “fix confidence report” and “why not fixed” diagnostics compatible with CI/editor flows.

## 11) Test rigor and regression protection gap
### Gap
Clippy has massive UI/regression, rustfix, cargo and config suites; aztec-lint test surface is currently small.

### Evidence
- Clippy compile-test harness is extensive (`tests/compile-test.rs:1`)
- aztec-lint has a small number of crate tests and fixture pairs (`crates/aztec-lint-rules/tests/noir_core_rules.rs:1`)

### What to do
- Build UI-style golden tests for every lint with:
  - true positive / false positive / false negative guard cases
  - macro/attribute edge cases
  - fix `.fixed` expectations
  - cross-version snapshot tests
- Add large-project benchmark corpus and performance regression CI.

## 12) Lint authoring toolchain gap
### Gap
Clippy has dedicated developer tooling to scaffold, register, and regenerate lint metadata/docs/tests. aztec-lint authoring is currently manual.

### Evidence
- Clippy `cargo dev new_lint` and `update_lints` workflows (`clippy_dev/src/new_lint.rs:1`, `clippy_dev/src/update_lints.rs:15`)
- aztec-lint rule metadata is hand-maintained in registry and catalog (`crates/aztec-lint-rules/src/engine/registry.rs:37`, `crates/aztec-lint-cli/src/commands/catalog.rs:10`)

### What to do
- Add `cargo xtask new-lint` and `cargo xtask update-lints` for aztec-lint.
- Generate registry/catalog/docs from a single metadata source.
- Enforce generation in CI to prevent drift.

## 13) Plugin/runtime extensibility gap (relative to Clippy-like maturity)
### Gap
Plugin API exists as draft skeleton but is not executed in `check/fix` pipeline.

### Evidence
- Plugin API doc explicitly says execution is not wired yet (`docs/plugin-api-v0.md:66`, `docs/plugin-api-v0.md:71`)

### What to do
- Decide whether external lint plugins are in-scope for Clippy equivalence target.
- If yes: wire plugin diagnostics into rule engine ordering, config, suppression, fixes, and output determinism.
- If no: remove from near-term critical path and focus on core lint platform parity first.

---

## Priority implementation plan to reach Clippy-like parity

## Phase 1: Integrity and metadata foundation (short-term) **COMPLETED**
- Unify rule source of truth (registry/catalog/config generation).
- Add unknown-rule validation and fail-fast behavior.
- Add lint metadata model: category, introduced version, lifecycle state, docs content.
- Expand `explain` to full lint docs.

## Phase 2: Semantic engine upgrade (medium-term) **COMPLETED**
- Add typed query API in `RuleContext` backed by Noir semantic model.
- Migrate current `noir_core` rules away from line-based heuristics.
- Rebuild Aztec taint/soundness on AST/HIR + CFG/DFG dataflow instead of substring sink detection.

## Phase 3: Diagnostics and fixes (medium-term) **COMPLETED**
- Introduce strict diagnostic/fix invariants and validation tests.
- Add richer machine-applicable suggestion model and grouped edits.
- Improve suppression scoping + lint-level semantics.

## Phase 4: Scale and quality bar (long-term)
- Expand rule count aggressively by category and maturity tier.
- Build Clippy-style UI/regression/fix/corpus test matrix.
- Add benchmark and performance gates.
- Add lint-authoring automation (`xtask`) and generated docs portal.

## Phase 5: Toolchain integration (long-term)
- Provide package-manager-native UX analogous to `cargo clippy`.
- Improve CI/editor integration contracts and stable machine output.

---

## Bottom line
To become Clippy-equivalent, `aztec-lint` needs to evolve from “good linter implementation” into a “lint platform”: larger rule corpus, compiler-semantic-driven analyses, hardened diagnostics/fixes, strong metadata lifecycle, and broad regression tooling. The highest-leverage immediate work is fixing metadata drift + unknown-rule validation, then migrating heuristic rules to typed semantic queries.
