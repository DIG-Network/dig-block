//! Receipt domain types: [`ReceiptStatus`], [`Receipt`], [`ReceiptList`].
//!
//! ## Requirements trace
//!
//! - **[RCP-001](docs/requirements/domains/receipt/specs/RCP-001.md)** — [`ReceiptStatus`] discriminants `0..=4` and `255`.
//! - **[RCP-002](docs/requirements/domains/receipt/specs/RCP-002.md)** — [`Receipt`] field layout (tx id, height, index, status, fees, state).
//! - **[NORMATIVE](docs/requirements/domains/receipt/NORMATIVE.md)** — receipt domain obligations.
//! - **[RCP-003](docs/requirements/domains/receipt/specs/RCP-003.md)** — [`ReceiptList`]: storage, Merkle [`ReceiptList::root`], accessors.
//! - **[RCP-004](docs/requirements/domains/receipt/specs/RCP-004.md)** (next) — aggregate helpers on [`ReceiptList`].
//! - **[SPEC §2.9](docs/resources/SPEC.md)** — receipt payload context.
//! - **[HSH-008](docs/requirements/domains/hashing/specs/HSH-008.md)** — receipts Merkle algorithm (same as this module’s root helper; see note below).
//!
//! ## Rationale
//!
//! - **`#[repr(u8)]`:** Stable single-byte tags for bincode payloads and receipt Merkle leaves ([RCP-001](docs/requirements/domains/receipt/specs/RCP-001.md) implementation notes).
//! - **`Failed = 255`:** Leaves `5..=254` for future specific failure codes without renumbering existing wire values.
//! - **`ReceiptStatus::from_u8`:** Unknown bytes map to [`ReceiptStatus::Failed`] so forward-compatible decoders never panic (RCP-001 implementation notes).
//! - **`ReceiptList::push` without immediate root update:** Batch amortization per [RCP-003](docs/requirements/domains/receipt/specs/RCP-003.md); callers must [`ReceiptList::finalize`] (or use [`ReceiptList::from_receipts`]).
//! - **`compute_receipts_root` location:** Implemented privately in this file to match [HSH-008](docs/requirements/domains/hashing/specs/HSH-008.md) while avoiding a `crate::hash` ↔ `types::receipt` import cycle. [HSH-008](docs/requirements/domains/hashing/specs/HSH-008.md) may relocate the symbol to [`crate::hash`](crate::hash) when that module owns root helpers.

use chia_sdk_types::MerkleTree;
use chia_sha2::Sha256;
use serde::{Deserialize, Serialize};

use crate::constants::EMPTY_ROOT;
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

/// Merkle root over ordered receipts: SHA-256(bincode(`Receipt`)) per leaf, then [`MerkleTree`] ([HSH-008](docs/requirements/domains/hashing/specs/HSH-008.md)).
///
/// **Empty list:** [`EMPTY_ROOT`] ([BLK-005](docs/requirements/domains/block_types/specs/BLK-005.md)).
///
/// **Tagged hashing:** [`MerkleTree`] applies leaf/node domain separation per [HSH-007](docs/requirements/domains/hashing/specs/HSH-007.md) (inherited from `chia-sdk-types`).
fn compute_receipts_root(receipts: &[Receipt]) -> Bytes32 {
    if receipts.is_empty() {
        return EMPTY_ROOT;
    }
    let hashes: Vec<Bytes32> = receipts
        .iter()
        .map(|r| {
            let bytes =
                bincode::serialize(r).expect("Receipt bincode serialization should not fail");
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            Bytes32::new(hasher.finalize())
        })
        .collect();
    MerkleTree::new(&hashes).root()
}

/// Ordered block receipts with a commitments root ([RCP-003](docs/requirements/domains/receipt/specs/RCP-003.md), SPEC §2.9).
///
/// **Wire:** [`Serialize`] / [`Deserialize`] include both `receipts` and `root`; consumers should re-validate or
/// call [`Self::finalize`] after deserializing if they distrust the stored root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptList {
    /// Receipts in **block order** (must align with `tx_index` / execution order).
    pub receipts: Vec<Receipt>,
    /// Merkle root over [`Self::receipts`] — see [`Self::finalize`].
    pub root: Bytes32,
}

impl Default for ReceiptList {
    /// Same as [`Self::new`] — keeps [`crate::AttestedBlock::new`] and tests working with [`Default::default`].
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiptList {
    /// Empty list, root = [`EMPTY_ROOT`] ([RCP-003](docs/requirements/domains/receipt/specs/RCP-003.md) `new()`).
    #[must_use]
    pub fn new() -> Self {
        Self {
            receipts: Vec::new(),
            root: EMPTY_ROOT,
        }
    }

    /// Take ownership of `receipts` and set [`Self::root`] via [`compute_receipts_root`].
    #[must_use]
    pub fn from_receipts(receipts: Vec<Receipt>) -> Self {
        let root = compute_receipts_root(&receipts);
        Self { receipts, root }
    }

    /// Append a receipt **without** updating [`Self::root`] — call [`Self::finalize`] when done ([RCP-003](docs/requirements/domains/receipt/specs/RCP-003.md)).
    pub fn push(&mut self, receipt: Receipt) {
        self.receipts.push(receipt);
    }

    /// Recompute [`Self::root`] from the current [`Self::receipts`] vector.
    pub fn finalize(&mut self) {
        self.root = compute_receipts_root(&self.receipts);
    }

    /// Borrow receipt at `index`, or `None` if out of bounds.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Receipt> {
        self.receipts.get(index)
    }

    /// First receipt whose [`Receipt::tx_id`] matches, or `None` ([RCP-003](docs/requirements/domains/receipt/specs/RCP-003.md) — linear scan).
    #[must_use]
    pub fn get_by_tx_id(&self, tx_id: Bytes32) -> Option<&Receipt> {
        self.receipts.iter().find(|r| r.tx_id == tx_id)
    }
}
