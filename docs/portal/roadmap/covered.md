# Intake Status: Covered

Generated from intake decisions in `docs/NEW_LINTS.md`.

- Status: `covered`
- Proposal count: `10`

| Proposal | Canonical mapping | Notes |
|---|---|---|
| `AZTEC_ENQUEUE_NOT_ONLY_SELF` | `AZTEC010` | Existing enqueue + `#[only_self]` boundary lint. |
| `AZTEC_PRIVATE_TO_ENQUEUE_TAINT` | `AZTEC001` | Covered, with planned enqueue-argument sink precision improvements. |
| `AZTEC_DEBUG_LOG_IN_PRIVATE` | `AZTEC003` | Existing private debug-log guard. |
| `AZTEC_UNCONSTRAINED_AFFECTS_SINK` | `AZTEC020` | Existing unconstrained-to-critical-sink coverage. |
| `AZTEC_NOTE_CONSUMED_WITHOUT_NULLIFIER` | `AZTEC030` | Implemented with rule-case/UI/corpus coverage in the first-wave rollout. |
| `AZTEC_DOMAIN_SEP_NULLIFIER` | `AZTEC031` | Implemented with rule-case and UI matrix coverage in the first-wave rollout. |
| `AZTEC_DOMAIN_SEP_COMMITMENT` | `AZTEC032` | Implemented with rule-case and UI matrix coverage in the first-wave rollout. |
| `AZTEC_PUBLIC_FN_MISSING_ONLY_SELF_WHEN_MUTATING_PRIVATE_STATE` | `AZTEC033` | Implemented with rule-case and UI matrix coverage in the first-wave rollout. |
| `AZTEC_HASH_INPUT_NOT_RANGE_CONSTRAINED` | `AZTEC034` | Implemented with rule-case/UI plus benchmark scenario coverage in the first-wave rollout. |
| `AZTEC_STORAGE_KEY_SUSPICIOUS` | `AZTEC035` | Implemented with rule-case and UI matrix coverage in the first-wave rollout. |

[Back to intake index](index.md)
