# Rule Roadmap

This roadmap tracks planned lint growth and intake decisions.

## Planning Rules

1. Canonical lint metadata and runtime registry stay synchronized; no placeholder lint IDs are added to the catalog or registry before implementation milestones and tests are defined in this roadmap.
2. Release notes must include a `Rule Growth by Category` block and report net rule growth by category (`correctness`, `maintainability`, `privacy`, `protocol`, `soundness`) rather than only raw total rule count.
3. Every accepted lint must have a target release, owner, and matrix fixture plan before implementation starts.

## Category x Maturity Matrix

| Category | Maturity | Rule IDs | Owner | Status | Target Release |
|---|---|---|---|---|---|
| `correctness` | `stable` | `NOIR001`, `NOIR002`, `NOIR010`, `NOIR020`, `NOIR030` | `noir-core maintainers` | `active` | `0.4.0` |
| `correctness` | `preview` | `AZTEC035` | `aztec-pack maintainers` | `active` | `0.5.0` |
| `correctness` | `preview` | `AZTEC038`, `AZTEC039`, `AZTEC041` | `aztec-pack maintainers` | `planned` | `0.6.0` |
| `correctness` | `experimental` | _none_ | `aztec-pack maintainers` | `unplanned` | `TBD` |
| `maintainability` | `stable` | `NOIR100` | `noir-core maintainers` | `active` | `0.4.0` |
| `maintainability` | `preview` | `NOIR101`, `NOIR110`, `NOIR120` | `noir-core maintainers` | `active` | `0.4.0` |
| `maintainability` | `experimental` | `AZTEC050`, `AZTEC051` | `aztec-pack maintainers` | `deferred` | `0.7.0` |
| `privacy` | `stable` | `AZTEC001`, `AZTEC003` | `aztec-pack maintainers` | `active` | `0.4.0` |
| `privacy` | `preview` | `AZTEC002` | `aztec-pack maintainers` | `active` | `0.4.0` |
| `privacy` | `preview` | `AZTEC036`, `AZTEC037` | `aztec-pack maintainers` | `planned` | `0.6.0` |
| `privacy` | `experimental` | _none_ | `aztec-pack maintainers` | `unplanned` | `TBD` |
| `protocol` | `stable` | `AZTEC010` | `aztec-pack maintainers` | `active` | `0.4.0` |
| `protocol` | `preview` | `AZTEC031`, `AZTEC032`, `AZTEC033` | `aztec-pack maintainers` | `active` | `0.5.0` |
| `protocol` | `preview` | `AZTEC040` | `aztec-pack maintainers` | `planned` | `0.6.0` |
| `protocol` | `experimental` | _none_ | `aztec-pack maintainers` | `unplanned` | `TBD` |
| `soundness` | `stable` | `AZTEC020`, `AZTEC021`, `AZTEC022` | `aztec-pack maintainers` | `active` | `0.4.0` |
| `soundness` | `preview` | `AZTEC030`, `AZTEC034` | `aztec-pack maintainers` | `active` | `0.5.0` |
| `soundness` | `experimental` | _none_ | `aztec-pack maintainers` | `unplanned` | `TBD` |

## First-Wave Backlog (ROI)

These were the first-wave accepted lints from `docs/NEW_LINTS.md` and are now active with matrix coverage.

| Rule ID | Category | Maturity | Owner | Status | Target Release | Matrix Test Plan |
|---|---|---|---|---|---|---|
| `AZTEC030` | `soundness` | `preview` | `aztec-pack maintainers` | `active` | `0.5.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` + corpus diagnostic contract |
| `AZTEC031` | `protocol` | `preview` | `aztec-pack maintainers` | `active` | `0.5.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` |
| `AZTEC032` | `protocol` | `preview` | `aztec-pack maintainers` | `active` | `0.5.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` |
| `AZTEC033` | `protocol` | `preview` | `aztec-pack maintainers` | `active` | `0.5.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` |
| `AZTEC034` | `soundness` | `preview` | `aztec-pack maintainers` | `active` | `0.5.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` + benchmark stress scenario |
| `AZTEC035` | `correctness` | `preview` | `aztec-pack maintainers` | `active` | `0.5.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` |

## Second-Wave Backlog

| Rule ID | Category | Maturity | Owner | Status | Target Release | Matrix Test Plan |
|---|---|---|---|---|---|---|
| `AZTEC036` | `privacy` | `preview` | `aztec-pack maintainers` | `planned` | `0.6.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` |
| `AZTEC037` | `privacy` | `preview` | `aztec-pack maintainers` | `planned` | `0.6.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` |
| `AZTEC038` | `correctness` | `preview` | `aztec-pack maintainers` | `planned` | `0.6.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` |
| `AZTEC039` | `correctness` | `preview` | `aztec-pack maintainers` | `planned` | `0.6.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` |
| `AZTEC040` | `protocol` | `preview` | `aztec-pack maintainers` | `planned` | `0.6.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` |
| `AZTEC041` | `correctness` | `preview` | `aztec-pack maintainers` | `planned` | `0.6.0` | `positive`, `negative`, `suppressed`, `false_positive_guard` |

## Deferred Backlog

| Rule ID | Category | Maturity | Owner | Status | Target Release | Rationale |
|---|---|---|---|---|---|---|
| `AZTEC050` | `maintainability` | `experimental` | `aztec-pack maintainers` | `deferred` | `0.7.0` | opt-in cost/performance lint, not default profile |
| `AZTEC051` | `maintainability` | `experimental` | `aztec-pack maintainers` | `deferred` | `0.7.0` | opt-in cost/performance lint, not default profile |

<!-- generated:lint-intake:start -->
## Suggestion Intake Mapping

Generated by `cargo xtask lint-intake --source docs/NEW_LINTS.md`.

Status counts:
- `covered`: 10
- `accepted`: 6
- `deferred`: 2
- `rejected`: 1

| Proposal | Status | Canonical mapping | Notes |
|---|---|---|---|
| `AZTEC_ENQUEUE_NOT_ONLY_SELF` | `covered` | `AZTEC010` | Existing enqueue + `#[only_self]` boundary lint. |
| `AZTEC_PRIVATE_TO_ENQUEUE_TAINT` | `covered` | `AZTEC001` | Covered, with planned enqueue-argument sink precision improvements. |
| `AZTEC_DEBUG_LOG_IN_PRIVATE` | `covered` | `AZTEC003` | Existing private debug-log guard. |
| `AZTEC_UNCONSTRAINED_AFFECTS_SINK` | `covered` | `AZTEC020` | Existing unconstrained-to-critical-sink coverage. |
| `AZTEC_NOTE_CONSUMED_WITHOUT_NULLIFIER` | `covered` | `AZTEC030` | Implemented with rule-case/UI/corpus coverage in the first-wave rollout. |
| `AZTEC_DOMAIN_SEP_NULLIFIER` | `covered` | `AZTEC031` | Implemented with rule-case and UI matrix coverage in the first-wave rollout. |
| `AZTEC_DOMAIN_SEP_COMMITMENT` | `covered` | `AZTEC032` | Implemented with rule-case and UI matrix coverage in the first-wave rollout. |
| `AZTEC_PUBLIC_FN_MISSING_ONLY_SELF_WHEN_MUTATING_PRIVATE_STATE` | `covered` | `AZTEC033` | Implemented with rule-case and UI matrix coverage in the first-wave rollout. |
| `AZTEC_HASH_INPUT_NOT_RANGE_CONSTRAINED` | `covered` | `AZTEC034` | Implemented with rule-case/UI plus benchmark scenario coverage in the first-wave rollout. |
| `AZTEC_STORAGE_KEY_SUSPICIOUS` | `covered` | `AZTEC035` | Implemented with rule-case and UI matrix coverage in the first-wave rollout. |
| `AZTEC_SECRET_BRANCH_AFFECTS_ENQUEUE` | `accepted` | `AZTEC036` | Second-wave strict-profile candidate. |
| `AZTEC_SECRET_BRANCH_AFFECTS_DELIVERY_COUNT` | `accepted` | `AZTEC037` | Second-wave strict-profile candidate. |
| `AZTEC_CHANGE_NOTE_MISSING_FRESH_RANDOMNESS` | `accepted` | `AZTEC038` | Second-wave correctness/privacy candidate. |
| `AZTEC_PARTIAL_SPEND_NOT_BALANCED` | `accepted` | `AZTEC039` | Second-wave correctness candidate. |
| `AZTEC_INITIALIZER_NOT_ONLY_SELF` | `accepted` | `AZTEC040` | Accepted under broadened initializer misuse scope. |
| `AZTEC_CAST_TRUNCATION_RISK` | `accepted` | `AZTEC041` | Second-wave correctness candidate. |
| `AZTEC_HASH_IN_LOOP` | `deferred` | `AZTEC050` | Opt-in cost/performance wave, not default profile. |
| `AZTEC_MERKLE_VERIFY_IN_LOOP` | `deferred` | `AZTEC051` | Opt-in cost/performance wave, not default profile. |
| `AZTEC_PUBLIC_FN_MUTATES_PRIVATE_STATE_WITHOUT_ONLY_SELF` | `rejected` | `AZTEC033` | Duplicate alias of accepted lint; canonical name is `AZTEC033_PUBLIC_MUTATES_PRIVATE_WITHOUT_ONLY_SELF`. |

<!-- generated:lint-intake:end -->
