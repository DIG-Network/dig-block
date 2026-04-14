//! [`AttestedBlock`] — L2 block plus attestation metadata (signers, aggregate sig, receipts, status).
//!
//! ## Requirements trace
//!
//! - **[ATT-001](docs/requirements/domains/attestation/specs/ATT-001.md)** — struct fields + [`AttestedBlock::new`].
//! - **[ATT-002](docs/requirements/domains/attestation/specs/ATT-002.md)** (next) — `signing_percentage`, `has_soft_finality`, `hash` delegation.
//! - **[NORMATIVE § ATT-001](docs/requirements/domains/attestation/NORMATIVE.md)** — field types and constructor invariants.
//! - **[SPEC §2.4](docs/resources/SPEC.md)** — wire / semantic context for attested payloads.
//!
//! ## Usage
//!
//! Wrap a finalized [`crate::L2Block`] once execution produces [`crate::ReceiptList`] entries; [`AttestedBlock::new`]
//! seeds [`Self::aggregate_signature`] with the **proposer** signature ([`L2Block::proposer_signature`]) before
//! validators aggregate their BLS shares (implementation notes in ATT-001). [`Self::signer_bitmap`] starts
//! empty; consensus layers record attestations via [`SignerBitmap::set_signed`](crate::SignerBitmap::set_signed)
//! / [`SignerBitmap::merge`](crate::SignerBitmap::merge) (ATT-004 / ATT-005).
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
use crate::primitives::Signature;

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
}
