//! Receipt domain types: [`ReceiptStatus`], [`Receipt`], [`ReceiptList`].
//!
//! ## Requirements trace
//!
//! - **[RCP-001](docs/requirements/domains/receipt/specs/RCP-001.md)** — [`ReceiptStatus`] discriminants `0..=4` and `255`.
//! - **[RCP-002](docs/requirements/domains/receipt/specs/RCP-002.md)** — [`Receipt`] field layout (tx id, height, index, status, fees, state).
//! - **[NORMATIVE](docs/requirements/domains/receipt/NORMATIVE.md)** — RCP-001 / RCP-002 obligations.
//! - **[SPEC §2.9](docs/resources/SPEC.md)** — receipt payload context.
//! - **RCP-003 — RCP-004:** [`ReceiptList`] storage and aggregates land in later specs.
//!
//! ## Rationale
//!
//! - **`#[repr(u8)]`:** Stable single-byte tags for bincode payloads and receipt Merkle leaves ([RCP-001](docs/requirements/domains/receipt/specs/RCP-001.md) implementation notes).
//! - **`Failed = 255`:** Leaves `5..=254` for future specific failure codes without renumbering existing wire values.
//! - **`ReceiptStatus::from_u8`:** Unknown bytes map to [`ReceiptStatus::Failed`] so forward-compatible decoders never panic (RCP-001 implementation notes).

use serde::{Deserialize, Serialize};

use crate::primitives::Bytes32;

/// Outcome of applying one transaction in a block ([SPEC §2.9](docs/resources/SPEC.md), RCP-001).
///
/// **Wire:** Use [`Self::as_u8`] / [`Self::from_u8`] for deterministic `u8` ↔ enum mapping; serde derives are
/// retained for schema evolution ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)) and may be
/// tuned in SER-* tasks for integer tagging.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReceiptStatus {
    /// Transaction executed successfully.
    Success = 0,
    /// Sender balance insufficient for the transaction cost.
    InsufficientBalance = 1,
    /// Nonce did not match account sequence.
    InvalidNonce = 2,
    /// Cryptographic signature verification failed.
    InvalidSignature = 3,
    /// Sender account missing from state.
    AccountNotFound = 4,
    /// Generic or reserved-range execution failure (`255` — see module docs).
    Failed = 255,
}

impl ReceiptStatus {
    /// Discriminant as a single byte (same as `self as u8` with [`#[repr(u8)]`](ReceiptStatus)).
    #[inline]
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Decode a wire / stored byte into [`ReceiptStatus`].
    ///
    /// **Unknown values:** Any byte other than `0..=4` or `255` maps to [`Self::Failed`] so new failure codes
    /// can be introduced later without breaking old decoders (they will classify unknowns as failed execution).
    #[must_use]
    pub fn from_u8(byte: u8) -> Self {
        match byte {
            0 => Self::Success,
            1 => Self::InsufficientBalance,
            2 => Self::InvalidNonce,
            3 => Self::InvalidSignature,
            4 => Self::AccountNotFound,
            255 => Self::Failed,
            _ => Self::Failed,
        }
    }
}

/// Result of executing one transaction inside a block ([RCP-002](docs/requirements/domains/receipt/specs/RCP-002.md), SPEC §2.9).
///
/// ## Field semantics
///
/// - **`tx_id`:** Transaction hash this receipt attests to (often spend-bundle / tx commitment — exact preimage in HSH-*).
/// - **`tx_index`:** Zero-based position in block body (RCP-002 implementation notes).
/// - **`post_state_root`:** State trie root **after** this tx; enables per-tx light-client checkpoints.
/// - **`cumulative_fees`:** Running sum of `fee_charged` for receipts `0..=tx_index` in the same block; execution must keep this
///   consistent when appending receipts ([RCP-002](docs/requirements/domains/receipt/specs/RCP-002.md) implementation notes).
///
/// **Serialization:** [`Serialize`] / [`Deserialize`] for bincode ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Hash identifying the executed transaction.
    pub tx_id: Bytes32,
    /// Block height containing this transaction.
    pub block_height: u64,
    /// Zero-based transaction index within the block.
    pub tx_index: u32,
    /// Execution outcome ([`ReceiptStatus`], RCP-001).
    pub status: ReceiptStatus,
    /// Fee debited for this transaction.
    pub fee_charged: u64,
    /// State root after applying this transaction.
    pub post_state_root: Bytes32,
    /// Sum of `fee_charged` for all receipts up to and including this one in the block.
    pub cumulative_fees: u64,
}

impl Receipt {
    /// Construct a receipt with all NORMATIVE fields ([RCP-002](docs/requirements/domains/receipt/specs/RCP-002.md)).
    ///
    /// **Note:** Callers must ensure `cumulative_fees` matches the monotonic fee aggregate for the block; this crate does not
    /// recompute it here (single-receipt constructor only).
    pub fn new(
        tx_id: Bytes32,
        block_height: u64,
        tx_index: u32,
        status: ReceiptStatus,
        fee_charged: u64,
        post_state_root: Bytes32,
        cumulative_fees: u64,
    ) -> Self {
        Self {
            tx_id,
            block_height,
            tx_index,
            status,
            fee_charged,
            post_state_root,
            cumulative_fees,
        }
    }
}

/// Ordered list of receipts with a Merkle root.
///
/// **ATT-001:** [`Default`] yields an empty placeholder so [`crate::AttestedBlock::new`] can be tested before
/// [RCP-003](docs/requirements/domains/receipt/specs/RCP-003.md) fills in real storage APIs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptList {
    _placeholder: (),
}
