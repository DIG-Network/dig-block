//! Internal helpers for L2 block Merkle roots and BIP158 filters ([BLK-004](docs/requirements/domains/block_types/specs/BLK-004.md)).
//!
//! **Spec:** [SPEC §3.3–§3.6](docs/resources/SPEC.md). Uses `chia-consensus` Merkle sets, `chia-sdk-types`
//! binary Merkle trees, `chia-sha2`, and `bitcoin::bip158::GcsFilterWriter` (BIP-158 parameters `M`, `P`
//! matching Bitcoin Core / [`bitcoin::bip158`](https://docs.rs/bitcoin/latest/bitcoin/bip158/index.html)).

use bitcoin::bip158::GcsFilterWriter;
use chia_consensus::merkle_set::compute_merkle_set_root;
use chia_protocol::Bytes32;
use chia_sdk_types::MerkleTree;
use chia_sha2::Sha256;
use clvmr::reduction::EvalErr;

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

/// [`L2Block::slash_proposal_leaf_hash`](crate::types::block::L2Block::slash_proposal_leaf_hash) — SHA-256 over raw payload.
#[inline]
pub fn slash_leaf_hash(payload: &[u8]) -> Bytes32 {
    let mut h = Sha256::new();
    h.update(payload);
    Bytes32::new(h.finalize())
}

/// Encode BIP-158 compact filter bytes (GCS) over **32-byte elements** (puzzle hashes and coin IDs), keyed
/// by the first 16 bytes of `block_identity` (same layout as Bitcoin’s block-filter construction).
///
/// **Chia parity:** [`block_creation.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py)
/// builds `byte_array_tx` from puzzle hashes / coin names, then `filter_hash = std_hash(PyBIP158(...).GetEncoded())`.
/// Elements should follow block semantics: addition puzzle hashes in [`L2Block::all_additions`] order, then
/// removal coin IDs in [`L2Block::all_removals`] order (SPEC §3.6).
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
