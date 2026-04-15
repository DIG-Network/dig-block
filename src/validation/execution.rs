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

/// Validated output from Tier 2 execution, carrying data forward to Tier 3 (state validation).
///
/// ## Fields (EXE-008)
///
/// When fully implemented, this struct will contain:
/// - `additions: Vec<Coin>` — coins created by `CREATE_COIN` conditions
/// - `removals: Vec<Bytes32>` — coin IDs consumed (spent) in this block
/// - `pending_assertions: Vec<PendingAssertion>` — height/time locks deferred to Tier 3 (EXE-009)
/// - `total_cost: Cost` — sum of CLVM execution costs across all bundles
/// - `total_fees: u64` — sum of per-bundle fees (input value - output value)
/// - `receipts: Vec<Receipt>` — per-SpendBundle execution receipts
///
/// Placeholder until EXE-008 implementation.
///
/// **`Default` / `PartialEq`:** Exposed so integration tests (SER-001 bincode round-trip) can construct and compare
/// placeholders without reaching private fields; removed or narrowed when real fields land ([EXE-008](docs/requirements/domains/execution_validation/specs/EXE-008.md)).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    _placeholder: (),
}
