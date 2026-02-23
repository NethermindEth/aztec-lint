# Intake Status: Covered

Generated from intake decisions in `docs/NEW_LINTS.md`.

- Status: `covered`
- Proposal count: `4`

| Proposal | Canonical mapping | Notes |
|---|---|---|
| `AZTEC_ENQUEUE_NOT_ONLY_SELF` | `AZTEC010` | Existing enqueue + `#[only_self]` boundary lint. |
| `AZTEC_PRIVATE_TO_ENQUEUE_TAINT` | `AZTEC001` | Covered, with planned enqueue-argument sink precision improvements. |
| `AZTEC_DEBUG_LOG_IN_PRIVATE` | `AZTEC003` | Existing private debug-log guard. |
| `AZTEC_UNCONSTRAINED_AFFECTS_SINK` | `AZTEC020` | Existing unconstrained-to-critical-sink coverage. |

[Back to intake index](index.md)
