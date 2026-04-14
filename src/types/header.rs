//! `L2BlockHeader` — independently hashable L2 block metadata and commitments.
//!
//! **Requirement:** [BLK-001](docs/requirements/domains/block_types/specs/BLK-001.md) /
//! [NORMATIVE § BLK-001](docs/requirements/domains/block_types/NORMATIVE.md#blk-001-l2blockheader-struct) /
//! [SPEC §2.2](docs/resources/SPEC.md).
//!
//! ## Usage
//!
//! Headers are constructed via the public constructors in BLK-002 (`new`, `new_with_collateral`, …).
//! For BLK-001, use struct literals in tests or internal call sites until those APIs exist. Field order
//! matches SPEC §2.2 so future **bincode** layout (SER-001, HSH-001) stays deterministic.
//!
//! ## Rationale
//!
//! Splitting header from body ([`super::block::L2Block`], BLK-003) mirrors an Ethereum-style header/body
//! split: attestations and light clients can process headers without deserializing `SpendBundle` payloads.
//!
//! ## Decisions
//!
//! - **`Bytes32`** and **`Cost`** come from [`crate::primitives`] so this crate has one type identity for
//!   hashes and CLVM cost (BLK-006).
//! - **L1 proof anchors** are `Option<Bytes32>`; omitted proofs serialize as `None` (default) per SPEC.
//! - **DFSP roots** are mandatory `Bytes32` fields; pre-activation they are set to [`crate::EMPTY_ROOT`]
//!   by constructors / validation (SVL-002), not by the type itself.

use serde::{Deserialize, Serialize};

use crate::primitives::{Bytes32, Cost};

/// DIG L2 block header: identity, Merkle commitments, L1 anchor, metadata, optional L1 proofs, slash
/// proposal commitments, and DFSP data-layer roots.
///
/// Field layout and semantics follow SPEC §2.2 table **Field groups**; keep this definition in sync with
/// that section when the wire format evolves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct L2BlockHeader {
    // ── Core identity ──
    /// Protocol version (`VERSION_V1` / `VERSION_V2`; BLK-007 selects from height).
    pub version: u16,
    /// Block height (genesis = 0).
    pub height: u64,
    /// Epoch number.
    pub epoch: u64,
    /// Hash of the parent block header.
    pub parent_hash: Bytes32,

    // ── State commitments ──
    /// CoinSet / state Merkle root after this block.
    pub state_root: Bytes32,
    /// Merkle root of spend-bundle hashes.
    pub spends_root: Bytes32,
    /// Merkle root of additions (Chia-style grouping by `puzzle_hash`).
    pub additions_root: Bytes32,
    /// Merkle root of removed coin IDs.
    pub removals_root: Bytes32,
    /// Merkle root of execution receipts.
    pub receipts_root: Bytes32,

    // ── L1 anchor ──
    /// Chia L1 block height this L2 block references.
    pub l1_height: u32,
    /// Chia L1 block hash.
    pub l1_hash: Bytes32,

    // ── Block metadata ──
    /// Unix timestamp (seconds).
    pub timestamp: u64,
    /// Proposer validator index.
    pub proposer_index: u32,
    /// Number of spend bundles in the block body.
    pub spend_bundle_count: u32,
    /// Aggregate CLVM cost of all spends in the block.
    pub total_cost: Cost,
    /// Total fees (value in − value out).
    pub total_fees: u64,
    /// Number of coin additions.
    pub additions_count: u32,
    /// Number of coin removals.
    pub removals_count: u32,
    /// Serialized full block size in bytes (header + body).
    pub block_size: u32,
    /// BIP158-style compact block filter hash.
    pub filter_hash: Bytes32,
    /// Reserved extension field (SPEC: default `ZERO_HASH` in constructors).
    pub extension_data: Bytes32,

    // ── L1 proof anchors ──
    /// Proposer L1 collateral proof coin id.
    #[serde(default)]
    pub l1_collateral_coin_id: Option<Bytes32>,
    /// Network validator collateral set anchor.
    #[serde(default)]
    pub l1_reserve_coin_id: Option<Bytes32>,
    /// Previous epoch finalization proof.
    #[serde(default)]
    pub l1_prev_epoch_finalizer_coin_id: Option<Bytes32>,
    /// Current epoch finalizer state.
    #[serde(default)]
    pub l1_curr_epoch_finalizer_coin_id: Option<Bytes32>,
    /// Network singleton existence proof.
    #[serde(default)]
    pub l1_network_coin_id: Option<Bytes32>,

    // ── Slash proposals ──
    /// Number of slash proposal payloads in the body.
    pub slash_proposal_count: u32,
    /// Merkle root over per-proposal hashes.
    pub slash_proposals_root: Bytes32,

    // ── DFSP data layer roots ──
    /// Collateral registry sparse Merkle root.
    pub collateral_registry_root: Bytes32,
    /// CID lifecycle state machine root.
    pub cid_state_root: Bytes32,
    /// Node registry sparse Merkle root.
    pub node_registry_root: Bytes32,
    /// Namespace update delta root for this block.
    pub namespace_update_root: Bytes32,
    /// DFSP epoch-boundary commitment digest.
    pub dfsp_finalize_commitment_root: Bytes32,
}
