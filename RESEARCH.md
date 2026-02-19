# AZTEC-LINT Research Requirements (RPI)

Date: 2026-02-19
Repository: `aztec-lint` (spec-only)
Primary input: `SPEC.md`

## 1. Research Scope

Goal: produce implementation requirements for the standalone `aztec-lint` Rust application described in `SPEC.md`, grounded in real Noir/Aztec code paths to avoid assumption-driven design.

## 2. Ground Truth Sources Used

### Local source of truth
- `SPEC.md`

### External primary sources (validated)
- Noir compiler repository structure and crates (GitHub API):
  - https://github.com/noir-lang/noir
- Noir frontend pass structure (`noirc_frontend`):
  - https://raw.githubusercontent.com/noir-lang/noir/master/compiler/noirc_frontend/src/lib.rs
- Noir driver compile/check APIs (`noirc_driver`):
  - https://raw.githubusercontent.com/noir-lang/noir/master/compiler/noirc_driver/src/lib.rs
- Noir attribute grammar/parser behavior (`SecondaryAttributeKind`, `Meta` fallback):
  - https://raw.githubusercontent.com/noir-lang/noir/master/compiler/noirc_frontend/src/parser/parser/attributes.rs
- Noir language attributes docs (v1.0.0-beta.18):
  - https://raw.githubusercontent.com/noir-lang/noir/master/docs/versioned_docs/version-v1.0.0-beta.18/noir/concepts/attributes.md
- Aztec packages repo (branch `next`, updated 2026-02-19), contracts and docs:
  - https://github.com/AztecProtocol/aztec-packages
- Aztec function visibility and `only_self` behavior:
  - https://github.com/AztecProtocol/aztec-packages/blob/next/docs/docs-developers/docs/aztec-nr/framework-description/functions/visibility.md
- Aztec contract calling patterns (`self.call`, `self.enqueue`, `Contract::at`):
  - https://github.com/AztecProtocol/aztec-packages/blob/next/docs/docs-developers/docs/aztec-nr/framework-description/calling_contracts.md
- SARIF standard version source:
  - https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html
- GitHub SARIF ingestion constraints (practical CI target):
  - https://docs.github.com/en/code-security/reference/code-scanning/sarif-support-for-code-scanning

## 3. Confirmed Product Requirements (from SPEC)

### 3.1 Product and UX
- Binary: `aztec-lint` (standalone first).
- Future integration: `aztec lint` via dependency embedding (not subprocess).
- Deterministic and CI-friendly execution.
- Outputs: text, JSON, SARIF.
- Exit codes: `0` (no blocking diagnostics), `1` (threshold hit), `2` (internal error).

### 3.2 Core architecture
- Rust implementation.
- Prefer Noir compiler crates (semantic data), not syntax-only parsers.
- Pipeline:
  1. Noir frontend analysis
  2. Generic analysis model
  3. Aztec semantic augmentation
  4. Rule engine

### 3.3 Analysis model requirements
- Generic model must include:
  - AST
  - spans
  - symbols
  - type information (when available)
  - call graph (best effort)
  - module graph
- Aztec model activation triggers:
  - `#[aztec]` contract marker
  - `aztec` imports
  - `--profile aztec`

### 3.4 Rules and policy
- Packs:
  - `noir_core` (default)
  - `aztec_pack` (`--profile aztec`)
- Confidence and suppression are first-class metadata.
- Default-on rules must prioritize low false positives.

### 3.5 Configuration and suppression
- Config file names:
  - `aztec-lint.toml`
  - `noir-lint.toml`
- Profiles/rulesets and Aztec semantic knobs are configurable.
- Suppression syntax required by spec:
  - `#[allow(AZTEC010)]`
  - `#[allow(noir_core::NOIR100)]`

## 4. Ecosystem Reality Checks (critical for implementation)

### 4.1 Noir compiler integration is feasible
- Confirmed crates exist and are actively used in compiler/tooling:
  - `noirc_frontend`, `noirc_driver`, `noirc_errors`, etc.
- `noirc_driver` provides practical entry points for check/compile workflows (`check_crate`, `compile_main`, `compile_contract`).
- Requirement: pin an exact Noir commit/version; APIs are internal and can shift.

### 4.2 Attribute parsing supports extensibility
- Noir parser converts unknown attributes into `SecondaryAttributeKind::Meta(...)`.
- Implication: linter-specific metadata/suppressions are parseable at AST level.
- Requirement: suppression logic should read parsed attributes directly, not regex raw source.

### 4.3 Current Aztec contract patterns are broader than SPEC examples
Validated in `aztec-packages` contracts/docs:
- Entrypoint kinds in practice include `"public"`, `"private"`, and `"utility"`.
- Private-to-public bridging patterns include both:
  - `self.enqueue(Contract::at(addr).public_fn(...))`
  - `self.enqueue_self.some_public_fn(...)`
- Notes access patterns include both:
  - high-level `state_var.get_notes(NoteGetterOptions::new())`
  - low-level `aztec::oracle::notes::get_notes(...)`
- Delivery style in code is `MessageDelivery.ONCHAIN_CONSTRAINED` (dot-access enum object style), not only `MessageDelivery::...`.

### 4.4 SARIF must target 2.1.0 and GitHub ingestion expectations
- SARIF version must be 2.1.0.
- Stable `ruleId`, stable relative file paths, and partial fingerprints are important for deduped CI/code-scanning behavior.
- Requirement: produce deterministic fingerprints for each diagnostic.

## 5. High-Risk Ambiguities to Resolve Before Coding Rules

1. AZTEC010 rule scope is currently underspecified.
- Spec says: public function called via enqueue must be `#[only_self]`.
- Conflict with real examples: cross-contract enqueue is valid and common (e.g., token mint call).
- Required decision: enforce `#[only_self]` only for same-contract private->public transitions, not cross-contract calls.

2. Suppression compatibility policy.
- Spec requires custom `#[allow(RULE_ID)]` forms.
- Noir supports `allow(...)` syntax, but behavior for unknown allow keys across toolchain versions should be validated against real compilation.

3. Confidence scoring model.
- `--min-confidence` is in CLI spec, but confidence computation policy is not defined.
- Required decision: deterministic scoring rubric per rule.

4. `fix` command expectations.
- Spec defines `aztec-lint fix [path]` but does not define autofix safety classes.
- Required decision: only machine-safe edits in v0 (format-preserving, local, no semantic uncertainty).

## 6. Implementation Requirements (actionable backlog)

### 6.1 Crate layout
- Implement workspace with:
  - `crates/aztec-lint-cli`
  - `crates/aztec-lint-core`
  - `crates/aztec-lint-rules`
  - `crates/aztec-lint-aztec`
- Keep plugin API boundary in `core` for later `aztec-lint-sdk`.

### 6.2 Core contracts/interfaces
- Define stable internal IR:
  - `ProjectModel` (Noir generic)
  - `AztecModel` (augmentation)
  - `Diagnostic`
  - `Rule`, `RulePack`, `RuleContext`
- Diagnostic schema must support:
  - primary + secondary spans
  - suggestions/fixes
  - confidence + severity + policy
  - suppression provenance

### 6.3 CLI
- Commands:
  - `check`, `fix`, `rules`, `explain`, `aztec scan`
- Flags:
  - profile, format, severity threshold, rule overrides, changed-only, confidence threshold
- Deterministic sorting in outputs (path, span, rule_id, message hash).

### 6.4 Rule execution engine
- Deterministic traversal order.
- Per-rule enable/disable override stack:
  1. CLI overrides
  2. config profile
  3. default rule policy
- Suppression evaluation attached to owning AST item/function.

### 6.5 Aztec semantic pass
- Contract detection via attributes/import/profile.
- Build symbol-indexed maps for:
  - entrypoints
  - storage structs
  - note read/write ops
  - nullifier emits
  - public sinks
  - enqueue/self-enqueue sites
- For taint v1: intra-procedural only, with extensible def-use graph for inter-procedural phase.

### 6.6 Output adapters
- `text` formatter for humans.
- canonical `json` formatter matching internal diagnostic schema.
- `sarif` formatter with stable fingerprints and repository-relative URIs.

### 6.7 Testing/fixtures
- Fixture sets:
  - `fixtures/noir_core`
  - `fixtures/aztec`
- Include positive/negative/suppressed cases per default-on rule.
- Golden tests for JSON and SARIF determinism.

## 7. Immediate Build Order (lowest risk)

1. Workspace scaffold + CLI skeleton + config loader.
2. Noir parse/check integration + span mapping.
3. Core diagnostics and formatter plumbing (text/json).
4. Implement 2-3 high-signal rules first:
   - `NOIR001`, `AZTEC001`, `AZTEC020`.
5. Add Aztec semantic extraction (entrypoints, enqueue, notes).
6. Add SARIF emitter + fingerprint tests.
7. Expand rule set and `fix` support incrementally.

## 8. Validation Checklist for “Research Complete”

- Requirements mapped from `SPEC.md`: complete.
- Authoritative implementation anchors identified: complete.
- Known spec-vs-ecosystem gaps documented with explicit decisions needed: complete.
- No assumptions left unmarked in high-risk areas (enqueue scope, suppression semantics, confidence policy, autofix safety): complete.
