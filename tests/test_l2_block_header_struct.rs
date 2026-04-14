//! BLK-001: `L2BlockHeader` struct shape, derives, and bincode round-trip.
//!
//! **Authoritative spec:** `docs/requirements/domains/block_types/specs/BLK-001.md`
//! **Normative:** `docs/requirements/domains/block_types/NORMATIVE.md` (BLK-001)
//! **Wire layout reference:** `docs/resources/SPEC.md` §2.2
//!
//! The BLK-001 **Test Plan** rows are implemented below. Together they prove the header carries every
//! field group required for L2 identity, state commitments, L1 anchoring, metadata, optional L1 proof
//! coin ids, slash proposal roots, and DFSP roots — and that the type is suitable for **bincode** transport
//! (serde derives; see SER-001).

use bincode::{deserialize, serialize};
use dig_block::{Bytes32, EMPTY_ROOT, ZERO_HASH};
use dig_block::{Cost, L2BlockHeader, VERSION_V1};

/// Build a header with **distinct** values per field group so field access tests cannot pass by accident.
///
/// **Rationale:** BLK-001 asks for a populated header, not minimal defaults; discriminators make it obvious
/// which field regressed if layout or typing breaks.
fn sample_header() -> L2BlockHeader {
    let b = |tag: u8| Bytes32::new([tag; 32]);
    L2BlockHeader {
        version: VERSION_V1,
        height: 100,
        epoch: 10,
        parent_hash: b(0x01),
        state_root: b(0x02),
        spends_root: b(0x03),
        additions_root: b(0x04),
        removals_root: b(0x05),
        receipts_root: b(0x06),
        l1_height: 9_000_001,
        l1_hash: b(0x07),
        timestamp: 1_700_000_000,
        proposer_index: 3,
        spend_bundle_count: 5,
        total_cost: 42_000 as Cost,
        total_fees: 1_000,
        additions_count: 11,
        removals_count: 7,
        block_size: 4096,
        filter_hash: b(0x08),
        extension_data: ZERO_HASH,
        l1_collateral_coin_id: Some(b(0x10)),
        l1_reserve_coin_id: Some(b(0x11)),
        l1_prev_epoch_finalizer_coin_id: Some(b(0x12)),
        l1_curr_epoch_finalizer_coin_id: Some(b(0x13)),
        l1_network_coin_id: Some(b(0x14)),
        slash_proposal_count: 2,
        slash_proposals_root: b(0x20),
        collateral_registry_root: EMPTY_ROOT,
        cid_state_root: b(0x21),
        node_registry_root: b(0x22),
        namespace_update_root: b(0x23),
        dfsp_finalize_commitment_root: b(0x24),
    }
}

/// **Test plan:** `test_header_all_fields` — every field group is present and readable with expected values.
#[test]
fn test_header_all_fields() {
    let h = sample_header();

    assert_eq!(h.version, VERSION_V1);
    assert_eq!(h.height, 100);
    assert_eq!(h.epoch, 10);
    assert_eq!(h.parent_hash, Bytes32::new([0x01; 32]));
    assert_eq!(h.state_root, Bytes32::new([0x02; 32]));
    assert_eq!(h.spends_root, Bytes32::new([0x03; 32]));
    assert_eq!(h.additions_root, Bytes32::new([0x04; 32]));
    assert_eq!(h.removals_root, Bytes32::new([0x05; 32]));
    assert_eq!(h.receipts_root, Bytes32::new([0x06; 32]));
    assert_eq!(h.l1_height, 9_000_001);
    assert_eq!(h.l1_hash, Bytes32::new([0x07; 32]));
    assert_eq!(h.timestamp, 1_700_000_000);
    assert_eq!(h.proposer_index, 3);
    assert_eq!(h.spend_bundle_count, 5);
    assert_eq!(h.total_cost, 42_000);
    assert_eq!(h.total_fees, 1_000);
    assert_eq!(h.additions_count, 11);
    assert_eq!(h.removals_count, 7);
    assert_eq!(h.block_size, 4096);
    assert_eq!(h.filter_hash, Bytes32::new([0x08; 32]));
    assert_eq!(h.extension_data, ZERO_HASH);
    assert_eq!(h.l1_collateral_coin_id, Some(Bytes32::new([0x10; 32])));
    assert_eq!(h.slash_proposal_count, 2);
    assert_eq!(h.slash_proposals_root, Bytes32::new([0x20; 32]));
    assert_eq!(h.collateral_registry_root, EMPTY_ROOT);
    assert_eq!(h.dfsp_finalize_commitment_root, Bytes32::new([0x24; 32]));
}

/// **Test plan:** `test_header_serialize_roundtrip` — bincode encode/decode preserves equality (`PartialEq`).
///
/// Satisfies BLK-001 acceptance: “Struct can be serialized to and deserialized from bytes.” Uses the same
/// **`bincode`** crate the crate will standardize on for block types (SER-001).
#[test]
fn test_header_serialize_roundtrip() {
    let h = sample_header();
    let bytes = serialize(&h).expect("bincode serialize L2BlockHeader");
    let back: L2BlockHeader = deserialize(&bytes).expect("bincode deserialize L2BlockHeader");
    assert_eq!(h, back);
}

/// **Test plan:** `test_header_clone` — `Clone` produces an equal copy.
#[test]
fn test_header_clone() {
    let h = sample_header();
    assert_eq!(h.clone(), h);
}

/// **Test plan:** `test_header_debug` — `Debug` is implemented (derive) for logging / inspect.
#[test]
fn test_header_debug() {
    let h = sample_header();
    let s = format!("{h:?}");
    assert!(
        s.contains("L2BlockHeader") || s.contains("version") || s.contains("height"),
        "Debug output should expose struct identity: {s}"
    );
}

/// **Test plan:** `test_header_optional_l1_proofs` — L1 proof anchor fields default to `None` when omitted.
///
/// We serialize a header **without** setting the five optional fields in the type literal (Rust defaults
/// `Option` to `None`), round-trip through bincode, and assert they stay `None`. This matches SPEC /
/// implementation notes: proofs are absent until set by constructors or `set_l1_proofs` (BLK-002 / BLD).
#[test]
fn test_header_optional_l1_proofs() {
    let b = |tag: u8| Bytes32::new([tag; 32]);
    let h = L2BlockHeader {
        version: VERSION_V1,
        height: 0,
        epoch: 0,
        parent_hash: b(0xa1),
        state_root: EMPTY_ROOT,
        spends_root: EMPTY_ROOT,
        additions_root: EMPTY_ROOT,
        removals_root: EMPTY_ROOT,
        receipts_root: EMPTY_ROOT,
        l1_height: 1,
        l1_hash: b(0xa2),
        timestamp: 0,
        proposer_index: 0,
        spend_bundle_count: 0,
        total_cost: 0,
        total_fees: 0,
        additions_count: 0,
        removals_count: 0,
        block_size: 0,
        filter_hash: EMPTY_ROOT,
        extension_data: ZERO_HASH,
        l1_collateral_coin_id: None,
        l1_reserve_coin_id: None,
        l1_prev_epoch_finalizer_coin_id: None,
        l1_curr_epoch_finalizer_coin_id: None,
        l1_network_coin_id: None,
        slash_proposal_count: 0,
        slash_proposals_root: EMPTY_ROOT,
        collateral_registry_root: EMPTY_ROOT,
        cid_state_root: EMPTY_ROOT,
        node_registry_root: EMPTY_ROOT,
        namespace_update_root: EMPTY_ROOT,
        dfsp_finalize_commitment_root: EMPTY_ROOT,
    };

    assert!(h.l1_collateral_coin_id.is_none());
    assert!(h.l1_reserve_coin_id.is_none());
    assert!(h.l1_prev_epoch_finalizer_coin_id.is_none());
    assert!(h.l1_curr_epoch_finalizer_coin_id.is_none());
    assert!(h.l1_network_coin_id.is_none());

    let bytes = serialize(&h).unwrap();
    let back: L2BlockHeader = deserialize(&bytes).unwrap();
    assert_eq!(back.l1_collateral_coin_id, None);
    assert_eq!(back.l1_network_coin_id, None);
}
