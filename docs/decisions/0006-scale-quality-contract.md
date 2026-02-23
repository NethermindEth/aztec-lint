# ADR 0006: Scale and Quality Contract

Date: 2026-02-23
Status: Accepted
Owners: aztec-lint maintainers

## Context

Rule-count growth, expanded test coverage, and performance gates need an explicit contract before broad rule expansion.
Without a locked contract, new rules can ship with inconsistent maturity labels, uneven test matrix depth, and unclear perf budget expectations.

## Decision

1. Maturity model is required for every active lint:
  - `stable`: high-confidence, intended for broad/default use.
  - `preview`: useful but still converging on precision and ergonomics.
  - `experimental`: opt-in investigation tier with stricter rollout controls.
2. Quantitative targets are fixed:
  - short-term target: at least 48 active lints with category coverage;
  - long-term target: 100+ active lints;
  - category floor: no category below 6 active lints in the short-term target set.
3. Minimum test matrix obligations by maturity tier:
  - `stable`: required UI positive/negative/suppression fixtures, required regression snapshots, required fix before/after coverage for machine-applicable suggestions, and required corpus coverage.
  - `preview`: required UI positive/negative/suppression fixtures and regression snapshots; fix and corpus coverage required before promotion to `stable`.
  - `experimental`: required UI positive and negative fixtures; suppression and fix coverage are optional unless the rule claims machine-applicable fixes.
4. Perf budget policy:
  - Benchmark scenarios must be tracked in versioned budget files.
  - Allowed variance is up to 3% per scenario as expected noise.
  - Regressions above 3% and up to 8% require explicit budget update + rationale in review.
  - Regressions above 8% are blocked until optimized or explicitly re-baselined with maintainer sign-off.
5. Lint intake policy for external suggestions is mandatory:
  - Each suggestion is triaged with exactly one status: `covered`, `accepted`, `deferred`, or `rejected`.
  - `covered`: existing lint ID already addresses it.
  - `accepted`: new lint ID is assigned and scheduled.
  - `deferred`: valid idea, intentionally postponed.
  - `rejected`: duplicate/out-of-scope/high-noise suggestion not scheduled as proposed.
  - Intake status must be recorded in `docs/NEW_LINTS.md` and carried into roadmap tracking.
6. Automation and publication contract:
  - `cargo xtask new-lint` is the default lint authoring path once available.
  - `cargo xtask update-lints` regenerates metadata-derived artifacts and is required before merge.
  - Generated docs portal content must come from canonical metadata and is gated for drift in CI.

## Rationale

- Establishes one maturity and quality bar for all new rule work.
- Prevents rule growth from outrunning correctness, regression protection, and performance discipline.
- Creates a deterministic lint intake process for community proposals.

## Consequences

Positive:

- Predictable quality and release readiness across maturity tiers.
- Better CI confidence through explicit test matrix and perf budget gates.
- Lower proposal churn due to explicit lint intake statuses.

Negative:

- Higher upfront authoring overhead for new lints.
- Additional CI/runtime cost for matrix and benchmark validation.

## Manual Review Checklist

- [x] Maturity tier model defined
- [x] Quantitative rule-count targets defined
- [x] Minimum test matrix obligations defined
- [x] Perf budget policy and allowed variance defined
- [x] Lint intake statuses and mapping policy defined
- [x] `xtask` and docs portal generation contract defined

Sign-off:

- 2026-02-23: Accepted as the scale-and-quality contract baseline.
