//! HSH-004: Additions root — `chia_consensus` Merkle set over `[puzzle_hash, hash_coin_ids(group)]` pairs.
//!
//! **Normative:** `docs/requirements/domains/hashing/NORMATIVE.md` (HSH-004)  
//! **Spec + test plan:** `docs/requirements/domains/hashing/specs/HSH-004.md`  
//! **Implementation:** `src/merkle_util.rs` [`dig_block::compute_additions_root`], delegated from [`dig_block::L2Block::compute_additions_root`]  
//! **Crate spec:** [SPEC §3.4](docs/resources/SPEC.md)  
//! **Chia reference:** [block_body_validation.py](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py) (additions Merkle construction)
//!
//! ## How these tests prove HSH-004
//!
//! - **`hsh004_empty_returns_empty_root`:** No additions ⇒ [`dig_block::EMPTY_ROOT`] (BLK-005 / SPEC empty-additions sentinel).
//! - **`hsh004_single_coin_matches_manual_merkle_set`:** One coin ⇒ Merkle-set input is exactly `[puzzle_hash, hash_coin_ids([coin_id])]`;
//!   result matches raw [`chia_consensus::merkle_set::compute_merkle_set_root`] on those two leaves (proves we use the consensus primitive).
//! - **`hsh004_same_puzzle_hash_groups_coin_ids`:** Two distinct coins sharing a `puzzle_hash` ⇒ one group; second leaf is
//!   `hash_coin_ids` over **both** IDs (sorted descending), not two separate groups.
//! - **`hsh004_distinct_puzzle_hashes_two_pairs`:** Two groups ⇒ four 32-byte leaves in **first-seen** `puzzle_hash` order,
//!   matching Chia dict insertion order when walking additions in block order.
//! - **`hsh004_chia_parity_two_bundles`:** Replicates the BLK-004 grouped-additions scenario: expected root built with the same
//!   leaf layout as [`block_body_validation`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py)
//!   style grouping (see HSH-004 implementation notes).
//! - **`hsh004_l2block_delegates_to_free_function`:** [`L2Block::compute_additions_root`] matches `compute_additions_root(&all_additions())`
//!   so header validation and offline Merkle math cannot drift.
//!
//! **Layout:** Flat `tests/` ([STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md)).  
//! **SocratiCode:** Not used in this environment (no MCP); search used repo + Chia upstream references.

use chia_bls::G2Element;
use chia_consensus::merkle_set::compute_merkle_set_root;
use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use chia_sha2::Sha256;
use dig_block::{
    compute_additions_root, Bytes32, Cost, L2Block, L2BlockHeader, Signature, EMPTY_ROOT,
};

/// Chia `hash_coin_ids` behavior copied for **expected** roots (mirrors `tests/test_l2_block_helpers.rs`).
fn hash_coin_ids_chia(ids: &mut [Bytes32]) -> Bytes32 {
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

fn dummy_header() -> L2BlockHeader {
    L2BlockHeader::new(
        1,
        0,
        Bytes32::new([0x11; 32]),
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        1,
        Bytes32::new([0x22; 32]),
        0,
        0,
        0 as Cost,
        0,
        0,
        0,
        0,
        EMPTY_ROOT,
    )
}

fn block_with_bundles(bundles: Vec<SpendBundle>) -> L2Block {
    L2Block::new(dummy_header(), bundles, vec![], Signature::default())
}

/// CREATE_COIN output with the given `puzzle_hash` (pattern aligned with `test_l2_block_helpers::bundle_create_coin_with_ph`).
fn bundle_create_coin_with_ph(output_ph: Bytes32) -> SpendBundle {
    let spent = Coin::new(
        hex::decode("4444444444444444444444444444444444444444444444444444444444444444")
            .unwrap()
            .try_into()
            .unwrap(),
        hex::decode("3333333333333333333333333333333333333333333333333333333333333333")
            .unwrap()
            .try_into()
            .unwrap(),
        1,
    );
    let mut sol = vec![0xffu8, 0xff, 0x33, 0xff, 0xa0];
    sol.extend_from_slice(output_ph.as_ref());
    sol.extend_from_slice(&[0xff, 0x01, 0x80, 0x80]);
    let spend = CoinSpend::new(
        spent,
        Program::new(vec![1u8].into()),
        Program::new(sol.into()),
    );
    SpendBundle::new(vec![spend], G2Element::default())
}

/// **Test plan:** `test_additions_root_empty` — empty slice ⇒ [`EMPTY_ROOT`].
#[test]
fn hsh004_empty_returns_empty_root() {
    let additions: Vec<Coin> = vec![];
    assert_eq!(compute_additions_root(&additions), EMPTY_ROOT);
}

/// **Test plan:** `test_additions_root_single_coin` — one addition ⇒ Merkle set over `[ph, hash(ids)]`.
#[test]
fn hsh004_single_coin_matches_manual_merkle_set() {
    let ph = Bytes32::new([0xc0; 32]);
    let parent = Bytes32::new([0x01; 32]);
    let coin = Coin::new(parent, ph, 123);
    let mut one = vec![coin.coin_id()];
    let id_hash = hash_coin_ids_chia(&mut one);
    let mut leafs = vec![ph.to_bytes(), id_hash.to_bytes()];
    let expected = Bytes32::new(compute_merkle_set_root(&mut leafs));
    assert_eq!(compute_additions_root(&[coin]), expected);
}

/// **Test plan:** `test_additions_root_same_puzzle_hash` — multiple coins, one `puzzle_hash` bucket.
#[test]
fn hsh004_same_puzzle_hash_groups_coin_ids() {
    let ph = Bytes32::new([0xde; 32]);
    let c1 = Coin::new(Bytes32::new([0x10; 32]), ph, 1);
    let c2 = Coin::new(Bytes32::new([0x20; 32]), ph, 2);
    // Slice order: c1 then c2 — IDs collected in that order; hash_coin_ids sorts internally.
    let mut ids = vec![c1.coin_id(), c2.coin_id()];
    let id_hash = hash_coin_ids_chia(&mut ids);
    let mut leafs = vec![ph.to_bytes(), id_hash.to_bytes()];
    let expected = Bytes32::new(compute_merkle_set_root(&mut leafs));
    assert_eq!(compute_additions_root(&[c1, c2]), expected);
}

/// **Test plan:** `test_additions_root_different_puzzle_hashes` — two groups, first-seen ph order in leaf stream.
#[test]
fn hsh004_distinct_puzzle_hashes_two_pairs() {
    let ph1 = Bytes32::new([0x01; 32]);
    let ph2 = Bytes32::new([0x02; 32]);
    let coin1 = Coin::new(Bytes32::new([0xaa; 32]), ph1, 1);
    let coin2 = Coin::new(Bytes32::new([0xbb; 32]), ph2, 1);
    let additions = [coin1, coin2];
    let mut a_ids: Vec<Bytes32> = additions
        .iter()
        .filter(|c| c.puzzle_hash == ph1)
        .map(Coin::coin_id)
        .collect();
    let mut b_ids: Vec<Bytes32> = additions
        .iter()
        .filter(|c| c.puzzle_hash == ph2)
        .map(Coin::coin_id)
        .collect();
    let h1 = hash_coin_ids_chia(&mut a_ids);
    let h2 = hash_coin_ids_chia(&mut b_ids);
    let mut leafs = vec![ph1.to_bytes(), h1.to_bytes(), ph2.to_bytes(), h2.to_bytes()];
    let expected = Bytes32::new(compute_merkle_set_root(&mut leafs));
    assert_eq!(compute_additions_root(&additions), expected);
}

/// **Test plan:** `test_additions_root_chia_parity` — two bundles, two puzzle groups (BLK-004 / HSH-004 cross-check).
#[test]
fn hsh004_chia_parity_two_bundles() {
    let ph1 = Bytes32::new([0x01; 32]);
    let ph2 = Bytes32::new([0x02; 32]);
    let b1 = bundle_create_coin_with_ph(ph1);
    let b2 = bundle_create_coin_with_ph(ph2);
    let block = block_with_bundles(vec![b1, b2]);
    let adds: Vec<Coin> = block.all_additions();
    assert_eq!(adds.len(), 2);

    let mut ids1: Vec<Bytes32> = adds
        .iter()
        .filter(|c| c.puzzle_hash == ph1)
        .map(Coin::coin_id)
        .collect();
    let mut ids2: Vec<Bytes32> = adds
        .iter()
        .filter(|c| c.puzzle_hash == ph2)
        .map(Coin::coin_id)
        .collect();
    let h1 = hash_coin_ids_chia(&mut ids1);
    let h2 = hash_coin_ids_chia(&mut ids2);
    let mut manual = vec![ph1.to_bytes(), h1.to_bytes(), ph2.to_bytes(), h2.to_bytes()];
    let expected = Bytes32::new(compute_merkle_set_root(&mut manual));

    assert_eq!(compute_additions_root(&adds), expected);
    assert_eq!(block.compute_additions_root(), expected);
}

/// **Test plan:** structural delegation — block helper uses the same algorithm as the crate-root function.
#[test]
fn hsh004_l2block_delegates_to_free_function() {
    let block = block_with_bundles(vec![
        bundle_create_coin_with_ph(Bytes32::new([0x07; 32])),
        bundle_create_coin_with_ph(Bytes32::new([0x08; 32])),
    ]);
    let adds = block.all_additions();
    assert_eq!(
        block.compute_additions_root(),
        compute_additions_root(&adds),
        "L2Block must delegate to compute_additions_root for a single normative path"
    );
}
