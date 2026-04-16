//! HSH-001: [`L2BlockHeader::hash`] — SHA-256 over the fixed-order header preimage ([SPEC §3.1](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/hashing/NORMATIVE.md` (HSH-001)  
//! **Spec + test plan:** `docs/requirements/domains/hashing/specs/HSH-001.md`  
//! **Implementation:** `src/types/header.rs` — [`L2BlockHeader::hash_preimage_bytes`], [`L2BlockHeader::HASH_PREIMAGE_LEN`]  
//! **Layout note:** HSH-001 / older SPEC prose cited “626 bytes”; summing the §3.1 table (33 fields) yields **710** —
//! the code constant is authoritative (see header module docs).
//!
//! ## How these tests prove HSH-001
//!
//! - **Determinism:** Same header → same [`Bytes32`] twice ([`chia_sha2::Sha256`]).
//! - **Preimage agreement:** [`L2BlockHeader::hash`] equals SHA-256 of [`L2BlockHeader::hash_preimage_bytes`] — proves
//!   `hash()` is not an alternate code path.
//! - **Little-endian:** Known `version` yields expected low byte first in the preimage.
//! - **Optionals:** `None` →32 bytes of [`ZERO_HASH`]; `Some` → raw coin id bytes at the same slot ([malleability](docs/requirements/domains/hashing/specs/HSH-001.md)).
//! - **Sensitivity:** Perturbing one scalar changes the digest (spot-check + preimage length).
//! - **SocratiCode:** Not available here; Repomix + `SPEC.md` §3.1 table used for field order.

use chia_sha2::Sha256;
use dig_block::{Bytes32, Cost, L2BlockHeader, VERSION_V1, ZERO_HASH};

/// Byte offset in [`L2BlockHeader::hash_preimage_bytes`] where the five L1 optional anchors start (after `extension_data`).
///
/// **Derivation:** 2+8+8 + 6×32 + 4+32 + (8+4+4+8+8+4+4+4) + 32+32 = **354** (matches `header.rs` serialization order).
const L1_OPTIONALS_PREIMAGE_OFFSET: usize = 354;

fn sample_filled_header() -> L2BlockHeader {
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
        collateral_registry_root: dig_block::EMPTY_ROOT,
        cid_state_root: b(0x21),
        node_registry_root: b(0x22),
        namespace_update_root: b(0x23),
        dfsp_finalize_commitment_root: b(0x24),
    }
}

/// **Test plan:** `test_header_hash_deterministic`
#[test]
fn hsh001_hash_is_deterministic() {
    let h = sample_filled_header();
    assert_eq!(h.hash(), h.hash());
}

/// **Test plan:** `test_header_hash_field_order` — manual SHA-256(preimage) matches `hash()`.
#[test]
fn hsh001_hash_matches_sha256_of_preimage() {
    let h = sample_filled_header();
    let pre = h.hash_preimage_bytes();
    let mut hasher = Sha256::new();
    hasher.update(pre);
    let manual = Bytes32::new(hasher.finalize());
    assert_eq!(h.hash(), manual);
}

/// **Test plan:** `test_header_hash_le_encoding` — `u16` version is little-endian in the first two bytes.
#[test]
fn hsh001_version_is_little_endian_in_preimage() {
    let mut h = sample_filled_header();
    h.version = 0x0102;
    let pre = h.hash_preimage_bytes();
    assert_eq!(pre[0], 0x02);
    assert_eq!(pre[1], 0x01);
}

/// **Test plan:** `test_header_hash_total_bytes`
#[test]
fn hsh001_preimage_length_is_constant() {
    let h = sample_filled_header();
    assert_eq!(
        h.hash_preimage_bytes().len(),
        L2BlockHeader::HASH_PREIMAGE_LEN
    );
    assert_eq!(L2BlockHeader::HASH_PREIMAGE_LEN, 710);
}

/// **Test plan:** `test_header_hash_optional_none` — each missing L1 anchor hashes as [`ZERO_HASH`].
#[test]
fn hsh001_optional_none_inserts_zero_hash_for_each_slot() {
    let h = L2BlockHeader::new(
        1,
        0,
        Bytes32::new([1; 32]),
        Bytes32::new([2; 32]),
        Bytes32::new([3; 32]),
        Bytes32::new([4; 32]),
        Bytes32::new([5; 32]),
        Bytes32::new([6; 32]),
        100,
        Bytes32::new([7; 32]),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        Bytes32::new([8; 32]),
    );
    let p = h.hash_preimage_bytes();
    for k in 0..5 {
        let start = L1_OPTIONALS_PREIMAGE_OFFSET + k * 32;
        assert_eq!(&p[start..start + 32], ZERO_HASH.as_ref(), "slot {k}");
    }
}

/// **Test plan:** `test_header_hash_optional_some`
#[test]
fn hsh001_optional_some_inserts_actual_bytes32() {
    let h = sample_filled_header();
    let p = h.hash_preimage_bytes();
    let first = h.l1_collateral_coin_id.unwrap();
    assert_eq!(
        &p[L1_OPTIONALS_PREIMAGE_OFFSET..L1_OPTIONALS_PREIMAGE_OFFSET + 32],
        first.as_ref()
    );
}

/// **Test plan:** `test_header_hash_single_field_change`
#[test]
fn hsh001_changing_height_changes_hash() {
    let a = sample_filled_header();
    let mut b = sample_filled_header();
    b.height = 101;
    assert_ne!(a.hash(), b.hash());
}

/// **Test plan:** `total_cost` LE bytes appear at the expected offset (after `spend_bundle_count`).
#[test]
fn hsh001_total_cost_little_endian_in_preimage() {
    let mut h = sample_filled_header();
    h.total_cost = 0x0102_0304_0506_0708;
    let pre = h.hash_preimage_bytes();
    // Offset: 2+8+8+192 +4+32 +8+4+4 = 262, then total_cost 8 bytes
    const OFF: usize = 2 + 8 + 8 + 6 * 32 + 4 + 32 + 8 + 4 + 4;
    assert_eq!(&pre[OFF..OFF + 8], &h.total_cost.to_le_bytes());
}
