# ADR 0004: Fix Safety Policy

Date: 2026-02-19 (updated 2026-02-21)
Status: Accepted
Owners: aztec-lint maintainers

## Context

The spec defines `aztec-lint fix [path]` and excludes auto-refactoring beyond safe edits.
Phase 3 requires allowing grouped, same-file multi-location fixes while preserving deterministic and safe behavior.

## Decision

Fix operations are restricted to semantics-preserving edits with deterministic application.

Allowed fix class:

1. Single-file edits only.
2. Candidate may be:
  - one contiguous span replacement, or
  - one grouped fix with multiple edits in the same file.
3. No symbol renames across references.
4. No control-flow rewrites.
5. No edits requiring type inference assumptions beyond direct local evidence.
6. Grouped edits must be prevalidated for non-overlap and in-bounds spans.

Disallowed:

- Cross-file changes.
- Grouped edits spanning multiple files.
- Speculative transformations.
- Any fix that changes public/private/security semantics.

## Application Contract

Fix execution behavior:

1. Lint pass computes candidate fixes.
2. Fixer validates candidate boundaries against current file snapshot.
3. Conflict detection and ranking are performed at candidate level (single edit or grouped candidate).
4. Candidate application is atomic:
  - single edit: applied directly;
  - grouped candidate: all edits apply as one transaction, or none are applied.
5. Within a grouped candidate, edits are applied in reverse span order after full prevalidation.
6. Any failed precondition drops that candidate; process continues deterministically.
7. Exit with code `2` only for internal fix engine errors.

Conflict policy:

- If two candidates overlap, prefer the higher-confidence candidate.
- If same confidence, prefer lower rule id lexical order.
- Loser candidate is skipped and recorded in debug output.
- Overlap checks include grouped-vs-grouped and grouped-vs-single candidates.

## Validation Requirements

Before claiming a fix as safe:

- Unit test proves unchanged parseability for target fixture.
- Golden test proves deterministic file result across repeated runs.
- Negative test verifies disallowed edits are rejected.
- Grouped fix tests prove transaction semantics (no partial writes).

## Rationale

- Enforces safe-by-default behavior.
- Prevents accidental semantic regressions.
- Enables richer autofix coverage while preserving safety and determinism.

## Consequences

Positive:

- High trust in autofix output, including grouped edits.
- Reproducible CI behavior.

Negative:

- Atomic grouped application adds implementation complexity.

## Manual Review Checklist

- [x] Safe edit boundary clearly defined
- [x] Grouped same-file edit class defined
- [x] Disallowed edit classes listed
- [x] Transaction (all-or-none) apply semantics defined
- [x] Conflict resolution policy deterministic
- [x] Validation gates defined
- [x] Team sign-off recorded

Sign-off:

- 2026-02-21: Updated for Phase 3 grouped fix transaction semantics.
