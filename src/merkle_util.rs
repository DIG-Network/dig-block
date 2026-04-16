//! Internal helpers for L2 block Merkle roots and BIP158 filters ([BLK-004](docs/requirements/domains/block_types/specs/BLK-004.md)).
//!
//! **Spec:** [SPEC §3.3–§3.6](docs/resources/SPEC.md). Uses `chia-consensus` Merkle sets, `chia-sdk-types`
//! binary Merkle trees, `chia-sha2`, and `bitcoin::bip158::GcsFilterWriter` (BIP-158 parameters `M`, `P`
//! matching Bitcoin Core / [`bitcoin::bip158`](https://docs.rs/bitcoin/latest/bitcoin/bip158/index.html)).
//!
//! **HSH-007 nuance:** `chia_sdk_types::MerkleTree` leaves/nodes use `0x01`/`0x02` tagging ([`crate::hash`]); `chia_consensus::merkle_set::compute_merkle_set_root` is a
//! **different** radix-tree hash (see `chia_consensus::merkle_set`) for sorted coin-id sets — do not mix the two formulas.
//!
//! **HSH-003 ([`compute_spends_root`]):** spends roots use **binary** [`MerkleTree`] over per-bundle digests; each leaf is
//! SHA-256 of the streamable [`SpendBundle`](chia_protocol::SpendBundle) bytes (same digest as [`SpendBundle::name`] /
//! [`chia_traits::Streamable::hash`]) — see [HSH-003](docs/requirements/domains/hashing/specs/HSH-003.md).
//!
//! **HSH-004 ([`compute_additions_root`]):** additions roots use **`chia_consensus::merkle_set`** (radix Merkle **set**),
//! not the tagged binary [`MerkleTree`] from HSH-007 — see [HSH-004](docs/requirements/domains/hashing/specs/HSH-004.md).
//!
//! **HSH-005 ([`compute_removals_root`]):** removals roots are a Merkle **set** over **raw removal coin IDs** (one leaf per
//! ID) — see [HSH-005](docs/requirements/domains/hashing/specs/HSH-005.md).
//!
//! **HSH-006 ([`compute_filter_hash`]):** `filter_hash` is SHA-256 of BIP-158 GCS bytes over addition `puzzle_hash`s then
//! removal `coin_id`s — see [HSH-006](docs/requirements/domains/hashing/specs/HSH-006.md).

use bitcoin::bip158::GcsFilterWriter;
use chia_consensus::merkle_set::compute_merkle_set_root;
use chia_protocol::{Bytes32, Coin, SpendBundle};
use chia_sdk_types::MerkleTree;
use chia_sha2::Sha256;
use chia_traits::Streamable;
use clvmr::reduction::EvalErr;
use indexmap::IndexMap;

use crate::constants::EMPTY_ROOT;

/// BIP-158 Golomb-Rice `P` and range `M` (same as Bitcoin / [`bitcoin::bip158`](https://docs.rs/bitcoin/latest/bitcoin/bip158/index.html)).
const BIP158_M: u64 = 784_931;
const BIP158_P: u8 = 19;

/// Hash a list of coin IDs the way Chia L1 does for grouped additions ([`hash_coin_ids`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/types/blockchain_format/coin.py)).
///
/// **Rules:** one id → SHA-256(raw id); multiple → sort ids descending lexicographic, concatenate, SHA-256.
pub fn hash_coin_ids(ids: &mut [Bytes32]) -> Bytes32 {
    match ids.len() {
        0 => EMPTY_ROOT,
        1 => {
            let mut h = Sha256::new();
            h.update(ids[0].as_ref());
            Bytes32::new(h.finalize())
        }
        _ => {
            ids.sort_unstable_by(|a, b| b.as_ref().cmp(a.as_ref()));
            let mut h = Sha256::new();
            for id in ids.iter() {
                h.update(id.as_ref());
            }
            Bytes32::new(h.finalize())
        }
    }
}

/// [`compute_merkle_set_root`] with DIG empty-set semantics: **empty → [`EMPTY_ROOT`]** (SPEC §3.3), not the
/// internal all-zero sentinel used inside `chia-consensus`’s radix tree.
pub fn merkle_set_root(leafs: &mut [[u8; 32]]) -> Bytes32 {
    if leafs.is_empty() {
        return EMPTY_ROOT;
    }
    Bytes32::new(compute_merkle_set_root(leafs))
}

/// Binary Merkle tree ([`MerkleTree`]) for spend-bundle hashes / slash leaves; empty → [`EMPTY_ROOT`].
pub fn merkle_tree_root(leaves: &[Bytes32]) -> Bytes32 {
    if leaves.is_empty() {
        return EMPTY_ROOT;
    }
    MerkleTree::new(leaves).root()
}

/// **Spends root** (header `spends_root`, SPEC §3.3) — Merkle root over ordered spend-bundle leaf digests.
///
/// **Normative:** [HSH-003](docs/requirements/domains/hashing/specs/HSH-003.md).
/// **Algorithm:** empty slice → [`EMPTY_ROOT`]; else a binary Merkle tree (`chia_sdk_types::MerkleTree::new`) over
/// leaves `SHA-256(bundle.to_bytes())` in **slice order** (block order). Tagged hashing inside the tree follows
/// HSH-007 using the same `chia-sdk-types` implementation.
///
/// **Equivalence:** For valid in-memory bundles, `SHA-256(to_bytes())` matches [`SpendBundle::name`] because Chia’s
/// streamable `hash()` hashes the same serialized bytes ([`Streamable::hash`](chia_traits::Streamable::hash)).
///
/// **Callers:** [`crate::L2Block::compute_spends_root`](crate::L2Block::compute_spends_root) delegates here so block
/// bodies and standalone bundle slices share one definition.
#[must_use]
pub fn compute_spends_root(spend_bundles: &[SpendBundle]) -> Bytes32 {
    if spend_bundles.is_empty() {
        return EMPTY_ROOT;
    }
    let leaves: Vec<Bytes32> = spend_bundles
        .iter()
        .map(|bundle| {
            let bytes = bundle.to_bytes().unwrap_or_else(|e| {
                panic!("SpendBundle::to_bytes failed for Merkle leaf (invariant): {e:?}")
            });
            let mut h = Sha256::new();
            h.update(&bytes);
            Bytes32::new(h.finalize())
        })
        .collect();
    merkle_tree_root(&leaves)
}

/// **`additions_root`** (header field, SPEC §3.4) — Merkle-set root over created coins grouped by `puzzle_hash`.
///
/// **Normative:** [HSH-004](docs/requirements/domains/hashing/specs/HSH-004.md) and Chia
/// [`block_body_validation`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py)
/// (additions handling ~158–175).  
/// **Algorithm:** Walk `additions` in **slice order**; bucket coin IDs by [`Coin::puzzle_hash`]. Emit **two** 32-byte
/// leaves per group, in **first-seen `puzzle_hash` order**: `[puzzle_hash, hash_coin_ids(ids…)]`, then
/// `chia_consensus::merkle_set::compute_merkle_set_root` with DIG empty-set semantics ([`EMPTY_ROOT`] when there are
/// no additions).
///
/// **Why [`IndexMap`] instead of `HashMap`:** HSH-004’s pseudocode uses a map for grouping; Chia’s Python uses dict
/// **insertion order** when flattening groups. Rust’s `HashMap` iteration order is nondeterministic, which would make roots
/// non-reproducible. [`IndexMap`] matches insertion-order semantics and the existing [`crate::L2Block::compute_additions_root`]
/// behavior exercised by BLK-004 tests.
///
/// **`hash_coin_ids`:** sorts multiple IDs descending by bytes, concatenates, SHA-256 — see the crate-internal
/// `hash_coin_ids` helper and Chia
/// [`coin.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/types/blockchain_format/coin.py).
///
/// **Callers:** [`crate::L2Block::compute_additions_root`](crate::L2Block::compute_additions_root) delegates here after
/// collecting [`SpendBundle::additions`]-derived [`Coin`]s so validation and tooling share one definition.
#[must_use]
pub fn compute_additions_root(additions: &[Coin]) -> Bytes32 {
    if additions.is_empty() {
        return EMPTY_ROOT;
    }
    let mut groups: IndexMap<Bytes32, Vec<Bytes32>> = IndexMap::new();
    for coin in additions {
        let id = coin.coin_id();
        groups.entry(coin.puzzle_hash).or_default().push(id);
    }
    let mut leafs: Vec<[u8; 32]> = Vec::with_capacity(groups.len() * 2);
    for (ph, mut ids) in groups {
        leafs.push(ph.to_bytes());
        leafs.push(hash_coin_ids(&mut ids).to_bytes());
    }
    merkle_set_root(&mut leafs)
}

/// **`removals_root`** (header field, SPEC §3.5) — Merkle-set root over all spent coin IDs in the block.
///
/// **Normative:** [HSH-005](docs/requirements/domains/hashing/specs/HSH-005.md) and Chia
/// [`block_body_validation`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py)
/// (~185, removal coin name set).  
/// **Algorithm:** Empty slice → [`EMPTY_ROOT`]. Otherwise each [`Bytes32`] removal ID is one **leaf** in the radix Merkle
/// set (`chia_consensus::merkle_set::compute_merkle_set_root`). Unlike [`compute_additions_root`], there is **no** grouping:
/// each spent coin ID is inserted directly.
///
/// **Order:** [`chia_consensus::merkle_set::compute_merkle_set_root`] defines a **set** hash: the same multiset of IDs
/// yields the same root regardless of slice order (validated in `tests/test_hsh_005_removals_root.rs`).
///
/// **Callers:** [`crate::L2Block::compute_removals_root`](crate::L2Block::compute_removals_root) delegates here using
/// [`crate::L2Block::all_removals`] order as the canonical body walk; permuting the input slice must not change the root
/// when the set of IDs is unchanged.
#[must_use]
pub fn compute_removals_root(removals: &[Bytes32]) -> Bytes32 {
    if removals.is_empty() {
        return EMPTY_ROOT;
    }
    // UFCS / ownership: inherent [`Bytes32::to_bytes`] takes `self` (Copy); avoid resolving to
    // [`Streamable::to_bytes`] (`Result`) while `chia_traits::Streamable` is in scope for HSH-003.
    let mut leafs: Vec<[u8; 32]> = removals.iter().map(|b| (*b).to_bytes()).collect();
    merkle_set_root(&mut leafs)
}

/// [`L2Block::slash_proposal_leaf_hash`](crate::types::block::L2Block::slash_proposal_leaf_hash) — SHA-256 over raw payload.
#[inline]
pub fn slash_leaf_hash(payload: &[u8]) -> Bytes32 {
    let mut h = Sha256::new();
    h.update(payload);
    Bytes32::new(h.finalize())
}

/// BIP-158 **encoded filter bytes** (Golomb–Rice GCS) for block light-client filtering ([HSH-006](docs/requirements/domains/hashing/specs/HSH-006.md)).
///
/// **Element order:** each [`Coin::puzzle_hash`] for `additions` in slice order, then each removal [`Bytes32`] in
/// `removals` slice order — matches [`crate::L2Block::all_additions`] / [`crate::L2Block::all_removals`] when those slices are built
/// the same way ([SPEC §3.6](docs/resources/SPEC.md)).
///
/// **Wire:** [`compact_block_filter_encoded`] is a thin wrapper assembling the `[[u8;32]; n]` table and calling
/// the crate-internal `bip158_filter_encoded` helper. **Commitment:** [`compute_filter_hash`] is **SHA-256** of the returned bytes (Chia
/// `std_hash(encoded)` pattern).
///
/// **Downstream:** Light clients need these bytes (not only the hash) to run membership queries via
/// [`bitcoin::bip158::BlockFilter`].
pub fn compact_block_filter_encoded(
    block_identity: Bytes32,
    additions: &[Coin],
    removals: &[Bytes32],
) -> Result<Vec<u8>, std::io::Error> {
    let mut elements: Vec<[u8; 32]> = Vec::with_capacity(additions.len() + removals.len());
    for c in additions {
        // UFCS: `Bytes32` also has [`Streamable::to_bytes`] (`Result`); use inherent `to_bytes(self) -> [u8; 32]`.
        elements.push(Bytes32::to_bytes(c.puzzle_hash));
    }
    for id in removals {
        elements.push(Bytes32::to_bytes(*id));
    }
    bip158_filter_encoded(block_identity, &elements)
}

/// **`filter_hash`** (header field, SPEC §3.6) — SHA-256 over BIP-158 compact filter bytes.
///
/// **Normative:** [HSH-006](docs/requirements/domains/hashing/specs/HSH-006.md).  
/// **Algorithm:** [`compact_block_filter_encoded`] then [`Sha256`] over the wire (empty IO error → hash of empty encoding,
/// matching previous [`crate::L2Block::compute_filter_hash`] behavior via `unwrap_or_default()`).
///
/// **SipHash keys:** First 8 + next 8 bytes (LE `u64`) of `block_identity` — same as the crate-internal
/// `bip158_filter_encoded` helper /
/// Bitcoin [`GcsFilterWriter`](bitcoin::bip158::GcsFilterWriter) initialization used in this crate.
///
/// **Callers:** [`crate::L2Block::compute_filter_hash`](crate::L2Block::compute_filter_hash) passes
/// [`crate::L2BlockHeader::parent_hash`](crate::types::header::L2BlockHeader::parent_hash) as `block_identity` (SPEC §6.4;
/// avoids circular dependence on the committed `filter_hash` inside [`L2Block::hash`](crate::L2Block::hash)).
#[must_use]
pub fn compute_filter_hash(
    block_identity: Bytes32,
    additions: &[Coin],
    removals: &[Bytes32],
) -> Bytes32 {
    let encoded =
        compact_block_filter_encoded(block_identity, additions, removals).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(&encoded);
    Bytes32::new(h.finalize())
}

/// Encode BIP-158 compact filter bytes (GCS) over **32-byte elements** (puzzle hashes and coin IDs), keyed
/// by the first 16 bytes of `block_identity` (same layout as Bitcoin’s block-filter construction).
///
/// **Chia parity:** [`block_creation.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py)
/// builds `byte_array_tx` from puzzle hashes / coin names, then `filter_hash = std_hash(PyBIP158(...).GetEncoded())`.
/// Elements should follow block semantics: addition puzzle hashes in [`crate::L2Block::all_additions`] order, then
/// removal coin IDs in [`crate::L2Block::all_removals`] order (SPEC §3.6).
pub fn bip158_filter_encoded(
    block_identity: Bytes32,
    elements: &[[u8; 32]],
) -> Result<Vec<u8>, std::io::Error> {
    let b = block_identity.as_ref();
    let k0 = u64::from_le_bytes(b[0..8].try_into().expect("8 bytes"));
    let k1 = u64::from_le_bytes(b[8..16].try_into().expect("8 bytes"));
    let mut out = Vec::new();
    {
        let mut w = GcsFilterWriter::new(&mut out, k0, k1, BIP158_M, BIP158_P);
        for e in elements {
            w.add_element(e);
        }
        w.finish()?;
    }
    Ok(out)
}

/// Map failed CLVM addition parsing to an empty vec (malformed spends contribute no additions for helpers).
#[inline]
pub fn empty_on_additions_err<T>(r: Result<Vec<T>, EvalErr>) -> Vec<T> {
    r.unwrap_or_default()
}
