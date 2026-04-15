//! HSH-005: Removals root — Merkle **set** over spent coin IDs (`Bytes32`), `chia_consensus::compute_merkle_set_root`.
//!
//! **Normative:** `docs/requirements/domains/hashing/NORMATIVE.md` (HSH-005)  
//! **Spec + test plan:** `docs/requirements/domains/hashing/specs/HSH-005.md`  
//! **Implementation:** `src/merkle_util.rs` [`dig_block::compute_removals_root`], delegated from [`dig_block::L2Block::compute_removals_root`]  
//! **Crate spec:** [SPEC §3.5](docs/resources/SPEC.md)  
//! **Chia reference:** [block_body_validation.py](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py) (~185)
//!
//! ## How these tests prove HSH-005
//!
//! - **`hsh005_empty_returns_empty_root`:** No removals ⇒ [`dig_block::EMPTY_ROOT`] (same empty-set convention as additions).
//! - **`hsh005_single_id_matches_merkle_set`:** One coin ID ⇒ root equals raw [`chia_consensus::merkle_set::compute_merkle_set_root`] on one leaf
//!   (proves we route through the consensus Merkle-set implementation, not a bespoke tree).
//! - **`hsh005_multiple_ids_matches_manual`:** Several IDs ⇒ matches manually built `[[u8;32]; n]` passed to `compute_merkle_set_root`.
//! - **`hsh005_order_independent_for_same_multiset`:** Permuting the input slice does not change the root — documents the **set**
//!   semantics required for light-client removal proofs (HSH-005 implementation notes).
//! - **`hsh005_chia_parity_from_spend_bundle`:** One spend bundle’s removal IDs match the BLK-004-style manual root (integration-style parity).
//! - **`hsh005_l2block_delegates_to_free_function`:** [`L2Block::compute_removals_root`] agrees with `compute_removals_root(&all_removals())`.
//!
//! **Layout:** Flat `tests/` ([STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md)).  
//! **SocratiCode:** Not used in this environment (no MCP).

use chia_bls::G2Element;
use chia_consensus::merkle_set::compute_merkle_set_root;
use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use dig_block::{
    compute_removals_root, Bytes32, Cost, L2Block, L2BlockHeader, Signature, EMPTY_ROOT,
};

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

/// Same single-CREATE pattern as BLK-004 tests — yields one removal (spent coin id).
fn spend_single_create_hex_coin() -> (Coin, SpendBundle) {
    let test_coin = Coin::new(
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
    let solution = hex::decode(
        "ffff33ffa02222222222222222222222222222222222222222222222222222222222222222ff01\
         8080",
    )
    .unwrap();
    let spend = CoinSpend::new(
        test_coin,
        Program::new(vec![1_u8].into()),
        Program::new(solution.into()),
    );
    let bundle = SpendBundle::new(vec![spend], G2Element::default());
    (test_coin, bundle)
}

/// **Test plan:** `test_removals_root_empty` — empty slice ⇒ [`EMPTY_ROOT`].
#[test]
fn hsh005_empty_returns_empty_root() {
    let ids: Vec<Bytes32> = vec![];
    assert_eq!(compute_removals_root(&ids), EMPTY_ROOT);
}

/// **Test plan:** `test_removals_root_single_removal` — one leaf through consensus `compute_merkle_set_root`.
#[test]
fn hsh005_single_id_matches_merkle_set() {
    let id = Bytes32::new([0x5a; 32]);
    let mut leafs = vec![id.to_bytes()];
    let expected = Bytes32::new(compute_merkle_set_root(&mut leafs));
    assert_eq!(compute_removals_root(&[id]), expected);
}

/// **Test plan:** `test_removals_root_multiple_removals` — N leaves, manual parity.
#[test]
fn hsh005_multiple_ids_matches_manual() {
    let ids: Vec<Bytes32> = (0u8..5).map(|i| Bytes32::new([i; 32])).collect();
    let mut leafs: Vec<[u8; 32]> = ids.iter().map(|b| (*b).to_bytes()).collect();
    let expected = Bytes32::new(compute_merkle_set_root(&mut leafs));
    assert_eq!(compute_removals_root(&ids), expected);
}

/// **Test plan:** `test_removals_root_order_independent` — permutations of the same IDs ⇒ same root.
#[test]
fn hsh005_order_independent_for_same_multiset() {
    let a = Bytes32::new([0x01; 32]);
    let b = Bytes32::new([0x02; 32]);
    let c = Bytes32::new([0x03; 32]);
    let r1 = compute_removals_root(&[a, b, c]);
    let r2 = compute_removals_root(&[c, a, b]);
    let r3 = compute_removals_root(&[b, c, a]);
    assert_eq!(r1, r2);
    assert_eq!(r2, r3);
}

/// **Test plan:** `test_removals_root_chia_parity` — known spend ⇒ same root as manual set over its removal id(s).
#[test]
fn hsh005_chia_parity_from_spend_bundle() {
    let (coin, sb) = spend_single_create_hex_coin();
    let ids: Vec<Bytes32> = sb.coin_spends.iter().map(|cs| cs.coin.coin_id()).collect();
    let mut leafs: Vec<[u8; 32]> = ids.iter().map(|b| (*b).to_bytes()).collect();
    let expected = Bytes32::new(compute_merkle_set_root(&mut leafs));
    assert_eq!(compute_removals_root(&ids), expected);
    assert_eq!(ids, vec![coin.coin_id()]);
    let block = block_with_bundles(vec![sb]);
    assert_eq!(block.compute_removals_root(), expected);
}

/// **Test plan:** delegation — block helper equals free function on the same ID list.
#[test]
fn hsh005_l2block_delegates_to_free_function() {
    let (_, sb) = spend_single_create_hex_coin();
    let block = block_with_bundles(vec![sb]);
    let ids = block.all_removals();
    assert_eq!(
        block.compute_removals_root(),
        compute_removals_root(&ids),
        "L2Block must delegate to compute_removals_root for a single normative path"
    );
}
