# Architecture

Date: 2026-02-19
Status: Active

## Purpose

This document summarizes the active architecture for `aztec-lint` and links the design decisions that contributors should treat as implementation contracts.

Core pipeline:

1. Noir frontend integration
2. Generic project model
3. Aztec semantic augmentation
4. Deterministic rule engine
5. Deterministic formatters (`text`/`json`/`sarif`)

## Decision Records

The following ADRs define core behavior constraints:

- `docs/decisions/0001-aztec010-scope.md`
- `docs/decisions/0002-suppression-semantics.md`
- `docs/decisions/0003-confidence-model.md`
- `docs/decisions/0004-fix-safety-policy.md`

Contributors should keep implementation aligned with accepted ADR decisions.

## Operator and Author Docs

- `docs/suppression.md`
- `docs/rule-authoring.md`
- `docs/plugin-api-v0.md`
- `docs/lints-reference.md`

## Stable Core APIs

The following public APIs are treated as stable contracts:

- `aztec_lint_core::model::{Span, ProjectModel, SemanticModel, AztecModel}`
- `aztec_lint_core::diagnostics::{Diagnostic, Severity, Confidence}`
- `aztec_lint_core::diagnostics::{sort_diagnostics, diagnostic_sort_key}`
- `aztec_lint_core::diagnostics::{diagnostic_fingerprint, span_fingerprint}`
- `aztec_lint_core::policy::*`

`ProjectModel.semantic` carries deterministic semantic facts for typed function inventory,
expression/statement nodes, CFG/DFG edges, call sites, and guard nodes.

## Semantic-First Enforcement

Correctness and soundness checks are semantic-first:

- typed semantic model/query facts are the default signal source
- text matching is fallback-only and must be isolated behind explicitly named helpers
- fallback execution must be gated on missing/incomplete semantic facts
