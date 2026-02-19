# ADR 0004: Fix Safety Policy

Date: 2026-02-19
Status: Accepted
Owners: aztec-lint maintainers

## Context

The spec defines `aztec-lint fix [path]` and excludes auto-refactoring beyond safe edits in v0. A concrete safety boundary is required before implementing fixes.

## Decision

v0 fix operations are restricted to span-local, semantics-preserving edits with deterministic application.

Allowed fix class (v0):

1. Single-file edits only.
2. Single contiguous span replacement only.
3. No symbol renames across references.
4. No control-flow rewrites.
5. No edits requiring type inference assumptions beyond direct local evidence.

Disallowed in v0:

- Cross-file changes.
- Multi-location coordinated edits.
- Speculative transformations.
- Any fix that changes public/private/security semantics.

## Application Contract

Fix execution behavior:

1. Lint phase computes candidate fixes.
2. Fixer validates span boundaries against current file snapshot.
3. Non-overlapping fixes are applied in reverse span order.
4. Any failed precondition drops that fix; process continues deterministically.
5. Exit with code `2` only for internal fix engine errors.

Conflict policy:

- If two fixes overlap, prefer the higher-confidence fix.
- If same confidence, prefer lower rule id lexical order.
- Loser fix is skipped and recorded in debug output.

## Validation Requirements

Before claiming a fix as safe:

- Unit test proves unchanged parseability for target fixture.
- Golden test proves deterministic file result across repeated runs.
- Negative test verifies disallowed edits are rejected.

## Rationale

- Enforces safe-by-default behavior.
- Prevents accidental semantic regressions.
- Keeps implementation tractable for initial releases.

## Consequences

Positive:

- High trust in autofix output.
- Reproducible CI behavior.

Negative:

- Limited autofix coverage in v0.

## Manual Review Checklist

- [x] Safe edit boundary clearly defined
- [x] Disallowed edit classes listed
- [x] Conflict resolution policy deterministic
- [x] Validation gates defined
- [x] Team sign-off recorded

Sign-off:

- 2026-02-19: Accepted for Phase 0 gate.

