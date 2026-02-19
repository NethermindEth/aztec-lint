# Architecture Baseline

Date: 2026-02-19
Status: Active

## Purpose

This document records the baseline architecture for `aztec-lint` and links the Phase 0 decisions that must be treated as contracts before implementation work in Phase 1+.

Core pipeline:

1. Noir frontend integration
2. Generic project model
3. Aztec semantic augmentation
4. Deterministic rule engine
5. Deterministic formatters (text/json/sarif)

## Phase 0 Decision Contracts

The following ADRs are mandatory inputs to implementation:

- `docs/decisions/0001-aztec010-scope.md`
- `docs/decisions/0002-suppression-semantics.md`
- `docs/decisions/0003-confidence-model.md`
- `docs/decisions/0004-fix-safety-policy.md`

No Phase 1 work should proceed if any ADR above is not in `Accepted` state.

## Phase 2 Core API Contract

The following public APIs are frozen at Phase 2 exit and should be treated as stable
contracts for later phases:

- `aztec_lint_core::model::{Span, ProjectModel, AztecModel}`
- `aztec_lint_core::diagnostics::{Diagnostic, Severity, Confidence}`
- `aztec_lint_core::diagnostics::{sort_diagnostics, diagnostic_sort_key}`
- `aztec_lint_core::diagnostics::{diagnostic_fingerprint, span_fingerprint}`
- `aztec_lint_core::policy::*`
