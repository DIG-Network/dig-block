//! HSH-006: `filter_hash` — SHA-256(BIP-158 Golomb–Rice compact filter over addition puzzle hashes + removal coin IDs).
//!
//! **Normative:** `docs/requirements/domains/hashing/NORMATIVE.md` (HSH-006)  
//! **Spec + test plan:** `docs/requirements/domains/hashing/specs/HSH-006.md`  
//! **Implementation:** `src/merkle_util.rs` [`dig_block::compute_filter_hash`], [`dig_block::compact_block_filter_encoded`];
//! [`dig_block::L2Block::compute_filter_hash`] delegates.  
//! **Crate spec:** [SPEC §3.6](docs/resources/SPEC.md)  
//! **BIP-158:** [BIP 158](https://github.com/bitcoin/bips/blob/master/bip-0158.mediawiki) — [`bitcoin::bip158`] in tests matches encoder params `M`, `P` in [`dig_block`] (see `merkle_util` constants).
//!
//! ## How these tests prove HSH-006
//!
//! - **`hsh006_empty_deterministic`:** No additions and no removals ⇒ **stable** [`compute_filter_hash`] across calls (empty-element
//!   filter encoding is well-defined for our writer).
//! - **`hsh006_additions_only_includes_puzzle_hashes`:** [`BlockFilter::match_any`] against the encoded bytes returns **true**
//!   for a `puzzle_hash` we inserted (proves additions contribute puzzle hashes, not coin ids).
//! - **`hsh006_removals_only_includes_coin_ids`:** Match on a removal `coin_id` with no additions (proves removals contribute raw IDs).
//! - **`hsh006_both_additions_and_removals`:** One addition + one removal ⇒ both queries match.
//! - **`hsh006_membership_negative_unlikely_element`:** A 32-byte value not inserted — expect **no** match (BIP-158 allows false
//!   positives; we pick an all-zero pattern unrelated to the small fixture set; if this ever flakes, widen Hamming distance).
//! - **`hsh006_deterministic_repeat`:** Same inputs ⇒ same digest.
//! - **`hsh006_l2block_delegates`:** [`L2Block::compute_filter_hash`] equals `compute_filter_hash(hash(), &additions, &removals)`.
//!
//! **Layout:** Flat `tests/` ([STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md)).  
//! **SocratiCode:** Not used in this environment (no MCP).

use bitcoin::bip158::BlockFilter;
use bitcoin::hashes::Hash;
use bitcoin::BlockHash;
use chia_bls::G2Element;
use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use dig_block::{
    compact_block_filter_encoded, compute_filter_hash, Bytes32, Cost, L2Block, L2BlockHeader,
    Signature, EMPTY_ROOT,
};

fn block_hash_from_identity(identity: Bytes32) -> BlockHash {
    BlockHash::from_byte_array(Bytes32::to_bytes(identity))
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

/// CREATE_COIN output with the given `puzzle_hash` (aligned with BLK-004 helpers).
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

/// **Test plan:** `test_filter_hash_empty` — deterministic hash on empty element lists.
#[test]
fn hsh006_empty_deterministic() {
    let id = Bytes32::new([0x33; 32]);
    let a: Vec<Coin> = vec![];
    let r: Vec<Bytes32> = vec![];
    let h1 = compute_filter_hash(id, &a, &r);
    let h2 = compute_filter_hash(id, &a, &r);
    assert_eq!(h1, h2);
}

/// **Test plan:** `test_filter_hash_additions_only` — positive membership on `puzzle_hash`.
#[test]
fn hsh006_additions_only_includes_puzzle_hashes() {
    let ph = Bytes32::new([0x77; 32]);
    let block_id = Bytes32::new([0x01; 32]);
    let coin = Coin::new(Bytes32::new([0xaa; 32]), ph, 1);
    let additions = vec![coin];
    let removals: Vec<Bytes32> = vec![];

    let encoded =
        compact_block_filter_encoded(block_id, &additions, &removals).expect("encode filter");
    let filter = BlockFilter::new(&encoded);
    let bh = block_hash_from_identity(block_id);
    assert!(
        filter
            .match_any(&bh, std::iter::once(ph.as_ref()))
            .expect("match query"),
        "filter must contain addition puzzle_hash (BIP-158, no false negatives for included elements)"
    );

    let h = compute_filter_hash(block_id, &additions, &removals);
    assert_eq!(h, compute_filter_hash(block_id, &additions, &removals));
}

/// **Test plan:** `test_filter_hash_removals_only` — positive membership on removal `coin_id`.
#[test]
fn hsh006_removals_only_includes_coin_ids() {
    let block_id = Bytes32::new([0x02; 32]);
    let removal_id = Bytes32::new([0x55; 32]);
    let additions: Vec<Coin> = vec![];
    let removals = vec![removal_id];

    let encoded = compact_block_filter_encoded(block_id, &additions, &removals).expect("encode");
    let filter = BlockFilter::new(&encoded);
    let bh = block_hash_from_identity(block_id);
    assert!(
        filter
            .match_any(&bh, std::iter::once(removal_id.as_ref()))
            .expect("match"),
        "filter must contain removal coin_id"
    );
}

/// **Test plan:** `test_filter_hash_both` — puzzle hash and removal id both match.
#[test]
fn hsh006_both_additions_and_removals() {
    let ph = Bytes32::new([0x88; 32]);
    let removal_id = Bytes32::new([0x99; 32]);
    let block_id = Bytes32::new([0x03; 32]);
    let coin = Coin::new(Bytes32::new([0xbb; 32]), ph, 2);
    let additions = vec![coin];
    let removals = vec![removal_id];

    let encoded = compact_block_filter_encoded(block_id, &additions, &removals).expect("encode");
    let filter = BlockFilter::new(&encoded);
    let bh = block_hash_from_identity(block_id);
    assert!(filter.match_any(&bh, std::iter::once(ph.as_ref())).unwrap());
    assert!(filter
        .match_any(&bh, std::iter::once(removal_id.as_ref()))
        .unwrap());
}

/// **Test plan:** `test_filter_membership_negative` — element not in constructed set (BIP-158 FP possible but unlikely here).
#[test]
fn hsh006_membership_negative_unlikely_element() {
    let ph = Bytes32::new([0x77; 32]);
    let block_id = Bytes32::new([0x04; 32]);
    let coin = Coin::new(Bytes32::new([0xcc; 32]), ph, 1);
    let additions = vec![coin];
    let removals: Vec<Bytes32> = vec![];

    let encoded = compact_block_filter_encoded(block_id, &additions, &removals).expect("encode");
    let filter = BlockFilter::new(&encoded);
    let bh = block_hash_from_identity(block_id);
    let not_in = Bytes32::new([0u8; 32]);
    assert!(
        !filter
            .match_any(&bh, std::iter::once(not_in.as_ref()))
            .expect("match"),
        "all-zero id should not appear in this tiny filter (if flaky, BIP-158 false positive — change pattern)"
    );
}

/// **Test plan:** `test_filter_hash_deterministic`.
#[test]
fn hsh006_deterministic_repeat() {
    let b = bundle_create_coin_with_ph(Bytes32::new([0xab; 32]));
    let block = block_with_bundles(vec![b]);
    let adds = block.all_additions();
    let rems = block.all_removals();
    let id = block.hash();
    assert_eq!(
        compute_filter_hash(id, &adds, &rems),
        compute_filter_hash(id, &adds, &rems)
    );
}

/// **Test plan:** `L2Block` uses the same normative function as the crate root.
#[test]
fn hsh006_l2block_delegates() {
    let b = bundle_create_coin_with_ph(Bytes32::new([0xcd; 32]));
    let block = block_with_bundles(vec![b]);
    let adds = block.all_additions();
    let rems = block.all_removals();
    assert_eq!(
        block.compute_filter_hash(),
        compute_filter_hash(block.hash(), &adds, &rems)
    );
}
