Below is a set of **Aztec-specific lints** that tend to have high practical value (audit-ish) while still fitting a **Clippy-style** philosophy: clear trigger, clear remediation, configurable false-positive controls. I grouped them by “signal-to-noise” and included what to detect + what to suggest.

## Intake status snapshot (2026-02-23)

Execution scheduling (owner, target release, and matrix test obligations) is tracked in `docs/rule-roadmap.md`.
Accepted lint IDs remain roadmap items until implementation lands with explicit fixture and regression coverage.

Status meanings for lint intake:

- `covered`: already handled by an existing lint ID (possibly with follow-up precision work).
- `accepted`: scheduled as a new canonical lint ID.
- `deferred`: valid idea, intentionally postponed with a later wave.
- `rejected`: not scheduled as-is (duplicate alias, out-of-scope, or noise risk), with rationale.

Current triage mapping:

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

## High-signal Aztec lints to add first

### 1) `AZTEC_ENQUEUE_NOT_ONLY_SELF`

**Problem:** `self.enqueue(…public_fn…)` targets a public function that isn’t `#[only_self]`.
**Detect:** `enqueue(Contract::at(this_address).f(...))` where `f` is `#[external("public")]` and missing `#[only_self]`.
**Fix:** Add `#[only_self]` to `f` or avoid enqueueing to an externally callable function.

### 2) `AZTEC_PRIVATE_TO_ENQUEUE_TAINT`

**Problem:** secret/note-derived values passed into `enqueue` args. (Your AZTEC001 but tuned to Aztec boundary.)
**Detect:** Taint sources (note fields, `msg_sender` in private if treated secret, private params) flowing to `enqueue` arguments.
**Fix:** Pass commitments/hashes, indices/handles, or prove a predicate instead.

### 3) `AZTEC_DEBUG_LOG_IN_PRIVATE`

**Problem:** `debug_log_*` called in `#[external("private")]`.
**Detect:** call to `debug_log` / `debug_log_format` in private entrypoints.
**Fix:** remove or gate behind dev feature; allowlist for tests/examples.

### 4) `AZTEC_DOMAIN_SEP_NULLIFIER`

**Problem:** nullifier computed without required domain separation.
**Detect:** argument to `emit_nullifier` (or configured nullifier sink) is hash of a tuple missing any of required components (config: contract addr, selector, chain/version, nonce/note id).
**Fix:** include required fields; prefer canonical helper if available.

### 5) `AZTEC_DOMAIN_SEP_COMMITMENT`

Same as above, but for note commitments / state commitments / message commitments.

### 6) `AZTEC_UNCONSTRAINED_AFFECTS_SINK`

**Problem:** value from `unconstrained` function influences a “critical sink”: enqueue args, public storage write, nullifier, commitment.
**Detect:** dataflow from `unconstrained fn` result to sink.
**Fix:** recompute/verify constrained; move logic into constrained code; assert relationship.

---

## Privacy/metadata lints (useful, but should default to warn or “strict profile”)

### 7) `AZTEC_SECRET_BRANCH_AFFECTS_ENQUEUE`

**Problem:** secret-dependent branch changes *which* enqueue happens or its arguments.
**Detect:** control-flow taint from secret values to:

* presence/absence of `enqueue` call, or
* which function is enqueued, or
* which arguments are used.
  **Fix:** constant-shape enqueues; commit to choice privately then reveal proof/commitment.

### 8) `AZTEC_SECRET_BRANCH_AFFECTS_DELIVERY_COUNT`

**Problem:** secret-dependent branch changes number of delivered messages/notes.
**Detect:** secret-influenced control flow that guards `.deliver(...)` calls or causes variable number of `.insert(...).deliver(...)`.
**Fix:** pad with dummy deliveries; constant K outputs; or suppress in non-strict mode (token change-note patterns).

This is the refined version of your AZTEC002: make sinks precise and keep confidence labeling.

---

## Token-pattern correctness lints (very valuable for real contracts)

### 9) `AZTEC_NOTE_CONSUMED_WITHOUT_NULLIFIER`

**Problem:** notes popped/consumed but no nullifier emitted.
**Detect:** use of `pop_notes`/`pop_note`/“consume” APIs (config) without any nullifier emission in the same function (or along all paths).
**Fix:** emit nullifier per consumed note; use standard library consume helper.

### 10) `AZTEC_CHANGE_NOTE_MISSING_FRESH_RANDOMNESS`

**Problem:** constructing a change note reusing old randomness or using deterministic randomness missing sufficient entropy/context.
**Detect:** change note randomness equals `note.randomness` or derived without including unique fields (e.g., missing nonce/index).
**Fix:** include unique salt/nonce; use canonical randomness derivation helper.

### 11) `AZTEC_PARTIAL_SPEND_NOT_BALANCED`

**Problem:** `total_transferred + change != original_note_amount` or remaining logic can go negative/overflow.
**Detect:** for `u128` arithmetic, check suspicious patterns:

* `remaining -= note.amount` without guard,
* `change = note.amount - remaining` without ensuring `note.amount > remaining` (or equivalent),
* total doesn’t reconcile before assertion.
  **Fix:** tighten assertions / restructure logic to preserve invariants.

(This is Clippy-like: “arithmetic and invariant hygiene”, but Aztec-specific due to note accounting.)

---

## Storage/API misuse lints (often found in audits)

### 12) `AZTEC_PUBLIC_FN_MISSING_ONLY_SELF_WHEN_MUTATING_PRIVATE_STATE`

**Problem:** public external function writes private-note storage / performs private-only transitions without `#[only_self]`.
**Detect:** `#[external("public")] fn` that calls private storage mutation patterns (`insert(...).deliver`, `pop_notes`, etc.) and lacks `#[only_self]`.
**Fix:** add `#[only_self]` or refactor into private entrypoint.

### 13) `AZTEC_INITIALIZER_NOT_ONLY_SELF` (or “initializer misuse”)

**Problem:** `#[initializer]` entrypoint is callable beyond intended scope / missing idempotence guard.
**Detect:** initializer that writes storage but doesn’t set a “initialized” flag or rely on framework guard; configurable.
**Fix:** add guard or rely on framework initializer constraints (if present).

### 14) `AZTEC_STORAGE_KEY_SUSPICIOUS`

**Problem:** nested `.at(from).at(from)` patterns where keys repeat accidentally (common copy/paste).
**Detect:** `.at(x).at(x)` or `.at(a).at(a)` repeated keys on map-of-maps, where second key expected different (e.g., `token_id`, `spender`).
**Fix:** use intended second key; rename variables; add helper method.

This is very Clippy-esque: detect suspicious repetition.

---

## Serialization/hash lints (ZK-specific but very relevant)

### 15) `AZTEC_HASH_INPUT_NOT_RANGE_CONSTRAINED`

**Problem:** hashing `Field` values that are semantically `uN`/bytes without bounds check.
**Detect:** `poseidon*_hash([... amount as Field ...])` or `to_field()` inputs used in commitments without `assert(x < 2^k)` or “bit-size gadget”.
**Fix:** add range constraint; use typed wrappers.

### 16) `AZTEC_CAST_TRUNCATION_RISK`

**Problem:** `amount as Field` / `Field as u128` without explicit bounds; risk of truncation/aliasing.
**Detect:** casts across Field↔integer types without nearby `assert` enforcing range.
**Fix:** range assert or explicit conversion helper.

---

## Constraint-cost lints (opt-in, but useful)

### 17) `AZTEC_HASH_IN_LOOP`

**Detect:** poseidon/pedersen/etc. calls inside loop with non-trivial bound.
**Fix:** tree hash / batch gadget / move to public if safe.

### 18) `AZTEC_MERKLE_VERIFY_IN_LOOP`

**Detect:** repeated membership verification for same root, especially in loops.
**Fix:** batch verification / aggregate proofs / restructure.

---

## How to keep these Clippy-like (low-noise)

### Recommended metadata per lint

* `policy`: `privacy|protocol|soundness|performance|correctness` (`cost` is acceptable roadmap shorthand but canonical metadata should use `performance`)
* `confidence`: `high|medium|low`
* `default severity`: `deny` only for high-confidence protocol breaks (`ENQUEUE_NOT_ONLY_SELF`, `NOTE_CONSUMED_WITHOUT_NULLIFIER` if reliable)

### Recommended “profiles”

* `default`: only the “generic Noir + high-confidence Aztec”
* `aztec`: enable the Aztec pack
* `aztec_strict`: enables metadata-leak style lints like `SECRET_BRANCH_AFFECTS_DELIVERY_COUNT`

---

## Suggested next 6 lints to implement (best ROI)

If you already have AZTEC001/002/003/010/020-ish:

1. `AZTEC_NOTE_CONSUMED_WITHOUT_NULLIFIER`
2. `AZTEC_DOMAIN_SEP_NULLIFIER`
3. `AZTEC_DOMAIN_SEP_COMMITMENT`
4. `AZTEC033_PUBLIC_MUTATES_PRIVATE_WITHOUT_ONLY_SELF`
5. `AZTEC_HASH_INPUT_NOT_RANGE_CONSTRAINED`
6. `AZTEC_STORAGE_KEY_SUSPICIOUS`

If you paste your current lint list (rule IDs + one-line description), I can:

* de-duplicate overlaps,
* propose severities/default profiles,
* and map each new lint to the exact Aztec-starter AST patterns (`#[external("...")]`, `.deliver(...)`, `enqueue`, `pop_notes`, etc.) so implementation is straightforward.
