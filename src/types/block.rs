//! `L2Block` — full L2 block: header, transaction body (`SpendBundle`s), slash proposal payloads, proposer signature.
//!
//! **Requirements:**
//! - [BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md) — struct + `new` / `hash` / `height` / `epoch`
//! - [SPEC §2.3](docs/resources/SPEC.md) — header/body split; block identity from header ([HSH-001](docs/requirements/domains/hashing/specs/HSH-001.md))
//!
//! ## Usage
//!
//! Build a block by assembling an [`L2BlockHeader`] (commitments, roots, counts) and the body fields.
//! **Canonical identity** is [`L2Block::hash`] → [`L2BlockHeader::hash`] only; spend bundles and slash
//! bytes are committed via Merkle roots **in the header**, not mixed into this hash (SPEC §2.3 / BLK-003 notes).
//!
//! ## Rationale
//!
//! - **`SpendBundle`** comes from **`chia-protocol`** so CLVM spends match L1/Chia tooling ([BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md)).
//! - **`Signature`** is the **`chia-bls`** type re-exported as [`crate::primitives::Signature`] ([BLK-006](docs/requirements/domains/block_types/specs/BLK-006.md)) so callers import one `dig_block` surface.
//! - **`slash_proposal_payloads`** are `Vec<Vec<u8>>` for opaque slash evidence (encoding evolves independently).

use chia_protocol::SpendBundle;
use serde::{Deserialize, Serialize};

use super::header::L2BlockHeader;
use crate::primitives::{Bytes32, Signature};

/// Complete L2 block: header plus body (spend bundles, slash payloads) and proposer attestation.
///
/// See [BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md) and [`SPEC §2.3`](docs/resources/SPEC.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Block {
    /// Block header (identity hash, Merkle roots, metadata).
    pub header: L2BlockHeader,
    /// Spend bundles included in this block (`chia-protocol`).
    pub spend_bundles: Vec<SpendBundle>,
    /// Raw slash proposal payloads (count should align with header slash fields when validated).
    pub slash_proposal_payloads: Vec<Vec<u8>>,
    /// BLS signature over the block from the proposer ([`crate::primitives::Signature`] / `chia-bls`).
    pub proposer_signature: Signature,
}

impl L2Block {
    /// Construct a block from all body fields and the header ([BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md) `new()`).
    ///
    /// **Note:** Callers must keep `header` fields (e.g. `spend_bundle_count`, Merkle roots) consistent with
    /// `spend_bundles` / `slash_proposal_payloads`; structural validation is separate (ERR-* / VAL-* requirements).
    pub fn new(
        header: L2BlockHeader,
        spend_bundles: Vec<SpendBundle>,
        slash_proposal_payloads: Vec<Vec<u8>>,
        proposer_signature: Signature,
    ) -> Self {
        Self {
            header,
            spend_bundles,
            slash_proposal_payloads,
            proposer_signature,
        }
    }

    /// Canonical block identity: SHA-256 over the header preimage only ([`L2BlockHeader::hash`], HSH-001 / SPEC §3.1).
    ///
    /// **Delegation:** identical to `self.header.hash()` — required by BLK-003 so light clients and
    /// signers can treat the header hash as the block id without serializing the body.
    #[inline]
    pub fn hash(&self) -> Bytes32 {
        self.header.hash()
    }

    /// Block height from the header ([`L2BlockHeader::height`]).
    #[inline]
    pub fn height(&self) -> u64 {
        self.header.height
    }

    /// Epoch from the header ([`L2BlockHeader::epoch`]).
    #[inline]
    pub fn epoch(&self) -> u64 {
        self.header.epoch
    }
}
