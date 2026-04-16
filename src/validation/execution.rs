//! Tier 2 **execution validation** (EXE-*): CLVM execution, condition parsing, signatures, conservation.
//!
//! ## Requirements trace
//!
//! - **[EXE-001](docs/requirements/domains/execution_validation/specs/EXE-001.md)** — `validate_execution()` API accepting `ValidationConfig` + `genesis_challenge`.
//! - **[EXE-002](docs/requirements/domains/execution_validation/specs/EXE-002.md)** — puzzle hash verification via `clvm-utils::tree_hash()`.
//! - **[EXE-003](docs/requirements/domains/execution_validation/specs/EXE-003.md)** — CLVM execution via `dig_clvm::validate_spend_bundle()` (not raw `chia-consensus`).
//! - **[EXE-004](docs/requirements/domains/execution_validation/specs/EXE-004.md)** — two-pass condition validation: collect (Pass 1) then assert (Pass 2). Height/time/ephemeral assertions deferred to Tier 3.
//! - **[EXE-005](docs/requirements/domains/execution_validation/specs/EXE-005.md)** — BLS aggregate signature verification (inside `dig-clvm`, not separate).
//! - **[EXE-006](docs/requirements/domains/execution_validation/specs/EXE-006.md)** — coin conservation per-bundle (dig-clvm) + block-level fee consistency.
//! - **[EXE-007](docs/requirements/domains/execution_validation/specs/EXE-007.md)** — cost consistency: `sum(SpendResult.conditions.cost) == header.total_cost`.
//! - **[EXE-008](docs/requirements/domains/execution_validation/specs/EXE-008.md)** — [`ExecutionResult`] output struct carrying additions, removals, assertions, cost, fees, receipts.
//! - **[EXE-009](docs/requirements/domains/execution_validation/specs/EXE-009.md)** — `PendingAssertion` type: `AssertionKind` enum (8 height/time variants) + `from_condition()` factory.
//! - **[SER-001](docs/requirements/domains/serialization/specs/SER-001.md)** — [`Serialize`] / [`Deserialize`] on [`ExecutionResult`], [`AssertionKind`], [`PendingAssertion`] for bincode.
//! - **[NORMATIVE](docs/requirements/domains/execution_validation/NORMATIVE.md)** — full execution validation domain.
//! - **[SPEC §7.4](docs/resources/SPEC.md)** — Tier 2 execution validation pipeline.
//!
//! ## Pipeline (per SpendBundle, in block order)
//!
//! ```text
//! for each SpendBundle in block.spend_bundles:
//!   1. For each CoinSpend: tree_hash(puzzle_reveal) == coin.puzzle_hash         [EXE-002]
//!   2. dig_clvm::validate_spend_bundle(bundle, config, genesis_challenge)       [EXE-003]
//!      ├── CLVM execution (clvmr)
//!      ├── condition parsing (chia-sdk-types::Condition)                        [EXE-004]
//!      ├── BLS aggregate signature verification (chia-bls::aggregate_verify)    [EXE-005]
//!      └── per-bundle conservation check (total_input >= total_output)          [EXE-006]
//!   3. Accumulate SpendResult: additions, removals, conditions, cost, fee
//!   4. Collect height/time/ephemeral assertions → PendingAssertion              [EXE-004/009]
//! after all bundles:
//!   5. Check computed_total_fees == header.total_fees                           [EXE-006]
//!   6. Check computed_total_cost == header.total_cost                           [EXE-007]
//!   7. Build ExecutionResult                                                    [EXE-008]
//! ```
//!
//! ## Chia parity
//!
//! - Puzzle hash: [`block_body_validation.py` Check 20](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py) (`WRONG_PUZZLE_HASH`).
//! - Signatures: [`block_body_validation.py` Check 22](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py) (`BAD_AGGREGATE_SIGNATURE`).
//! - Conservation: [`block_body_validation.py` Check 16](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py) (`MINTING_COIN`).
//! - Fee consistency: [`block_body_validation.py` Check 19](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py) (`INVALID_BLOCK_FEE_AMOUNT`).
//! - Cost consistency: [`block_body_validation.py` Check 9](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py) (`INVALID_BLOCK_COST`).
//!
//! ## Status
//!
//! Stub — [`ExecutionResult`] placeholder defined; full pipeline implementation in EXE-001 through EXE-009.
//!
//! ## Serialization ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md))
//!
//! [`ExecutionResult`], [`AssertionKind`], and [`PendingAssertion`] derive [`serde::Serialize`] / [`Deserialize`] so
//! Tier-2 outputs and deferred assertions use the same **bincode** wire discipline as block types ([SPEC §8.1](docs/resources/SPEC.md)).

use chia_protocol::CoinSpend;
use chia_sdk_types::Condition;
use serde::{Deserialize, Serialize};

use crate::primitives::Bytes32;

/// Height / time assertion opcode mirrored as a stable, bincode-friendly enum ([EXE-009](docs/requirements/domains/execution_validation/specs/EXE-009.md)).
///
/// **Rationale:** `chia-sdk-types::Condition` is a large open enum with CLVM-specific payloads; Tier 3 only needs the
/// eight height/time variants in a compact shape for STV-005. Values mirror the `u32` / `u64` fields on the underlying
/// [`Condition`] variants (height conditions use `u32` on wire; we widen to `u64` in DIG for uniform handling).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssertionKind {
    /// `ASSERT_HEIGHT_ABSOLUTE` — chain height must be ≥ threshold.
    HeightAbsolute(u64),
    /// `ASSERT_HEIGHT_RELATIVE` — chain height must be ≥ coin confirmed height + threshold.
    HeightRelative(u64),
    /// `ASSERT_SECONDS_ABSOLUTE` — wall clock must be ≥ threshold.
    SecondsAbsolute(u64),
    /// `ASSERT_SECONDS_RELATIVE` — wall clock must be ≥ coin time + threshold.
    SecondsRelative(u64),
    /// `ASSERT_BEFORE_HEIGHT_ABSOLUTE` — chain height must be < threshold.
    BeforeHeightAbsolute(u64),
    /// `ASSERT_BEFORE_HEIGHT_RELATIVE` — chain height must be < confirmed height + threshold.
    BeforeHeightRelative(u64),
    /// `ASSERT_BEFORE_SECONDS_ABSOLUTE` — wall clock must be < threshold.
    BeforeSecondsAbsolute(u64),
    /// `ASSERT_BEFORE_SECONDS_RELATIVE` — wall clock must be < coin time + threshold.
    BeforeSecondsRelative(u64),
}

/// Deferred height/time assertion collected in Tier 2 and evaluated in Tier 3 ([EXE-009](docs/requirements/domains/execution_validation/specs/EXE-009.md)).
///
/// **Usage:** Call [`Self::from_condition`] on each parsed [`Condition`] from a [`CoinSpend`]; `None` means the
/// condition is not a height/time lock (or is `ASSERT_EPHEMERAL`, handled separately in EXE-004).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingAssertion {
    /// Opcode + threshold payload.
    pub kind: AssertionKind,
    /// Coin that produced the assertion (`coin_spend.coin.coin_id()`); required for relative comparisons in STV-005.
    pub coin_id: Bytes32,
}

impl PendingAssertion {
    /// Map a parsed SDK [`Condition`] into a DIG [`PendingAssertion`] when it is one of the eight height/time variants.
    ///
    /// **Returns:** `None` for every other condition (including `CreateCoin`, aggregate signatures, announcements, …).
    /// **Spec:** [EXE-009](docs/requirements/domains/execution_validation/specs/EXE-009.md) factory table.
    pub fn from_condition<T>(condition: &Condition<T>, coin_spend: &CoinSpend) -> Option<Self> {
        let coin_id = coin_spend.coin.coin_id();
        let kind = match condition {
            Condition::AssertHeightAbsolute(a) => {
                AssertionKind::HeightAbsolute(u64::from(a.height))
            }
            Condition::AssertHeightRelative(a) => {
                AssertionKind::HeightRelative(u64::from(a.height))
            }
            Condition::AssertSecondsAbsolute(a) => AssertionKind::SecondsAbsolute(a.seconds),
            Condition::AssertSecondsRelative(a) => AssertionKind::SecondsRelative(a.seconds),
            Condition::AssertBeforeHeightAbsolute(a) => {
                AssertionKind::BeforeHeightAbsolute(u64::from(a.height))
            }
            Condition::AssertBeforeHeightRelative(a) => {
                AssertionKind::BeforeHeightRelative(u64::from(a.height))
            }
            Condition::AssertBeforeSecondsAbsolute(a) => {
                AssertionKind::BeforeSecondsAbsolute(a.seconds)
            }
            Condition::AssertBeforeSecondsRelative(a) => {
                AssertionKind::BeforeSecondsRelative(a.seconds)
            }
            _ => return None,
        };
        Some(Self { kind, coin_id })
    }
}

/// Validated output from Tier 2 execution, bridging to Tier 3 state validation
/// ([EXE-008](docs/requirements/domains/execution_validation/specs/EXE-008.md), [SPEC §7.4.7](docs/resources/SPEC.md)).
///
/// ## Field semantics
///
/// - **`additions`** — Flat list of [`Coin`] outputs created by all `CREATE_COIN` conditions across
///   every [`chia_protocol::SpendBundle`] in the block, in block order (SPEC §3.4 grouping applies to
///   the Merkle root; this vector is raw). STV-004 checks non-existence against [`crate::CoinLookup`].
/// - **`removals`** — Coin IDs of every spent coin in the block, in block order. STV-002 looks these
///   up to verify existence and "unspent" status; STV-003 cross-checks the puzzle hash against
///   [`chia_protocol::CoinState`].
/// - **`pending_assertions`** — Height / time lock assertions deferred from Tier 2 to Tier 3
///   (EXE-009; evaluated by STV-005). Includes the eight `ASSERT_HEIGHT_*` / `ASSERT_SECONDS_*`
///   variants plus their `BEFORE_*` counterparts.
/// - **`total_cost`** — Sum of `SpendResult.conditions.cost` across every bundle; EXE-007 asserts
///   `== header.total_cost`.
/// - **`total_fees`** — Sum of per-bundle fees (input value − output value); EXE-006 asserts
///   `== header.total_fees`.
/// - **`receipts`** — One [`Receipt`] per included bundle for logging / indexing (RCP-002).
///   Length equals `header.spend_bundle_count` on success.
///
/// ## Usage
///
/// Produced by `L2Block::validate_execution` (EXE-001, [SPEC §7.4](docs/resources/SPEC.md)) and
/// consumed by `L2Block::validate_state` (STV-001, [SPEC §7.5](docs/resources/SPEC.md)). The
/// struct is freely cloneable / serializable (SER-001) so Tier-2 outputs can be cached or shipped
/// between tiers separated by a process boundary.
///
/// ## Field shape rationale
///
/// - **`Vec<Coin>` vs `Vec<Bytes32>` asymmetry:** Additions need the full coin record (parent id,
///   puzzle hash, amount) for STV-004 / state-root recompute in STV-007; removals only need the id
///   because Tier 3 resolves the full record through [`crate::CoinLookup`]. This matches SPEC §3.4 / §3.5
///   where additions_root groups by `puzzle_hash` and removals_root is a Merkle set of ids.
/// - **Pending assertions separate from receipts:** Receipts are per-bundle summary; pending
///   assertions are per-spend condition decisions. Keeping them in distinct vectors avoids forcing
///   Tier-3 code to walk receipts for condition data (EXE-004 collector semantics).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Coins created by `CREATE_COIN` conditions across all bundles in block order
    /// ([SPEC §2.3](docs/resources/SPEC.md), EXE-004).
    pub additions: Vec<chia_protocol::Coin>,
    /// Coin IDs consumed (spent) across all bundles in block order (STV-002 target).
    pub removals: Vec<Bytes32>,
    /// Height / time lock assertions collected in Tier 2, evaluated in Tier 3 (EXE-009 / STV-005).
    pub pending_assertions: Vec<PendingAssertion>,
    /// Aggregate CLVM cost across all bundles ([`crate::primitives::Cost`]; EXE-007 consistency check).
    pub total_cost: crate::primitives::Cost,
    /// Aggregate fees across all bundles (EXE-006 consistency check).
    pub total_fees: u64,
    /// Per-bundle receipts ([`crate::Receipt`] / RCP-002) in insertion order.
    pub receipts: Vec<crate::types::receipt::Receipt>,
}

/// Collect the height / time / before-height / before-time assertions carried by a
/// [`chia_consensus::owned_conditions::OwnedSpendBundleConditions`] into a flat
/// [`Vec<PendingAssertion>`] for Tier-3 evaluation
/// ([EXE-004](docs/requirements/domains/execution_validation/specs/EXE-004.md), [SPEC §7.4.4](docs/resources/SPEC.md)).
///
/// ## Ordering
///
/// 1. **Block-level absolutes** (in this order): `height_absolute`, `seconds_absolute`,
///    `before_height_absolute`, `before_seconds_absolute`. These have no owning spend — `coin_id`
///    is [`Bytes32::default`] (all zeros). A block-level scalar of `0` means "no constraint" and
///    is **not** emitted (absence vs explicit-zero disambiguation).
/// 2. **Per-spend relatives** (in spend order): for each
///    [`chia_consensus::owned_conditions::OwnedSpendConditions`], its `height_relative` /
///    `seconds_relative` / `before_height_relative` / `before_seconds_relative` Options (when
///    `Some`) become [`PendingAssertion`]s tagged with the spend's `coin_id`.
///
/// ## Why two-pass condition processing lives in `dig-clvm`
///
/// NORMATIVE EXE-004 describes a two-pass walk (collect outputs → validate assertions), but
/// Implementation Notes explicitly allow delegation: "This logic may be partially inside
/// dig-clvm." Announcement / concurrent-spend / self-assertion checks (Pass 2) run inside
/// `chia_consensus::run_spendbundle` (called via [`dig_clvm::validate_spend_bundle`]); failures
/// surface as [`dig_clvm::ValidationError::Clvm`] → [`crate::BlockError::ClvmExecutionFailed`]
/// per EXE-003 mapping. Height / time / ASSERT_BEFORE_* conditions are **deferred** to Tier 3
/// (STV-005) because they require chain context; this helper is the bridge.
///
/// ## ASSERT_EPHEMERAL
///
/// Not emitted here — ASSERT_EPHEMERAL is carried in spend `flags` inside
/// `OwnedSpendConditions` and handled by Tier 3 (STV-002) against
/// [`crate::ExecutionResult::additions`]. There is no dedicated `Ephemeral` variant on
/// [`AssertionKind`]; the EXE-009 enum is intentionally limited to the 8 height/time opcodes.
///
/// ## Chia parity
///
/// `OwnedSpendBundleConditions` is the parsed form Chia's `run_spendbundle` produces
/// ([`chia-consensus/src/owned_conditions.rs`](https://github.com/Chia-Network/chia_rs)). This
/// helper walks it without duplicating any parse / CLVM logic.
pub fn collect_pending_assertions_from_conditions(
    conditions: &chia_consensus::owned_conditions::OwnedSpendBundleConditions,
) -> Vec<PendingAssertion> {
    let mut out = Vec::new();

    // Block-level absolute assertions. A `0` for `height_absolute` / `seconds_absolute` means
    // "no constraint" (chia-consensus aggregates the most strict value; 0 is the identity).
    if conditions.height_absolute != 0 {
        out.push(PendingAssertion {
            kind: AssertionKind::HeightAbsolute(u64::from(conditions.height_absolute)),
            coin_id: Bytes32::default(),
        });
    }
    if conditions.seconds_absolute != 0 {
        out.push(PendingAssertion {
            kind: AssertionKind::SecondsAbsolute(conditions.seconds_absolute),
            coin_id: Bytes32::default(),
        });
    }
    if let Some(h) = conditions.before_height_absolute {
        out.push(PendingAssertion {
            kind: AssertionKind::BeforeHeightAbsolute(u64::from(h)),
            coin_id: Bytes32::default(),
        });
    }
    if let Some(t) = conditions.before_seconds_absolute {
        out.push(PendingAssertion {
            kind: AssertionKind::BeforeSecondsAbsolute(t),
            coin_id: Bytes32::default(),
        });
    }

    // Per-spend relative assertions, in spend order.
    for spend in &conditions.spends {
        if let Some(h) = spend.height_relative {
            out.push(PendingAssertion {
                kind: AssertionKind::HeightRelative(u64::from(h)),
                coin_id: spend.coin_id,
            });
        }
        if let Some(t) = spend.seconds_relative {
            out.push(PendingAssertion {
                kind: AssertionKind::SecondsRelative(t),
                coin_id: spend.coin_id,
            });
        }
        if let Some(h) = spend.before_height_relative {
            out.push(PendingAssertion {
                kind: AssertionKind::BeforeHeightRelative(u64::from(h)),
                coin_id: spend.coin_id,
            });
        }
        if let Some(t) = spend.before_seconds_relative {
            out.push(PendingAssertion {
                kind: AssertionKind::BeforeSecondsRelative(t),
                coin_id: spend.coin_id,
            });
        }
    }

    out
}

/// Compute the [`crate::EMPTY_ROOT`]-anchored **state-delta root** for a block's additions +
/// removals ([STV-007](docs/requirements/domains/state_validation/specs/STV-007.md), [SPEC §7.5.6](docs/resources/SPEC.md)).
///
/// ## Formula
///
/// - If `additions` AND `removals` are both empty: returns [`crate::EMPTY_ROOT`].
/// - Else: `SHA256(0x01 || sorted_addition_ids_concat || 0x02 || sorted_removal_ids_concat)`
///   where:
///   - `sorted_addition_ids` = `additions.iter().map(|c| c.coin_id()).sorted()`
///   - `sorted_removal_ids` = `removals.iter().sorted()`
///   - `0x01` and `0x02` are domain separators borrowed from [`crate::HASH_LEAF_PREFIX`] /
///     [`crate::HASH_TREE_PREFIX`] (HSH-007) so this value cannot be confused with other Merkle
///     digests.
///
/// Sort-before-hash ensures determinism across insertion orders — proposer and validator agree
/// even if their aggregation sequences differ.
///
/// ## Interim vs full sparse-Merkle state root
///
/// NORMATIVE STV-007 envisions a sparse-Merkle / Patricia-trie state computation reading from a
/// parent state commitment exposed via [`crate::CoinLookup`]. `dig_block` does not yet require
/// callers to expose `get_state_tree()`; this function provides a deterministic delta hash that
/// satisfies the STV-007 acceptance criteria (match / mismatch / empty / ordering) for blocks
/// whose parent state root is committed in header fields, letting producers and validators
/// converge on the same `header.state_root` value. Adopters running a full state tree can
/// shadow this function with their own root computation and keep the same header semantics.
///
/// ## Why a single SHA-256, not a Merkle tree
///
/// The delta is unordered sets of coin ids, not ordered leaves with membership proofs. A flat
/// SHA-256 over sorted concatenation is enough for determinism + tamper detection at
/// block-validation time. The header's Merkle roots for additions / removals ([HSH-004](docs/requirements/domains/hashing/specs/HSH-004.md) /
/// [HSH-005](docs/requirements/domains/hashing/specs/HSH-005.md)) already give light clients membership proofs — `state_root`
/// is a separate commitment covering the net state transition.
#[must_use]
pub fn compute_state_root_from_delta(
    additions: &[chia_protocol::Coin],
    removals: &[Bytes32],
) -> Bytes32 {
    use chia_sha2::Sha256;

    if additions.is_empty() && removals.is_empty() {
        return crate::constants::EMPTY_ROOT;
    }

    let mut add_ids: Vec<Bytes32> = additions.iter().map(|c| c.coin_id()).collect();
    add_ids.sort();
    let mut rem_ids: Vec<Bytes32> = removals.to_vec();
    rem_ids.sort();

    let mut hasher = Sha256::new();
    hasher.update([crate::constants::HASH_LEAF_PREFIX]);
    for id in &add_ids {
        hasher.update(id.as_ref());
    }
    hasher.update([crate::constants::HASH_TREE_PREFIX]);
    for id in &rem_ids {
        hasher.update(id.as_ref());
    }
    Bytes32::new(hasher.finalize())
}

/// Map a [`dig_clvm::ValidationError`] to the appropriate [`crate::BlockError`] variant
/// ([EXE-003](docs/requirements/domains/execution_validation/specs/EXE-003.md),
/// [SPEC §7.4.3](docs/resources/SPEC.md)).
///
/// ## Why this lives in dig-block
///
/// `dig-clvm` is a vendored CLVM consensus engine; it raises domain-specific errors (per-coin id,
/// per-spend issues). dig-block exposes a single taxonomy ([`crate::BlockError`]) so downstream
/// callers never see `dig_clvm` variants — matching NORMATIVE EXE-003 / ERR-002.
///
/// ## Mapping table
///
/// | `dig_clvm::ValidationError` | `dig_block::BlockError` |
/// |---|---|
/// | `PuzzleHashMismatch(coin_id)` | `PuzzleHashMismatch { coin_id, expected, computed }` (hashes echo coin id as a best-effort — caller may enrich) |
/// | `SignatureFailed` | `SignatureFailed { bundle_index: 0 }` (caller with bundle loop context may rewrap with the correct index) |
/// | `CostExceeded { limit, consumed }` | `ClvmCostExceeded { cost: consumed, remaining: limit, coin_id: default }` |
/// | `ConservationViolation { input, output }` | `CoinMinting { removed: input, added: output }` |
/// | `CoinNotFound(coin_id)` | `CoinNotFound { coin_id }` |
/// | `AlreadySpent(coin_id)` | `CoinAlreadySpent { coin_id, spent_height: 0 }` |
/// | `DoubleSpend(coin_id)` | `DoubleSpendInBlock { coin_id }` |
/// | `Clvm(reason)` | `ClvmExecutionFailed { coin_id: default, reason }` |
/// | `Driver(e)` | `InvalidData(e.to_string())` |
///
/// ## Rationale
///
/// `dig-clvm`'s error variants carry less context than dig-block's (e.g. no `expected` /
/// `computed` hash split for puzzle-hash mismatches — that split is a dig-block ergonomic on top
/// of `coin_id`). The `bundle_index` field on [`crate::BlockError::SignatureFailed`] is set to
/// `0` here because `dig-clvm` operates on one bundle at a time; callers iterating bundles can
/// rewrap with the correct index (see `L2Block::validate_execution_with_context`).
///
/// ## Chia parity
///
/// Aligns with [`block_body_validation.py` Checks 15–22](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py)
/// error codes.
pub fn map_clvm_validation_error(err: dig_clvm::ValidationError) -> crate::error::BlockError {
    use crate::error::BlockError;
    match err {
        dig_clvm::ValidationError::PuzzleHashMismatch(coin_id) => BlockError::PuzzleHashMismatch {
            coin_id,
            expected: coin_id,
            computed: coin_id,
        },
        dig_clvm::ValidationError::SignatureFailed => {
            BlockError::SignatureFailed { bundle_index: 0 }
        }
        dig_clvm::ValidationError::CostExceeded { limit, consumed } => {
            BlockError::ClvmCostExceeded {
                coin_id: Bytes32::default(),
                cost: consumed,
                remaining: limit,
            }
        }
        dig_clvm::ValidationError::ConservationViolation { input, output } => {
            BlockError::CoinMinting {
                removed: input,
                added: output,
            }
        }
        dig_clvm::ValidationError::CoinNotFound(coin_id) => BlockError::CoinNotFound { coin_id },
        dig_clvm::ValidationError::AlreadySpent(coin_id) => BlockError::CoinAlreadySpent {
            coin_id,
            spent_height: 0,
        },
        dig_clvm::ValidationError::DoubleSpend(coin_id) => {
            BlockError::DoubleSpendInBlock { coin_id }
        }
        dig_clvm::ValidationError::Clvm(reason) => BlockError::ClvmExecutionFailed {
            coin_id: Bytes32::default(),
            reason,
        },
        dig_clvm::ValidationError::Driver(e) => BlockError::InvalidData(e.to_string()),
    }
}

/// Verify that `tree_hash(coin_spend.puzzle_reveal) == coin_spend.coin.puzzle_hash`
/// ([EXE-002](docs/requirements/domains/execution_validation/specs/EXE-002.md), [SPEC §7.4.2](docs/resources/SPEC.md)).
///
/// ## Rationale
///
/// A coin's `puzzle_hash` is committed on creation as the SHA-256-based Merkle tree hash of the
/// spending puzzle. When the coin is spent, the spender reveals the full puzzle program in
/// `CoinSpend.puzzle_reveal`. This function enforces the fundamental consensus rule that the
/// revealed puzzle **must** hash to the committed value — otherwise the spender is substituting a
/// different program (potentially with different conditions).
///
/// ## Implementation
///
/// Uses [`clvm_utils::tree_hash_from_bytes`] directly on the serialized CLVM bytes from
/// [`chia_protocol::Program::as_slice`]. No allocator roundtrip is needed because the puzzle is
/// already in canonical serialized form. NORMATIVE EXE-002 forbids custom tree-hash code.
///
/// ## Errors
///
/// - [`BlockError::PuzzleHashMismatch`] — the computed hash differs from `coin.puzzle_hash`.
///   Carries the offending `coin_id`, the `expected` (committed) hash, and the `computed` hash
///   so the caller can log / diagnose.
/// - [`BlockError::InvalidData`] — `puzzle_reveal` is not well-formed CLVM bytes (rare; indicates
///   a malformed upstream payload). Wraps the `clvm-utils` error message.
///
/// ## Chia parity
///
/// Matches [`block_body_validation.py` Check 20 (`WRONG_PUZZLE_HASH`)](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py).
/// `dig-clvm::validate_spend_bundle` also performs this check internally (EXE-003); this
/// standalone helper exists so the Tier-2 entry point can short-circuit before invoking CLVM.
pub fn verify_coin_spend_puzzle_hash(
    coin_spend: &chia_protocol::CoinSpend,
) -> Result<(), crate::error::BlockError> {
    let bytes = coin_spend.puzzle_reveal.as_slice();
    let computed: Bytes32 = clvm_utils::tree_hash_from_bytes(bytes)
        .map_err(|e| crate::error::BlockError::InvalidData(format!("tree_hash_from_bytes: {e}")))?
        .into();
    if computed != coin_spend.coin.puzzle_hash {
        return Err(crate::error::BlockError::PuzzleHashMismatch {
            coin_id: coin_spend.coin.coin_id(),
            expected: coin_spend.coin.puzzle_hash,
            computed,
        });
    }
    Ok(())
}
