Below is a **unified technical specification document**, structured and normalized for **LLM ingestion and downstream implementation**.

It consolidates all architectural decisions, conventions, and design constraints discussed.

---

# AZTEC-LINT

## Unified Technical Specification (LLM-Optimized)

Version: 0.1
Language: Rust
Primary Target: Aztec + Noir projects
Distribution: Standalone first, later integrated into Aztec CLI

---

# 1. Project Goals

## 1.1 Primary Objective

Build a **standalone Rust linter** for Noir projects that:

* Provides **general Noir linting**
* Provides **Aztec-specialized security/privacy rules**
* Is distributable independently
* Can later be integrated into `aztec` CLI as `aztec lint`

The tool must:

* Be deterministic
* Be CI-friendly
* Provide machine-readable output (JSON/SARIF)
* Minimize false positives
* Support configurable Aztec conventions
* Be extensible (plugin-ready design)

---

# 2. Tool Identity

Binary name (standalone phase):

```
aztec-lint
```

Later embedded as:

```
aztec lint
```

---

# 3. Architectural Model

The linter consists of three conceptual layers:

```
Noir Frontend → Generic Analysis Model → Aztec Semantic Augmentation → Rule Engine
```

---

# 4. Core Architecture

## 4.1 Language & Ecosystem

* Implemented in **Rust**
* Uses Noir compiler crates for parsing + semantic analysis (preferred over tree-sitter)
* Avoid pure syntax-only parsing for Aztec rules

Reason:
Aztec rules require:

* Type information
* Call graph awareness
* Symbol resolution
* Attribute inspection
* Cross-function reasoning

---

## 4.2 Internal Model

### 4.2.1 Generic Noir Model

Extract:

* AST
* Span (file, start, end)
* Symbol table
* Type information (if available)
* Call graph (best effort)
* Module graph

---

### 4.2.2 Aztec Semantic Augmentation Layer

Activated when:

* `#[aztec]` contract attribute found
* OR imports from `aztec::*`
* OR config profile `aztec` selected

Build:

```
AztecModel {
    contracts
    entrypoints
    storage_structs
    note_read_sites
    note_write_sites
    nullifier_emit_sites
    public_sinks
    enqueue_sites
}
```

Classification rules derived from aztec-starter:

Entrypoints:

* `#[external("public")]`
* `#[external("private")]`
* `#[initializer]`
* `#[only_self]`

Contract marker:

* `#[aztec] pub contract ...`

Storage:

* `#[storage] struct Storage { ... }`

Private notes:

* `.get_notes(NoteGetterOptions::new())`
* `.insert(...).deliver(MessageDelivery::ONCHAIN_CONSTRAINED)`

Public bridging:

* `self.enqueue(Contract::at(self.context.this_address()).fn(...))`

---

# 5. Rule System

Rules are grouped into **packs**.

---

# 6. Rule Packs

## 6.1 noir_core (General Noir)

Default profile.

### Correctness Rules (Default-On)

| ID      | Description                                       |
| ------- | ------------------------------------------------- |
| NOIR001 | Unused variable/import                            |
| NOIR002 | Suspicious shadowing                              |
| NOIR010 | Boolean computed but not asserted                 |
| NOIR020 | Array indexing without bounds validation          |
| NOIR030 | Unconstrained value influencing constrained logic |

---

### Maintainability Rules (Warn)

| ID      | Description          |
| ------- | -------------------- |
| NOIR100 | Magic numbers        |
| NOIR110 | Function too complex |
| NOIR120 | Excessive nesting    |

---

### Performance Rules (Opt-in)

| ID      | Description                 |
| ------- | --------------------------- |
| NOIR200 | Heavy operation inside loop |

---

## 6.2 aztec_pack (Aztec-Specialized)

Enabled by `--profile aztec`.

---

### Privacy Rules (Default-On)

| ID       | Description                                       |
| -------- | ------------------------------------------------- |
| AZTEC001 | Secret/Note → Public sink leakage                 |
| AZTEC002 | Secret-dependent branching affecting public state |
| AZTEC003 | Private entrypoint uses debug logging             |

---

### Protocol Rules (Default-On)

| ID       | Description                                               |
| -------- | --------------------------------------------------------- |
| AZTEC010 | Public function called via enqueue must be `#[only_self]` |
| AZTEC011 | Nullifier missing domain separation fields                |
| AZTEC012 | Commitment missing domain separation fields               |

---

### Soundness Rules (Default-On)

| ID       | Description                                               |
| -------- | --------------------------------------------------------- |
| AZTEC020 | Unconstrained influence on commitments/storage/nullifiers |
| AZTEC021 | Missing range constraints before hashing/serialization    |
| AZTEC022 | Suspicious Merkle witness usage                           |

---

### Constraint Cost (Opt-in)

| ID       | Description                 |
| -------- | --------------------------- |
| AZTEC040 | Expensive primitive in loop |
| AZTEC041 | Repeated membership proofs  |

---

# 7. Taint Model (Aztec)

Sources:

* Values read from notes
* Private entrypoint parameters
* Secret state

Sinks:

* Public entrypoint outputs
* Public storage writes
* Enqueued public calls
* Oracle arguments
* Logs/events

Propagation:

* Intra-procedural def-use
* Later extend to inter-procedural

---

# 8. Configuration System

File name:

```
aztec-lint.toml
```

or

```
noir-lint.toml
```

---

## Example Config

```
[profile.default]
ruleset = ["noir_core"]

[profile.aztec]
extends = ["default"]
ruleset = ["noir_core", "aztec_pack"]

[aztec]
contract_attribute = "aztec"
external_attribute = "external"
external_kinds = ["public", "private"]
only_self_attribute = "only_self"
initializer_attribute = "initializer"
storage_attribute = "storage"

imports_prefixes = ["aztec", "::aztec"]

note_getter_fns = ["get_notes"]
nullifier_fns = ["emit_nullifier", "nullify"]
enqueue_fn = "enqueue"
contract_at_fn = "at"

[aztec.domain_separation]
nullifier_requires = ["contract_address", "nonce"]
commitment_requires = ["contract_address", "note_type"]
```

---

# 9. CLI Specification

## Commands

```
aztec-lint check [path]
aztec-lint fix [path]
aztec-lint rules
aztec-lint explain <RULE_ID>
aztec-lint aztec scan
```

---

## Flags

```
--profile default|aztec
--format text|json|sarif
--severity-threshold warning|error
--deny RULE_ID
--warn RULE_ID
--allow RULE_ID
--changed-only
--min-confidence high|medium|low
```

---

## Exit Codes

| Code | Meaning                 |
| ---- | ----------------------- |
| 0    | No blocking diagnostics |
| 1    | Diagnostics ≥ threshold |
| 2    | Internal error          |

---

# 10. Output Format

Each diagnostic:

```
{
  "rule_id": "AZTEC001",
  "severity": "error",
  "confidence": "high",
  "policy": "privacy",
  "message": "...",
  "primary_span": {...},
  "secondary_spans": [...],
  "suggestions": [...],
  "fixes": [...]
}
```

Must support SARIF.

---

# 11. Repository Layout

```
crates/
  aztec-lint-cli/
  aztec-lint-core/
  aztec-lint-rules/
  aztec-lint-aztec/
  aztec-lint-sdk/        (future plugins)

fixtures/
  noir_core/
  aztec/
```

---

# 12. Integration Plan with Aztec CLI

Phase 1:
Standalone binary.

Phase 2:
Refactor into reusable crates.

Aztec CLI adds:

```
aztec lint
```

Integration Strategy:

Preferred: embed `aztec-lint-core` as dependency inside Aztec CLI.

Avoid subprocess model.

Versioning:
Aztec CLI pins specific linter version.

---

# 13. Rule Quality Criteria

Default-on rules must:

* Be high-confidence
* Have low false positives
* Map to real audit findings
* Be suppressible via `#[allow(RULE_ID)]`

---

# 14. Suppression

Allow:

```
#[allow(AZTEC010)]
```

or block:

```
#[allow(noir_core::NOIR100)]
```

Suppressions must:

* Be attached to function/item
* Be visible in diagnostic output

---

# 15. Phased Implementation Plan

Phase 0:

* Parsing
* Basic rule runner
* 8 noir_core rules

Phase 1:

* Aztec detection
* AZTEC001
* AZTEC010
* AZTEC020

Phase 2:

* Taint propagation
* SARIF
* Config system stable

Phase 3:

* Plugin system (WASM)

---

# 16. Design Principles

1. Deterministic analysis
2. No network calls
3. Zero telemetry
4. Config-over-convention
5. Safe-by-default
6. High signal over high coverage

---

# 17. Future Extensibility

Plugin system:

* WASM-based rule extensions
* Versioned API
* Sandbox execution

---

# 18. Non-Goals (v0)

* Full formal verification
* Whole-program constraint counting
* Auto-refactoring beyond safe edits
* IDE integration (later via LSP adapter)

---

# 19. Long-Term Vision

`aztec lint` becomes:

* Default CI step
* Audit pre-check tool
* Enforcer of Aztec best practices
* Standard privacy guardrail layer

---

If desired, next step can be:

* Convert this into a production-ready README
* Generate initial crate scaffolding
* Or write the concrete implementation skeleton for `AZTEC001` in Rust.
