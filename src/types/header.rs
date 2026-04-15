//! `L2BlockHeader` — independently hashable L2 block metadata and commitments.
//!
//! **Requirements:**
//! - [BLK-001](docs/requirements/domains/block_types/specs/BLK-001.md) — field groups /
//!   [NORMATIVE § BLK-001](docs/requirements/domains/block_types/NORMATIVE.md#blk-001-l2blockheader-struct)
//! - [BLK-002](docs/requirements/domains/block_types/specs/BLK-002.md) — constructors /
//!   [NORMATIVE § BLK-002](docs/requirements/domains/block_types/NORMATIVE.md#blk-002-l2blockheader-constructors)
//! - [BLK-007](docs/requirements/domains/block_types/specs/BLK-007.md) — version auto-detection /
//!   [NORMATIVE](docs/requirements/domains/block_types/NORMATIVE.md) (BLK-007)
//! - [SVL-001](docs/requirements/domains/structural_validation/specs/SVL-001.md) — header `version` vs height / DFSP activation ([`L2BlockHeader::validate`])
//! - [HSH-001](docs/requirements/domains/hashing/specs/HSH-001.md) — header `hash()` (SPEC §3.1 field order;
//!   preimage length [`L2BlockHeader::HASH_PREIMAGE_LEN`])
//! - [SPEC §2.2](docs/resources/SPEC.md), [SPEC §8.3 Genesis](docs/resources/SPEC.md#83-genesis-block)
//!
//! ## Usage
//!
//! Prefer [`L2BlockHeader::new`], [`L2BlockHeader::new_with_collateral`], [`L2BlockHeader::new_with_l1_proofs`],
//! or [`L2BlockHeader::genesis`] so **`version` is never caller-supplied** (auto-detected from `height`;
//! shared rules in [`L2BlockHeader::protocol_version_for_height`] and
//! [`L2BlockHeader::protocol_version_for_height_with_activation`] (BLK-007). Production code
//! that needs wall-clock timestamps should set `timestamp` after `new()` or use [`crate::builder::BlockBuilder`]
//! (BLD-005): [`L2BlockHeader::new`] leaves `timestamp` at **0** per SPEC’s derived-`new()` parameter list.
//!
//! Field order matches SPEC §2.2 so **bincode** layout stays deterministic (SER-001, HSH-001).
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

use std::time::{SystemTime, UNIX_EPOCH};

use chia_sha2::Sha256;
use chia_streamable_macro::Streamable;
use serde::{Deserialize, Serialize};

use crate::constants::{DFSP_ACTIVATION_HEIGHT, EMPTY_ROOT, ZERO_HASH};
use crate::error::BlockError;
use crate::primitives::{Bytes32, Cost, VERSION_V1, VERSION_V2};

/// DIG L2 block header: identity, Merkle commitments, L1 anchor, metadata, optional L1 proofs, slash
/// proposal commitments, and DFSP data-layer roots.
///
/// Field layout and semantics follow SPEC §2.2 table **Field groups**; keep this definition in sync with
/// that section when the wire format evolves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Streamable)]
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

impl L2BlockHeader {
    /// Protocol version for `height` given an explicit DFSP activation height (BLK-007).
    ///
    /// **Rules:** If `dfsp_activation_height == u64::MAX` (DFSP disabled sentinel), returns [`VERSION_V1`].
    /// Otherwise returns [`VERSION_V2`] when `height >= dfsp_activation_height`, else [`VERSION_V1`].
    ///
    /// **Rationale:** Parameterizing activation height lets tests cover pre/post-fork behavior without
    /// recompiling [`DFSP_ACTIVATION_HEIGHT`](crate::constants::DFSP_ACTIVATION_HEIGHT). Production code
    /// should call [`Self::protocol_version_for_height`] instead.
    #[inline]
    pub fn protocol_version_for_height_with_activation(
        height: u64,
        dfsp_activation_height: u64,
    ) -> u16 {
        if dfsp_activation_height == u64::MAX {
            VERSION_V1
        } else if height >= dfsp_activation_height {
            VERSION_V2
        } else {
            VERSION_V1
        }
    }

    /// Protocol version for `height` using the crate’s [`DFSP_ACTIVATION_HEIGHT`] constant.
    #[inline]
    pub fn protocol_version_for_height(height: u64) -> u16 {
        Self::protocol_version_for_height_with_activation(height, DFSP_ACTIVATION_HEIGHT)
    }

    /// **SVL-001 / SPEC §5.1 Step 1:** ensure [`L2BlockHeader::version`] matches the protocol rule for this header’s
    /// [`L2BlockHeader::height`] and an explicit DFSP activation height.
    ///
    /// **Algorithm:** `expected = `[`Self::protocol_version_for_height_with_activation`]`(height, dfsp_activation_height)`;
    /// reject with [`BlockError::InvalidVersion`] when `version != expected` ([structural_validation NORMATIVE](docs/requirements/domains/structural_validation/NORMATIVE.md#svl-001-header-version-check)).
    ///
    /// **Production:** Prefer [`Self::validate`], which passes [`DFSP_ACTIVATION_HEIGHT`](crate::constants::DFSP_ACTIVATION_HEIGHT)
    /// (BLK-005 sentinel `u64::MAX` ⇒ always expect [`VERSION_V1`](crate::primitives::VERSION_V1) until governance changes the constant).
    /// This method stays **public** so tests can simulate a finite fork height without recompiling the crate.
    pub fn validate_with_dfsp_activation(
        &self,
        dfsp_activation_height: u64,
    ) -> Result<(), BlockError> {
        let expected =
            Self::protocol_version_for_height_with_activation(self.height, dfsp_activation_height);
        if self.version != expected {
            return Err(BlockError::InvalidVersion {
                expected,
                actual: self.version,
            });
        }
        Ok(())
    }

    /// Tier 1 header structural validation using crate-wide constants ([SVL-*](docs/requirements/domains/structural_validation/NORMATIVE.md)).
    ///
    /// **Current steps:** [SVL-001](docs/requirements/domains/structural_validation/specs/SVL-001.md) only — additional
    /// header checks (DFSP roots, cost/size, timestamp) will chain here in implementation order.
    pub fn validate(&self) -> Result<(), BlockError> {
        self.validate_with_dfsp_activation(DFSP_ACTIVATION_HEIGHT)?;
        Ok(())
    }

    /// Byte length of the fixed preimage fed to [`Self::hash`] (all 33 rows of [SPEC §3.1](docs/resources/SPEC.md)).
    ///
    /// **Accounting:** 20×[`Bytes32`] fields + `u16` + 6×`u64` + 7×`u32` = 640 + 70 = **710** bytes.
    /// The SPEC prose once said “626 bytes”; summing the §3.1 table yields **710** — this constant is authoritative for code.
    pub const HASH_PREIMAGE_LEN: usize = 710;

    /// Serialize the exact **710-byte** preimage for [HSH-001](docs/requirements/domains/hashing/specs/HSH-001.md) /
    /// [SPEC §3.1](docs/resources/SPEC.md) (same order as [`Self::hash`]).
    ///
    /// **Usage:** Tests and debug tooling can diff preimages without re-deriving field order; [`Self::hash`] is
    /// `SHA-256(self.hash_preimage_bytes())`.
    ///
    /// **Optionals:** Each `Option<Bytes32>` occupies 32 bytes: [`ZERO_HASH`] when `None`, raw bytes when `Some`.
    pub fn hash_preimage_bytes(&self) -> [u8; Self::HASH_PREIMAGE_LEN] {
        fn put(buf: &mut [u8; L2BlockHeader::HASH_PREIMAGE_LEN], i: &mut usize, bytes: &[u8]) {
            buf[*i..*i + bytes.len()].copy_from_slice(bytes);
            *i += bytes.len();
        }
        fn put_opt(
            buf: &mut [u8; L2BlockHeader::HASH_PREIMAGE_LEN],
            i: &mut usize,
            o: &Option<Bytes32>,
        ) {
            let slice = match o {
                Some(b) => b.as_ref(),
                None => ZERO_HASH.as_ref(),
            };
            buf[*i..*i + 32].copy_from_slice(slice);
            *i += 32;
        }
        let mut buf = [0u8; Self::HASH_PREIMAGE_LEN];
        let mut i = 0usize;
        put(&mut buf, &mut i, &self.version.to_le_bytes());
        put(&mut buf, &mut i, &self.height.to_le_bytes());
        put(&mut buf, &mut i, &self.epoch.to_le_bytes());
        put(&mut buf, &mut i, self.parent_hash.as_ref());
        put(&mut buf, &mut i, self.state_root.as_ref());
        put(&mut buf, &mut i, self.spends_root.as_ref());
        put(&mut buf, &mut i, self.additions_root.as_ref());
        put(&mut buf, &mut i, self.removals_root.as_ref());
        put(&mut buf, &mut i, self.receipts_root.as_ref());
        put(&mut buf, &mut i, &self.l1_height.to_le_bytes());
        put(&mut buf, &mut i, self.l1_hash.as_ref());
        put(&mut buf, &mut i, &self.timestamp.to_le_bytes());
        put(&mut buf, &mut i, &self.proposer_index.to_le_bytes());
        put(&mut buf, &mut i, &self.spend_bundle_count.to_le_bytes());
        put(&mut buf, &mut i, &self.total_cost.to_le_bytes());
        put(&mut buf, &mut i, &self.total_fees.to_le_bytes());
        put(&mut buf, &mut i, &self.additions_count.to_le_bytes());
        put(&mut buf, &mut i, &self.removals_count.to_le_bytes());
        put(&mut buf, &mut i, &self.block_size.to_le_bytes());
        put(&mut buf, &mut i, self.filter_hash.as_ref());
        put(&mut buf, &mut i, self.extension_data.as_ref());
        put_opt(&mut buf, &mut i, &self.l1_collateral_coin_id);
        put_opt(&mut buf, &mut i, &self.l1_reserve_coin_id);
        put_opt(&mut buf, &mut i, &self.l1_prev_epoch_finalizer_coin_id);
        put_opt(&mut buf, &mut i, &self.l1_curr_epoch_finalizer_coin_id);
        put_opt(&mut buf, &mut i, &self.l1_network_coin_id);
        put(&mut buf, &mut i, &self.slash_proposal_count.to_le_bytes());
        put(&mut buf, &mut i, self.slash_proposals_root.as_ref());
        put(&mut buf, &mut i, self.collateral_registry_root.as_ref());
        put(&mut buf, &mut i, self.cid_state_root.as_ref());
        put(&mut buf, &mut i, self.node_registry_root.as_ref());
        put(&mut buf, &mut i, self.namespace_update_root.as_ref());
        put(
            &mut buf,
            &mut i,
            self.dfsp_finalize_commitment_root.as_ref(),
        );
        debug_assert_eq!(i, Self::HASH_PREIMAGE_LEN);
        buf
    }

    /// Canonical block identity: SHA-256 over [`Self::hash_preimage_bytes`] ([HSH-001](docs/requirements/domains/hashing/specs/HSH-001.md)).
    ///
    /// **Requirement:** [SPEC §3.1](docs/resources/SPEC.md). Numeric fields are little-endian; each optional L1 anchor
    /// contributes 32 bytes of raw [`Bytes32`] or [`ZERO_HASH`] when `None` (malleability-safe encoding).
    ///
    /// **Primitive:** [`chia_sha2::Sha256`] only ([`crate::primitives`] / project crypto rules).
    pub fn hash(&self) -> Bytes32 {
        let mut hasher = Sha256::new();
        hasher.update(self.hash_preimage_bytes());
        Bytes32::new(hasher.finalize())
    }

    /// Standard header constructor (SPEC §2.2 **Derived methods** / `new()`).
    ///
    /// Sets `version` via [`Self::protocol_version_for_height`]; `timestamp` to **0** (SPEC omits it from
    /// the `new` parameter list—set explicitly or use [`Self::genesis`] / block builder for wall clock);
    /// L1 proof anchors to `None`; slash summary to empty; DFSP roots to [`EMPTY_ROOT`]; `extension_data`
    /// to [`ZERO_HASH`].
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        height: u64,
        epoch: u64,
        parent_hash: Bytes32,
        state_root: Bytes32,
        spends_root: Bytes32,
        additions_root: Bytes32,
        removals_root: Bytes32,
        receipts_root: Bytes32,
        l1_height: u32,
        l1_hash: Bytes32,
        proposer_index: u32,
        spend_bundle_count: u32,
        total_cost: Cost,
        total_fees: u64,
        additions_count: u32,
        removals_count: u32,
        block_size: u32,
        filter_hash: Bytes32,
    ) -> Self {
        Self::with_l1_anchors(
            height,
            epoch,
            parent_hash,
            state_root,
            spends_root,
            additions_root,
            removals_root,
            receipts_root,
            l1_height,
            l1_hash,
            0,
            proposer_index,
            spend_bundle_count,
            total_cost,
            total_fees,
            additions_count,
            removals_count,
            block_size,
            filter_hash,
            ZERO_HASH,
            None,
            None,
            None,
            None,
            None,
            0,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
        )
    }

    /// Like [`Self::new`] but sets [`L2BlockHeader::l1_collateral_coin_id`] to the given proof coin id.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_collateral(
        height: u64,
        epoch: u64,
        parent_hash: Bytes32,
        state_root: Bytes32,
        spends_root: Bytes32,
        additions_root: Bytes32,
        removals_root: Bytes32,
        receipts_root: Bytes32,
        l1_height: u32,
        l1_hash: Bytes32,
        proposer_index: u32,
        spend_bundle_count: u32,
        total_cost: Cost,
        total_fees: u64,
        additions_count: u32,
        removals_count: u32,
        block_size: u32,
        filter_hash: Bytes32,
        l1_collateral_coin_id: Bytes32,
    ) -> Self {
        Self::with_l1_anchors(
            height,
            epoch,
            parent_hash,
            state_root,
            spends_root,
            additions_root,
            removals_root,
            receipts_root,
            l1_height,
            l1_hash,
            0,
            proposer_index,
            spend_bundle_count,
            total_cost,
            total_fees,
            additions_count,
            removals_count,
            block_size,
            filter_hash,
            ZERO_HASH,
            Some(l1_collateral_coin_id),
            None,
            None,
            None,
            None,
            0,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
        )
    }

    /// Full L1 proof anchor set (SPEC field order: collateral, reserve, prev/curr finalizer, network coin).
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_l1_proofs(
        height: u64,
        epoch: u64,
        parent_hash: Bytes32,
        state_root: Bytes32,
        spends_root: Bytes32,
        additions_root: Bytes32,
        removals_root: Bytes32,
        receipts_root: Bytes32,
        l1_height: u32,
        l1_hash: Bytes32,
        proposer_index: u32,
        spend_bundle_count: u32,
        total_cost: Cost,
        total_fees: u64,
        additions_count: u32,
        removals_count: u32,
        block_size: u32,
        filter_hash: Bytes32,
        l1_collateral_coin_id: Bytes32,
        l1_reserve_coin_id: Bytes32,
        l1_prev_epoch_finalizer_coin_id: Bytes32,
        l1_curr_epoch_finalizer_coin_id: Bytes32,
        l1_network_coin_id: Bytes32,
    ) -> Self {
        Self::with_l1_anchors(
            height,
            epoch,
            parent_hash,
            state_root,
            spends_root,
            additions_root,
            removals_root,
            receipts_root,
            l1_height,
            l1_hash,
            0,
            proposer_index,
            spend_bundle_count,
            total_cost,
            total_fees,
            additions_count,
            removals_count,
            block_size,
            filter_hash,
            ZERO_HASH,
            Some(l1_collateral_coin_id),
            Some(l1_reserve_coin_id),
            Some(l1_prev_epoch_finalizer_coin_id),
            Some(l1_curr_epoch_finalizer_coin_id),
            Some(l1_network_coin_id),
            0,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
        )
    }

    /// Genesis header (SPEC §8.3): `parent_hash = network_id`, zeroed counts/costs, empty Merkle roots.
    ///
    /// **`timestamp`:** set from `SystemTime::now()` (SPEC §8.3). Tests should assert structural fields, not
    /// an exact timestamp.
    pub fn genesis(network_id: Bytes32, l1_height: u32, l1_hash: Bytes32) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let height = 0u64;
        Self::with_l1_anchors(
            height, 0, network_id, EMPTY_ROOT, EMPTY_ROOT, EMPTY_ROOT, EMPTY_ROOT, EMPTY_ROOT,
            l1_height, l1_hash, timestamp, 0, 0, 0, 0, 0, 0, 0, EMPTY_ROOT, ZERO_HASH, None, None,
            None, None, None, 0, EMPTY_ROOT, EMPTY_ROOT, EMPTY_ROOT, EMPTY_ROOT, EMPTY_ROOT,
            EMPTY_ROOT,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn with_l1_anchors(
        height: u64,
        epoch: u64,
        parent_hash: Bytes32,
        state_root: Bytes32,
        spends_root: Bytes32,
        additions_root: Bytes32,
        removals_root: Bytes32,
        receipts_root: Bytes32,
        l1_height: u32,
        l1_hash: Bytes32,
        timestamp: u64,
        proposer_index: u32,
        spend_bundle_count: u32,
        total_cost: Cost,
        total_fees: u64,
        additions_count: u32,
        removals_count: u32,
        block_size: u32,
        filter_hash: Bytes32,
        extension_data: Bytes32,
        l1_collateral_coin_id: Option<Bytes32>,
        l1_reserve_coin_id: Option<Bytes32>,
        l1_prev_epoch_finalizer_coin_id: Option<Bytes32>,
        l1_curr_epoch_finalizer_coin_id: Option<Bytes32>,
        l1_network_coin_id: Option<Bytes32>,
        slash_proposal_count: u32,
        slash_proposals_root: Bytes32,
        collateral_registry_root: Bytes32,
        cid_state_root: Bytes32,
        node_registry_root: Bytes32,
        namespace_update_root: Bytes32,
        dfsp_finalize_commitment_root: Bytes32,
    ) -> Self {
        Self {
            version: Self::protocol_version_for_height(height),
            height,
            epoch,
            parent_hash,
            state_root,
            spends_root,
            additions_root,
            removals_root,
            receipts_root,
            l1_height,
            l1_hash,
            timestamp,
            proposer_index,
            spend_bundle_count,
            total_cost,
            total_fees,
            additions_count,
            removals_count,
            block_size,
            filter_hash,
            extension_data,
            l1_collateral_coin_id,
            l1_reserve_coin_id,
            l1_prev_epoch_finalizer_coin_id,
            l1_curr_epoch_finalizer_coin_id,
            l1_network_coin_id,
            slash_proposal_count,
            slash_proposals_root,
            collateral_registry_root,
            cid_state_root,
            node_registry_root,
            namespace_update_root,
            dfsp_finalize_commitment_root,
        }
    }
}
