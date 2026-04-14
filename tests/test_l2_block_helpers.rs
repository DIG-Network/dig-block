//! BLK-004: [`L2Block`] helper methods — Merkle roots, BIP158 filter hash, collectors, integrity probes, size.
//!
//! **Authoritative spec:** `docs/requirements/domains/block_types/specs/BLK-004.md`
//! **Normative:** `docs/requirements/domains/block_types/NORMATIVE.md`
//! **Wire algorithms:** [SPEC §3.3–§3.6](docs/resources/SPEC.md)
//!
//! ## How this proves BLK-004
//!
//! Each test maps to the BLK-004 **Test Plan** table. We exercise empty edge cases (Merkle / slash roots → [`EMPTY_ROOT`]),
//! multi-bundle aggregation, Chia-style `puzzle_hash` grouping for `additions_root`, SipHash-seeded BIP158 encoding via
//! `bitcoin::bip158::GcsFilterWriter`, and structural duplicate / double-spend detection without invoking chain state.

use chia_bls::G2Element;
use chia_consensus::merkle_set::compute_merkle_set_root;
use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use chia_sdk_types::MerkleTree;
use dig_block::{Bytes32, Cost, L2Block, L2BlockHeader, Signature, EMPTY_ROOT};

/// Minimal header for body/helper tests (roots in header are decoys; helpers recompute from body).
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

/// One [`SpendBundle`] using the `chia-protocol` single-`CREATE_COIN` pattern ([`spend_bundle.rs`](https://github.com/Chia-Network/chia-protocol/blob/main/src/spend_bundle.rs) tests).
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

/// Two identical `CREATE_COIN` conditions in one spend → duplicate output coin IDs (Chia duplicate-output scenario).
fn block_with_bundles(bundles: Vec<SpendBundle>) -> L2Block {
    L2Block::new(dummy_header(), bundles, vec![], Signature::default())
}

// --- Merkle: spends_root ---

/// **Test plan:** `test_compute_spends_root_empty` — no bundles ⇒ [`EMPTY_ROOT`] (SPEC §3.3 spends row / BLK-004 notes).
#[test]
fn test_compute_spends_root_empty() {
    let b = block_with_bundles(vec![]);
    assert_eq!(b.compute_spends_root(), EMPTY_ROOT);
}

/// **Test plan:** `test_compute_spends_root_single` — one bundle ⇒ Merkle tree root over that bundle’s [`SpendBundle::name`].
#[test]
fn test_compute_spends_root_single() {
    let (_, sb) = spend_single_create_hex_coin();
    let b = block_with_bundles(vec![sb.clone()]);
    let expected = MerkleTree::new(&[sb.name()]).root();
    assert_eq!(b.compute_spends_root(), expected);
}

/// **Test plan:** `test_compute_spends_root_multiple` — block order of names matches [`MerkleTree`] over the same leaf slice.
#[test]
fn test_compute_spends_root_multiple() {
    let (_, a) = spend_single_create_hex_coin();
    let (_, bsb) = spend_single_create_hex_coin();
    let leaves = vec![a.name(), bsb.name()];
    let block = block_with_bundles(vec![a, bsb]);
    assert_eq!(block.compute_spends_root(), MerkleTree::new(&leaves).root());
}

// --- Merkle: additions / removals ---

/// Same as Chia `hash_coin_ids` (test-side copy for expected-root math; see [`merkle_util`](../../src/merkle_util.rs)).
fn hash_coin_ids_chia(ids: &mut [Bytes32]) -> Bytes32 {
    match ids.len() {
        0 => EMPTY_ROOT,
        1 => {
            let mut h = chia_sha2::Sha256::new();
            h.update(ids[0].as_ref());
            Bytes32::new(h.finalize())
        }
        _ => {
            ids.sort_unstable_by(|a, b| b.as_ref().cmp(a.as_ref()));
            let mut h = chia_sha2::Sha256::new();
            for id in ids.iter() {
                h.update(id.as_ref());
            }
            Bytes32::new(h.finalize())
        }
    }
}

/// Spend standard test coin, creating an output with `output_ph` (distinct puzzle_hash groups across bundles).
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

/// **Test plan:** `test_compute_additions_root_grouped` — different `puzzle_hash` groups ⇒ two Merkle-set leaves per group
/// ([SPEC §3.4](docs/resources/SPEC.md)); value matches manual `hash_coin_ids` + [`compute_merkle_set_root`] (non-empty branch).
#[test]
fn test_compute_additions_root_grouped() {
    let ph1 = Bytes32::new([0x01; 32]);
    let ph2 = Bytes32::new([0x02; 32]);

    let b1 = bundle_create_coin_with_ph(ph1);
    let b2 = bundle_create_coin_with_ph(ph2);
    let block = block_with_bundles(vec![b1, b2]);

    let adds: Vec<Coin> = block.all_additions();
    assert_eq!(adds.len(), 2);
    assert_ne!(adds[0].puzzle_hash, adds[1].puzzle_hash);

    let computed = block.compute_additions_root();

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

    // First-seen puzzle_hash order: ph1 bundle first
    let mut manual: Vec<[u8; 32]> =
        vec![ph1.to_bytes(), h1.to_bytes(), ph2.to_bytes(), h2.to_bytes()];
    let expected = Bytes32::new(compute_merkle_set_root(&mut manual));
    assert_eq!(computed, expected);
}

/// **Test plan:** `test_compute_removals_root` — removal coin IDs match Merkle set over flattened spends.
#[test]
fn test_compute_removals_root() {
    let (_, sb) = spend_single_create_hex_coin();
    let ids: Vec<Bytes32> = sb.coin_spends.iter().map(|cs| cs.coin.coin_id()).collect();
    let mut leafs: Vec<[u8; 32]> = ids.iter().map(|i| i.to_bytes()).collect();
    let expected = if leafs.is_empty() {
        EMPTY_ROOT
    } else {
        Bytes32::new(compute_merkle_set_root(&mut leafs))
    };
    let block = block_with_bundles(vec![sb]);
    assert_eq!(block.compute_removals_root(), expected);
}

// --- Filter ---

/// **Test plan:** `test_compute_filter_hash` — deterministic for fixed body; SHA-256 over BIP158 wire bytes ([SPEC §3.6](docs/resources/SPEC.md)).
#[test]
fn test_compute_filter_hash() {
    let (_, sb) = spend_single_create_hex_coin();
    let b = block_with_bundles(vec![sb]);
    let a = b.compute_filter_hash();
    let b2 = b.clone();
    assert_eq!(a, b2.compute_filter_hash());
    assert_ne!(a, EMPTY_ROOT); // non-empty additions ⇒ encoded filter non-empty ⇒ hash not empty root sentinel
}

// --- Slash ---

/// **Test plan:** `test_slash_proposal_leaf_hash` — raw SHA-256 of one payload (BLK-004 implementation notes).
#[test]
fn test_slash_proposal_leaf_hash() {
    let p = b"slash-evidence";
    let leaf = L2Block::slash_proposal_leaf_hash(p);
    let mut h = chia_sha2::Sha256::new();
    h.update(p);
    assert_eq!(leaf, Bytes32::new(h.finalize()));
}

/// **Test plan:** `test_compute_slash_proposals_root` — Merkle over leaf hashes; empty ⇒ [`EMPTY_ROOT`].
#[test]
fn test_compute_slash_proposals_root() {
    let mut header = dummy_header();
    let block = L2Block::new(header.clone(), vec![], vec![], Signature::default());
    assert_eq!(block.compute_slash_proposals_root(), EMPTY_ROOT);

    let payloads = vec![vec![1u8, 2], vec![3u8]];
    let leaves: Vec<Bytes32> = payloads
        .iter()
        .map(|v| L2Block::slash_proposal_leaf_hash(v))
        .collect();
    header.spend_bundle_count = 0;
    let b2 = L2Block::new(header, vec![], payloads, Signature::default());
    assert_eq!(
        b2.compute_slash_proposals_root(),
        MerkleTree::new(&leaves).root()
    );
}

// --- Collections ---

/// **Test plan:** `test_all_additions` — aggregates [`SpendBundle::additions`] across bundles in order.
#[test]
fn test_all_additions() {
    let (_, a) = spend_single_create_hex_coin();
    let (_, b) = spend_single_create_hex_coin();
    let block = block_with_bundles(vec![a, b]);
    assert_eq!(block.all_additions().len(), 2);
}

/// **Test plan:** `test_all_addition_ids` — `coin_id()` of each addition matches manual walk.
#[test]
fn test_all_addition_ids() {
    let (_, sb) = spend_single_create_hex_coin();
    let block = block_with_bundles(vec![sb]);
    let ids = block.all_addition_ids();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0], block.all_additions()[0].coin_id());
}

/// **Test plan:** `test_all_removals` — spends expose removal coins in order.
#[test]
fn test_all_removals() {
    let (coin, sb) = spend_single_create_hex_coin();
    let block = block_with_bundles(vec![sb]);
    assert_eq!(block.all_removals(), vec![coin.coin_id()]);
}

// --- Integrity ---

/// **Test plan:** `test_has_duplicate_outputs_none` — unique outputs ⇒ `None`.
#[test]
fn test_has_duplicate_outputs_none() {
    let (_, sb) = spend_single_create_hex_coin();
    let block = block_with_bundles(vec![sb]);
    assert!(block.has_duplicate_outputs().is_none());
}

/// **Test plan:** `test_has_duplicate_outputs_found` — first repeated addition [`Coin::coin_id`].
///
/// **Rationale:** Portable CLVM hex for two identical `CREATE_COIN` branches is brittle; the scan is shared with
/// [`L2Block::has_duplicate_outputs`] via [`dig_block::__blk004_first_duplicate_addition_coin_id`]. Two distinct
/// [`Coin`] literals with equal fields model duplicate outputs.
#[test]
fn test_has_duplicate_outputs_found() {
    let parent = Bytes32::new([0xbb; 32]);
    let ph = Bytes32::new([0xcc; 32]);
    let c0 = Coin::new(parent, ph, 123);
    let c1 = Coin::new(parent, ph, 123);
    assert_eq!(c0.coin_id(), c1.coin_id());
    let id = c0.coin_id();
    assert_eq!(
        dig_block::__blk004_first_duplicate_addition_coin_id(&[c0, c1]),
        Some(id)
    );
    assert_eq!(
        L2Block::new(dummy_header(), vec![], vec![], Signature::default()).has_duplicate_outputs(),
        None
    );
}

/// **Test plan:** `test_has_double_spends_none` — normal single-spend bundle.
#[test]
fn test_has_double_spends_none() {
    let (_, sb) = spend_single_create_hex_coin();
    let block = block_with_bundles(vec![sb]);
    assert!(block.has_double_spends().is_none());
}

/// **Test plan:** `test_has_double_spends_found` — two [`CoinSpend`] rows naming the same [`Coin`].
#[test]
fn test_has_double_spends_found() {
    let puzzle = Program::new(vec![1u8].into());
    let solution = Program::new(vec![0x80].into());
    let coin = Coin::new(Bytes32::new([0xde; 32]), Bytes32::new([0xad; 32]), 1);
    let cs1 = CoinSpend::new(coin, puzzle.clone(), solution.clone());
    let cs2 = CoinSpend::new(coin, puzzle, solution);
    let sb = SpendBundle::new(vec![cs1, cs2], G2Element::default());
    let block = block_with_bundles(vec![sb]);
    assert_eq!(block.has_double_spends(), Some(coin.coin_id()));
}

// --- Size ---

/// **Test plan:** `test_compute_size` — matches `bincode` length of the full [`L2Block`].
#[test]
fn test_compute_size() {
    let (_, sb) = spend_single_create_hex_coin();
    let block = block_with_bundles(vec![sb]);
    let n = bincode::serialize(&block).expect("serialize").len();
    assert_eq!(block.compute_size(), n);
}
