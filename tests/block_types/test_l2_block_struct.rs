//! BLK-003: `L2Block` — fields, `new`, and delegation of `hash` / `height` / `epoch` to the header.
//!
//! **Authoritative spec:** `docs/requirements/domains/block_types/specs/BLK-003.md`
//! **Normative:** `docs/requirements/domains/block_types/NORMATIVE.md` (BLK-003)
//! **Header hash (delegation target):** [HSH-001](docs/requirements/domains/hashing/specs/HSH-001.md) /
//! [`L2BlockHeader::hash`](dig_block::L2BlockHeader::hash)
//!
//! ## What this proves
//!
//! - The struct exposes the four required fields with the required types (`SpendBundle` from `chia-protocol`,
//!   [`dig_block::Signature`] re-exporting `chia-bls`).
//! - [`L2Block::hash`] returns the **same** [`Bytes32`] as [`L2BlockHeader::hash`] on the embedded header,
//!   demonstrating that block identity is header-only per SPEC §2.3 / BLK-003 implementation notes.
//! - [`L2Block::height`] and [`L2Block::epoch`] match the header scalars.
//! - Empty `Vec`s for spend bundles and slash payloads still produce a well-formed value (boundary cases in
//!   the BLK-003 test plan).

use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use dig_block::{Bytes32, Cost, L2Block, L2BlockHeader, Signature};

/// Minimal structurally valid [`SpendBundle`] for field-typing tests (not consensus-valid).
///
/// **Rationale:** BLK-003 requires `Vec<SpendBundle>`; one bundle proves non-empty body paths without pulling
/// in `tests/common` (this integration test binary stays self-contained).
fn minimal_spend_bundle() -> SpendBundle {
    let coin = Coin::new(Bytes32::default(), Bytes32::default(), 1);
    let coin_spend = CoinSpend::new(coin, Program::from(vec![0x01]), Program::from(vec![0x80]));
    SpendBundle::new(vec![coin_spend], Signature::default())
}

/// Header suitable for [`L2Block::new`] tests: deterministic roots, height 7 / epoch 3.
fn sample_header() -> L2BlockHeader {
    let tag = |b: u8| Bytes32::new([b; 32]);
    L2BlockHeader::new(
        7,
        3,
        tag(0x01),
        tag(0x02),
        tag(0x03),
        tag(0x04),
        tag(0x05),
        tag(0x06),
        100,
        tag(0x07),
        2,
        1,
        100 as Cost,
        0,
        0,
        0,
        0,
        tag(0x08),
    )
}

/// **Test plan:** `test_l2block_new` — [`L2Block::new`] stores all arguments; field accessors match.
///
/// **Proof:** We pass a distinct header, one spend bundle, one slash payload, and a non-default signature;
/// each field read back equals the input. This directly satisfies BLK-003 “constructor accepting all fields”.
#[test]
fn test_l2block_new() {
    let header = sample_header();
    let sb = minimal_spend_bundle();
    let slash = vec![vec![0xde, 0xad]];
    let sig = Signature::default();

    let block = L2Block::new(header.clone(), vec![sb.clone()], slash.clone(), sig.clone());

    assert_eq!(block.header, header);
    assert_eq!(block.spend_bundles.len(), 1);
    assert_eq!(block.slash_proposal_payloads, slash);
    assert_eq!(block.proposer_signature, sig);
}

/// **Test plan:** `test_l2block_hash_delegates` — [`L2Block::hash`] equals [`L2BlockHeader::hash`] on the same header.
///
/// **Proof:** If delegation were wrong (e.g. hashing body), `block.hash()` would diverge from `header.hash()`.
/// Equality proves BLK-003’s “hash() delegates to header.hash()”.
#[test]
fn test_l2block_hash_delegates() {
    let header = sample_header();
    let h = header.hash();
    let block = L2Block::new(
        header,
        vec![minimal_spend_bundle()],
        vec![vec![1, 2, 3]],
        Signature::default(),
    );
    assert_eq!(block.hash(), h);
    assert_eq!(block.hash(), block.header.hash());
}

/// **Test plan:** `test_l2block_height_delegates` — [`L2Block::height`] is `header.height`.
#[test]
fn test_l2block_height_delegates() {
    let header = sample_header();
    let expected = header.height;
    let block = L2Block::new(header, vec![], vec![], Signature::default());
    assert_eq!(block.height(), expected);
}

/// **Test plan:** `test_l2block_epoch_delegates` — [`L2Block::epoch`] is `header.epoch`.
#[test]
fn test_l2block_epoch_delegates() {
    let header = sample_header();
    let expected = header.epoch;
    let block = L2Block::new(header, vec![], vec![], Signature::default());
    assert_eq!(block.epoch(), expected);
}

/// **Test plan:** `test_l2block_empty_spend_bundles` — `spend_bundles` may be empty.
///
/// **Proof:** Type system + successful construction; callers may build blocks with zero spends before
/// validation rules (e.g. header `spend_bundle_count` consistency) are applied elsewhere.
#[test]
fn test_l2block_empty_spend_bundles() {
    let header = sample_header();
    let block = L2Block::new(header, vec![], vec![], Signature::default());
    assert!(block.spend_bundles.is_empty());
}

/// **Test plan:** `test_l2block_empty_slash_proposals` — `slash_proposal_payloads` may be empty.
#[test]
fn test_l2block_empty_slash_proposals() {
    let header = sample_header();
    let block = L2Block::new(
        header,
        vec![minimal_spend_bundle()],
        vec![],
        Signature::default(),
    );
    assert!(block.slash_proposal_payloads.is_empty());
}

/// **Test plan (extra):** serde round-trip preserves data (SER-001 alignment; BLK-003 uses serde derives).
#[test]
fn test_l2block_serde_roundtrip() {
    let block = L2Block::new(
        sample_header(),
        vec![minimal_spend_bundle()],
        vec![vec![0xab]],
        Signature::default(),
    );
    let bytes = bincode::serialize(&block).expect("serialize");
    let back: L2Block = bincode::deserialize(&bytes).expect("deserialize");
    assert_eq!(back.header, block.header);
    assert_eq!(back.spend_bundles.len(), block.spend_bundles.len());
    assert_eq!(back.slash_proposal_payloads, block.slash_proposal_payloads);
}
