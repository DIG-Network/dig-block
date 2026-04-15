//! HSH-003: Spends root — Merkle root over SHA-256(serialized [`SpendBundle`]) leaves in block order.
//!
//! **Normative:** `docs/requirements/domains/hashing/NORMATIVE.md` (HSH-003)  
//! **Spec + test plan:** `docs/requirements/domains/hashing/specs/HSH-003.md`  
//! **Implementation:** `src/merkle_util.rs` [`dig_block::compute_spends_root`], delegated from [`dig_block::L2Block::compute_spends_root`]  
//! **Crate spec:** [SPEC §3.3](docs/resources/SPEC.md)  
//! **Tagged tree:** [HSH-007](docs/requirements/domains/hashing/specs/HSH-007.md) / [`chia_sdk_types::MerkleTree`]
//!
//! ## How these tests prove HSH-003
//!
//! - **`hsh003_empty_returns_empty_root`:** No bundles → [`dig_block::EMPTY_ROOT`] (SHA-256 of empty string, BLK-005).
//! - **`hsh003_leaf_is_sha256_of_serialized_bundle`:** Leaf digest is computed from [`Streamable::to_bytes`] and SHA-256,
//!   not an ad hoc field — matches the requirement’s `sha256(bundle.to_bytes())` rule.
//! - **`hsh003_leaf_matches_spend_bundle_name`:** Chia’s [`SpendBundle::name`] is `hash().into()` on the same streamable
//!   bytes, so the HSH-003 leaf equals `name()` — interoperability with existing Chia tooling.
//! - **`hsh003_single_bundle_matches_merkle_tree`:** One leaf → root equals [`MerkleTree::new`] over that leaf (tagged
//!   leaf step per HSH-007); disambiguates “single leaf” from naive double-SHA-256 misconceptions in older prose.
//! - **`hsh003_multiple_bundles_block_order`:** Three bundles → root matches `MerkleTree` over the leaf slice in order.
//! - **`hsh003_order_matters`:** Swapping two bundles changes the root (commitment to ordering).
//! - **`hsh003_deterministic`:** Same input slice → same root twice.
//! - **`hsh003_l2block_delegates_to_free_function`:** [`L2Block::compute_spends_root`] is a thin wrapper over
//!   [`dig_block::compute_spends_root`] so BLK-004 tests and header validation share one algorithm.
//!
//! **Layout:** Flat `tests/` ([STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md)).  
//! **SocratiCode:** Not used in this environment (no MCP).

use chia_bls::G2Element;
use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use chia_sdk_types::MerkleTree;
use chia_sha2::Sha256;
use chia_traits::Streamable;
use dig_block::{
    compute_spends_root, Bytes32, Cost, L2Block, L2BlockHeader, Signature, EMPTY_ROOT,
};

/// Deterministic minimal header for body-only spends-root tests (header roots are not validated here).
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

/// One [`SpendBundle`] using the `chia-protocol` single-`CREATE_COIN` pattern (same fixture idea as BLK-004 tests).
fn spend_single_create_hex_coin() -> SpendBundle {
    spend_bundle_with_coin_hex(
        "4444444444444444444444444444444444444444444444444444444444444444",
        "3333333333333333333333333333333333333333333333333333333333333333",
    )
}

/// Second fixture with **distinct** streamable bytes so two bundles are not byte-identical (needed for order tests).
fn spend_single_create_hex_coin_alt() -> SpendBundle {
    spend_bundle_with_coin_hex(
        "5555555555555555555555555555555555555555555555555555555555555555",
        "6666666666666666666666666666666666666666666666666666666666666666",
    )
}

fn spend_bundle_with_coin_hex(parent_hex: &str, puzzle_hex: &str) -> SpendBundle {
    let test_coin = Coin::new(
        hex::decode(parent_hex).unwrap().try_into().unwrap(),
        hex::decode(puzzle_hex).unwrap().try_into().unwrap(),
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
    SpendBundle::new(vec![spend], G2Element::default())
}

/// SHA-256 over streamable serialization — the HSH-003 normative leaf step (see spec pseudocode).
fn leaf_sha256_serialized(bundle: &SpendBundle) -> Bytes32 {
    let bytes = bundle.to_bytes().expect("fixture bundle must serialize");
    let mut h = Sha256::new();
    h.update(&bytes);
    Bytes32::new(h.finalize())
}

/// **Test plan:** `test_spends_root_empty` — empty slice ⇒ [`EMPTY_ROOT`].
#[test]
fn hsh003_empty_returns_empty_root() {
    let bundles: Vec<SpendBundle> = vec![];
    assert_eq!(compute_spends_root(&bundles), EMPTY_ROOT);
}

/// **Test plan:** leaf digest matches explicit `SHA-256(to_bytes())` construction.
#[test]
fn hsh003_leaf_is_sha256_of_serialized_bundle() {
    let sb = spend_single_create_hex_coin();
    let from_fn = {
        let bytes = sb.to_bytes().unwrap();
        let mut h = Sha256::new();
        h.update(&bytes);
        Bytes32::new(h.finalize())
    };
    assert_eq!(from_fn, leaf_sha256_serialized(&sb));
}

/// **Test plan:** Chia bundle identity hash matches HSH-003 leaf definition.
#[test]
fn hsh003_leaf_matches_spend_bundle_name() {
    let sb = spend_single_create_hex_coin();
    assert_eq!(leaf_sha256_serialized(&sb), sb.name());
}

/// **Test plan:** `test_spends_root_single_bundle` — one bundle ⇒ [`MerkleTree`] root over one leaf.
#[test]
fn hsh003_single_bundle_matches_merkle_tree() {
    let sb = spend_single_create_hex_coin();
    let leaf = leaf_sha256_serialized(&sb);
    let expected = MerkleTree::new(&[leaf]).root();
    assert_eq!(compute_spends_root(&[sb]), expected);
}

/// **Test plan:** `test_spends_root_multiple_bundles` — ordered list matches manual [`MerkleTree`].
#[test]
fn hsh003_multiple_bundles_block_order() {
    let a = spend_single_create_hex_coin();
    let b = spend_single_create_hex_coin();
    let c = spend_single_create_hex_coin();
    let leaves = [
        leaf_sha256_serialized(&a),
        leaf_sha256_serialized(&b),
        leaf_sha256_serialized(&c),
    ];
    let expected = MerkleTree::new(&leaves).root();
    assert_eq!(compute_spends_root(&[a, b, c]), expected);
}

/// **Test plan:** `test_spends_root_order_matters` — permutation changes the root.
///
/// **Note:** Two byte-identical bundles occupy identical Merkle leaves, so swapping them cannot change the root.
/// This test uses **distinct** fixtures ([`spend_single_create_hex_coin`] vs [`spend_single_create_hex_coin_alt`])
/// so leaf digests differ and order is observable.
#[test]
fn hsh003_order_matters() {
    let a = spend_single_create_hex_coin();
    let b = spend_single_create_hex_coin_alt();
    let r1 = compute_spends_root(&[a.clone(), b.clone()]);
    let r2 = compute_spends_root(&[b, a]);
    assert_ne!(
        r1, r2,
        "spends_root must commit to spend-bundle order (SPEC §3.3 / HSH-003)"
    );
}

/// **Test plan:** `test_spends_root_deterministic` — pure function over bundle slice.
#[test]
fn hsh003_deterministic() {
    let a = spend_single_create_hex_coin();
    let b = spend_single_create_hex_coin();
    let slice = [a, b];
    assert_eq!(compute_spends_root(&slice), compute_spends_root(&slice));
}

/// **Test plan:** [`L2Block::compute_spends_root`] agrees with [`compute_spends_root`] on the same `spend_bundles` vec.
#[test]
fn hsh003_l2block_delegates_to_free_function() {
    let a = spend_single_create_hex_coin();
    let b = spend_single_create_hex_coin();
    let bundles = vec![a, b];
    let block = L2Block::new(
        dummy_header(),
        bundles.clone(),
        vec![],
        Signature::default(),
    );
    assert_eq!(block.compute_spends_root(), compute_spends_root(&bundles));
}
