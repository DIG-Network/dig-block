//! [`AttestedBlock`] — L2 block plus attestation metadata (signers, aggregate sig, receipts, status).
//!
//! ## Requirements trace
//!
//! - **[ATT-001](docs/requirements/domains/attestation/specs/ATT-001.md)** — struct fields + [`AttestedBlock::new`].
//! - **[ATT-002](docs/requirements/domains/attestation/specs/ATT-002.md)** — [`AttestedBlock::signing_percentage`],
//!   [`AttestedBlock::has_soft_finality`], [`AttestedBlock::hash`] (delegate to [`SignerBitmap`] / [`L2Block::hash`]).
//! - **[SER-002](docs/requirements/domains/serialization/specs/SER-002.md)** — [`Self::to_bytes`] / [`Self::from_bytes`] (bincode; decode errors → [`BlockError::InvalidData`](crate::BlockError::InvalidData)).
//! - **[NORMATIVE § ATT-001 / ATT-002](docs/requirements/domains/attestation/NORMATIVE.md)** — constructor + query API.
//! - **[SPEC §2.4](docs/resources/SPEC.md)** — wire / semantic context for attested payloads.
//!
//! ## Usage
//!
//! Wrap a finalized [`crate::L2Block`] once execution produces [`crate::ReceiptList`] entries; [`AttestedBlock::new`]
//! seeds [`Self::aggregate_signature`] with the **proposer** signature ([`L2Block::proposer_signature`]) before
//! validators aggregate their BLS shares (implementation notes in ATT-001). [`Self::signer_bitmap`] starts
//! empty; consensus layers record attestations via [`SignerBitmap::set_signed`](crate::SignerBitmap::set_signed)
//! / [`SignerBitmap::merge`](crate::SignerBitmap::merge) (ATT-004 / ATT-005). Use [`Self::signing_percentage`] and
//! [`Self::has_soft_finality`] for quorum checks; [`Self::hash`] is the same [`Bytes32`] as [`L2Block::hash`] (ATT-002).
//!
//! ## Rationale
//!
//! - **Initial aggregate = proposer:** Matches ATT-001 / NORMATIVE — there is no separate “epoch genesis”
//!   signature slot; the proposer’s block signature bootstraps the aggregate until attesters merge in.
//! - **Status `Pending`:** Constructor does not imply validation; structural / execution tiers advance status
//!   outside this crate (see [`BlockStatus`](crate::BlockStatus)).

use serde::{Deserialize, Serialize};

use super::block::L2Block;
use super::receipt::ReceiptList;
use super::signer_bitmap::SignerBitmap;
use super::status::BlockStatus;
use crate::error::BlockError;
use crate::primitives::{Bytes32, Signature};

/// L2 block wrapped with attestation state: who signed, aggregate BLS signature, receipts, lifecycle status.
///
/// **Fields (NORMATIVE ATT-001):** See [ATT-001](docs/requirements/domains/attestation/specs/ATT-001.md) table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestedBlock {
    /// Underlying L2 block (header hash is canonical block id via [`L2Block::hash`]).
    pub block: L2Block,
    /// Which validators have attested (bit-packed, sized by epoch validator set).
    pub signer_bitmap: SignerBitmap,
    /// Rolling aggregate BLS signature over attester contributions; starts as the proposer signature.
    pub aggregate_signature: Signature,
    /// Execution receipts for spends in this block ([`ReceiptList`]; fleshed out in RCP-003/004).
    pub receipts: ReceiptList,
    /// Attestation / finality pipeline state ([`BlockStatus`]).
    pub status: BlockStatus,
}

impl AttestedBlock {
    /// Build an attested wrapper: empty bitmap, [`BlockStatus::Pending`], aggregate sig = proposer’s signature.
    ///
    /// **Invariants (ATT-001):**
    /// - [`Self::signer_bitmap`] is [`SignerBitmap::new`](crate::SignerBitmap::new)(`validator_count`) → zero signers.
    /// - [`Self::status`] is [`BlockStatus::Pending`].
    /// - [`Self::aggregate_signature`] is a **clone** of [`L2Block::proposer_signature`].
    /// - [`Self::block`] and [`Self::receipts`] move in from the caller.
    ///
    /// **Panics:** If `validator_count > MAX_VALIDATORS` — same cap as [`crate::SignerBitmap::new`].
    pub fn new(block: L2Block, validator_count: u32, receipts: ReceiptList) -> Self {
        let aggregate_signature = block.proposer_signature.clone();
        Self {
            signer_bitmap: SignerBitmap::new(validator_count),
            aggregate_signature,
            status: BlockStatus::Pending,
            block,
            receipts,
        }
    }

    /// Integer signing progress `0..=100` — thin wrapper over [`SignerBitmap::signing_percentage`].
    ///
    /// **ATT-002:** Keeps quorum logic on one type (`SignerBitmap` math in ATT-004) while exposing a stable
    /// [`AttestedBlock`] surface for consensus code ([NORMATIVE § ATT-002](docs/requirements/domains/attestation/NORMATIVE.md)).
    #[inline]
    #[must_use]
    pub fn signing_percentage(&self) -> u64 {
        self.signer_bitmap.signing_percentage()
    }

    /// `true` iff [`Self::signing_percentage`] `>= threshold_pct` (soft-finality / stake-threshold gate).
    ///
    /// **ATT-002:** Delegates to [`SignerBitmap::has_threshold`] so boundary behavior matches ATT-004 tests
    /// (integer division, `validator_count == 0` → `0%`).
    #[inline]
    #[must_use]
    pub fn has_soft_finality(&self, threshold_pct: u64) -> bool {
        self.signer_bitmap.has_threshold(threshold_pct)
    }

    /// Block identity: SHA-256 header preimage hash ([`L2Block::hash`], HSH-001 / SPEC §2.3).
    ///
    /// **ATT-002 / NORMATIVE:** MUST equal `self.block.hash()` so attested and raw blocks share one id in indices,
    /// checkpoints, and P2P — no second hash domain for the attestation wrapper.
    #[inline]
    #[must_use]
    pub fn hash(&self) -> Bytes32 {
        self.block.hash()
    }

    /// Serialize attested block (inner [`L2Block`] + bitmap + signatures + receipts + status) to **bincode** ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md)).
    ///
    /// **Infallible:** Mirrors [`L2Block::to_bytes`] — struct must encode; panics only on invariant violation.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("AttestedBlock serialization should never fail")
    }

    /// Deserialize from **bincode** bytes ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md)).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BlockError> {
        bincode::deserialize(bytes).map_err(|e| BlockError::InvalidData(e.to_string()))
    }
}
