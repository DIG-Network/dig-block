//! `L2Block` — full L2 block: header, transaction body (`SpendBundle`s), slash proposal payloads, proposer signature.
//!
//! **Requirements:**
//! - [BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md) — struct + `new` / `hash` / `height` / `epoch`
//! - [HSH-003](docs/requirements/domains/hashing/specs/HSH-003.md) — [`crate::compute_spends_root`] (spends Merkle root)
//! - [BLK-004](docs/requirements/domains/block_types/specs/BLK-004.md) — Merkle roots, BIP158 `filter_hash` preimage,
//!   additions/removals collectors, duplicate / double-spend probes, serialized size
//! - [SPEC §2.3](docs/resources/SPEC.md), [SPEC §3.3–§3.6](docs/resources/SPEC.md) — body commitments + filter
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

use chia_protocol::{Coin, SpendBundle};
use chia_streamable_macro::Streamable;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::header::L2BlockHeader;
use crate::merkle_util::{
    bip158_filter_encoded, empty_on_additions_err, hash_coin_ids, merkle_set_root,
    merkle_tree_root, slash_leaf_hash,
};
use crate::primitives::{Bytes32, Signature};

/// Complete L2 block: header plus body (spend bundles, slash payloads) and proposer attestation.
///
/// See [BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md) and [`SPEC §2.3`](docs/resources/SPEC.md).
/// **Chia [`Streamable`] (wire):** see [`L2BlockHeader`] — gossip uses this encoding; persistence uses bincode + zstd in dig-blockstore.
#[derive(Debug, Clone, Serialize, Deserialize, Streamable)]
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

    // --- BLK-004: Merkle roots (SPEC §3.3–§3.5) ---

    /// Merkle root over spend-bundle leaf digests in **block order**; empty body → [`crate::EMPTY_ROOT`].
    ///
    /// **Delegation:** [`crate::compute_spends_root`] ([HSH-003](docs/requirements/domains/hashing/specs/HSH-003.md)) —
    /// each leaf is SHA-256 of serialized [`SpendBundle`] bytes; [`chia_sdk_types::MerkleTree`] applies tagged hashing
    /// (HSH-007, SPEC §3.3 `spends_root` row).
    #[must_use]
    pub fn compute_spends_root(&self) -> Bytes32 {
        crate::compute_spends_root(&self.spend_bundles)
    }

    /// Additions Merkle root: group created coins by `puzzle_hash`, pair each group with [`hash_coin_ids`],
    /// then [`chia_consensus::merkle_set::compute_merkle_set_root`]-compatible set ([SPEC §3.4](docs/resources/SPEC.md)).
    ///
    /// **Order:** [`IndexMap`] preserves first-seen `puzzle_hash` order while aggregating coin IDs (Chia
    /// `dict` insertion order parity).
    #[must_use]
    pub fn compute_additions_root(&self) -> Bytes32 {
        let mut groups: IndexMap<Bytes32, Vec<Bytes32>> = IndexMap::new();
        for coin in self.all_additions() {
            let id = coin.coin_id();
            groups.entry(coin.puzzle_hash).or_default().push(id);
        }
        if groups.is_empty() {
            return merkle_set_root(&mut []);
        }
        let mut leafs: Vec<[u8; 32]> = Vec::with_capacity(groups.len() * 2);
        for (ph, mut ids) in groups {
            leafs.push(ph.to_bytes());
            leafs.push(hash_coin_ids(&mut ids).to_bytes());
        }
        merkle_set_root(&mut leafs)
    }

    /// Removals Merkle set over all spent coin IDs in **spend-bundle then coin-spend order**.
    #[must_use]
    pub fn compute_removals_root(&self) -> Bytes32 {
        let ids = self.all_removals();
        if ids.is_empty() {
            return merkle_set_root(&mut []);
        }
        let mut leafs: Vec<[u8; 32]> = ids.iter().map(|b| b.to_bytes()).collect();
        merkle_set_root(&mut leafs)
    }

    /// SHA-256 of the BIP-158–encoded element set (SPEC §3.6, Chia `std_hash(encoded)`).
    ///
    /// **Elements:** each addition’s `puzzle_hash`, then each removal’s `coin_id`, in [`Self::all_additions`] /
    /// [`Self::all_removals`] order. **SipHash keys:** first 8 + next 8 bytes (LE `u64` pair) of [`Self::hash`]
    /// (header identity), matching Bitcoin/rust-bitcoin `GcsFilterWriter` initialization.
    #[must_use]
    pub fn compute_filter_hash(&self) -> Bytes32 {
        let mut buf: Vec<[u8; 32]> = Vec::new();
        for c in self.all_additions() {
            buf.push(c.puzzle_hash.to_bytes());
        }
        for id in self.all_removals() {
            buf.push(id.to_bytes());
        }
        let encoded = bip158_filter_encoded(self.hash(), &buf).unwrap_or_default();
        let mut h = chia_sha2::Sha256::new();
        h.update(&encoded);
        Bytes32::new(h.finalize())
    }

    /// Binary Merkle root over slash payload digests (`sha256` each), in payload order.
    #[must_use]
    pub fn compute_slash_proposals_root(&self) -> Bytes32 {
        Self::slash_proposals_root_from(&self.slash_proposal_payloads)
    }

    /// [`Self::compute_slash_proposals_root`] for an explicit payload list (tests, pre-serialized batches).
    #[must_use]
    pub fn slash_proposals_root_from(payloads: &[Vec<u8>]) -> Bytes32 {
        if payloads.is_empty() {
            return merkle_tree_root(&[]);
        }
        let leaves: Vec<Bytes32> = payloads.iter().map(|p| slash_leaf_hash(p)).collect();
        merkle_tree_root(&leaves)
    }

    /// Single slash payload leaf digest (building block for [`Self::compute_slash_proposals_root`]).
    #[must_use]
    pub fn slash_proposal_leaf_hash(payload: &[u8]) -> Bytes32 {
        slash_leaf_hash(payload)
    }

    // --- BLK-004: collections & integrity ---

    /// All `CREATE_COIN` outputs from every spend bundle (CLVM-simulated per [`SpendBundle::additions`]).
    #[must_use]
    pub fn all_additions(&self) -> Vec<Coin> {
        let mut out = Vec::new();
        for sb in &self.spend_bundles {
            out.extend(empty_on_additions_err(sb.additions()));
        }
        out
    }

    /// Coin IDs of every addition in body order (same walk as [`Self::all_additions`]).
    #[must_use]
    pub fn all_addition_ids(&self) -> Vec<Bytes32> {
        self.all_additions()
            .into_iter()
            .map(|c| c.coin_id())
            .collect()
    }

    /// Spent coin IDs (`CoinSpend.coin`) in bundle / spend order.
    #[must_use]
    pub fn all_removals(&self) -> Vec<Bytes32> {
        self.spend_bundles
            .iter()
            .flat_map(|sb| sb.coin_spends.iter().map(|cs| cs.coin.coin_id()))
            .collect()
    }

    /// First duplicate output coin ID in addition set, else `None` (SPEC / Chia duplicate-output check).
    #[must_use]
    pub fn has_duplicate_outputs(&self) -> Option<Bytes32> {
        first_duplicate_addition_coin_id(&self.all_additions())
    }

    /// First coin ID spent twice as a removal, else `None`.
    #[must_use]
    pub fn has_double_spends(&self) -> Option<Bytes32> {
        let mut seen = std::collections::HashSet::<Bytes32>::new();
        self.all_removals().into_iter().find(|&id| !seen.insert(id))
    }

    /// Full `bincode` body size (header + spends + slash payloads + signature), per SPEC serialization rules.
    #[must_use]
    pub fn compute_size(&self) -> usize {
        bincode::serialize(self).map(|b| b.len()).unwrap_or(0)
    }
}

/// First repeated [`Coin::coin_id`] in a slice of additions (shared by [`L2Block::has_duplicate_outputs`]).
#[must_use]
fn first_duplicate_addition_coin_id(coins: &[Coin]) -> Option<Bytes32> {
    let mut seen = std::collections::HashSet::<Bytes32>::new();
    for c in coins {
        let id = c.coin_id();
        if !seen.insert(id) {
            return Some(id);
        }
    }
    None
}

/// Exposed for [`tests/test_l2_block_helpers.rs`] (BLK-004) only — not protocol surface.
#[doc(hidden)]
#[must_use]
pub fn __blk004_first_duplicate_addition_coin_id(coins: &[Coin]) -> Option<Bytes32> {
    first_duplicate_addition_coin_id(coins)
}
