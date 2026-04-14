//! BLK-002: `L2BlockHeader` constructors and shared version detection.
//!
//! **Authoritative spec:** `docs/requirements/domains/block_types/specs/BLK-002.md`
//! **Normative:** `docs/requirements/domains/block_types/NORMATIVE.md` (BLK-002)
//! **Wire / genesis:** `docs/resources/SPEC.md` §2.2 (derived `new`), §8.3 (genesis)
//!
//! Implements the BLK-002 **Test Plan**. These tests prove public constructors never take a `version`
//! argument (callers cannot create height/version skew), that collateral and full L1 proof variants
//! populate the optional anchor fields correctly, and that genesis matches the network bootstrap layout.

use dig_block::{
    Bytes32, Cost, L2BlockHeader, DFSP_ACTIVATION_HEIGHT, EMPTY_ROOT, VERSION_V1, ZERO_HASH,
};

fn tag_byte(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// **Test plan:** `test_new_constructor` — [`L2BlockHeader::new`] fills defaults and sets `version` from height.
///
/// With default `DFSP_ACTIVATION_HEIGHT == u64::MAX`, [`L2BlockHeader::protocol_version_for_height`] must
/// yield [`VERSION_V1`] for any practical height (BLK-007 / SPEC). We also assert SPEC defaults: L1 proofs
/// absent, `extension_data == ZERO_HASH`, empty slash summary, DFSP roots `EMPTY_ROOT`, `timestamp == 0`.
#[test]
fn test_new_constructor() {
    let h = L2BlockHeader::new(
        7,
        1,
        tag_byte(0x01),
        tag_byte(0x02),
        tag_byte(0x03),
        tag_byte(0x04),
        tag_byte(0x05),
        tag_byte(0x06),
        100,
        tag_byte(0x07),
        2,
        3,
        400 as Cost,
        50,
        8,
        9,
        5000,
        tag_byte(0x08),
    );

    assert_eq!(h.version, VERSION_V1);
    assert_eq!(h.version, L2BlockHeader::protocol_version_for_height(7));
    assert_eq!(h.height, 7);
    assert_eq!(h.epoch, 1);
    assert_eq!(h.timestamp, 0);
    assert_eq!(h.extension_data, ZERO_HASH);
    assert!(h.l1_collateral_coin_id.is_none());
    assert!(h.l1_network_coin_id.is_none());
    assert_eq!(h.slash_proposal_count, 0);
    assert_eq!(h.slash_proposals_root, EMPTY_ROOT);
    assert_eq!(h.collateral_registry_root, EMPTY_ROOT);
    assert_eq!(h.dfsp_finalize_commitment_root, EMPTY_ROOT);
}

/// **Test plan:** `test_new_with_collateral` — collateral proof lands in `l1_collateral_coin_id` only.
#[test]
fn test_new_with_collateral() {
    let collateral = tag_byte(0xcc);
    let h = L2BlockHeader::new_with_collateral(
        1,
        0,
        tag_byte(0x01),
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        1,
        tag_byte(0x02),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        EMPTY_ROOT,
        collateral,
    );

    assert_eq!(h.l1_collateral_coin_id, Some(collateral));
    assert!(h.l1_reserve_coin_id.is_none());
    assert!(h.l1_prev_epoch_finalizer_coin_id.is_none());
    assert!(h.l1_curr_epoch_finalizer_coin_id.is_none());
    assert!(h.l1_network_coin_id.is_none());
}

/// **Test plan:** `test_new_with_l1_proofs` — all five optional L1 anchors are `Some`.
#[test]
fn test_new_with_l1_proofs() {
    let c = tag_byte(0x10);
    let r = tag_byte(0x11);
    let pf = tag_byte(0x12);
    let cf = tag_byte(0x13);
    let n = tag_byte(0x14);

    let h = L2BlockHeader::new_with_l1_proofs(
        2,
        0,
        tag_byte(0x01),
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        1,
        tag_byte(0x02),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        EMPTY_ROOT,
        c,
        r,
        pf,
        cf,
        n,
    );

    assert_eq!(h.l1_collateral_coin_id, Some(c));
    assert_eq!(h.l1_reserve_coin_id, Some(r));
    assert_eq!(h.l1_prev_epoch_finalizer_coin_id, Some(pf));
    assert_eq!(h.l1_curr_epoch_finalizer_coin_id, Some(cf));
    assert_eq!(h.l1_network_coin_id, Some(n));
}

/// **Test plan:** `test_genesis_constructor` — `parent_hash == network_id`, roots `EMPTY_ROOT`, extension `ZERO_HASH`.
///
/// Proves BLK-002 / SPEC §8.3 structural bootstrap. **`timestamp`** is wall-clock in [`L2BlockHeader::genesis`];
/// we only assert it is non-zero with high probability rather than fixing clock injection in this crate test.
#[test]
fn test_genesis_constructor() {
    let network_id = tag_byte(0x99);
    let l1_h = 12_345_u32;
    let l1_id = tag_byte(0xaa);

    let h = L2BlockHeader::genesis(network_id, l1_h, l1_id);

    assert_eq!(h.parent_hash, network_id);
    assert_eq!(h.l1_height, l1_h);
    assert_eq!(h.l1_hash, l1_id);
    assert_eq!(h.epoch, 0);
    assert_eq!(h.state_root, EMPTY_ROOT);
    assert_eq!(h.spends_root, EMPTY_ROOT);
    assert_eq!(h.receipts_root, EMPTY_ROOT);
    assert_eq!(h.filter_hash, EMPTY_ROOT);
    assert_eq!(h.extension_data, ZERO_HASH);
    assert_eq!(h.spend_bundle_count, 0);
    assert_eq!(h.additions_count, 0);
    assert_eq!(h.removals_count, 0);
    assert_eq!(h.slash_proposal_count, 0);
    assert_eq!(h.total_cost, 0);
    assert_eq!(h.total_fees, 0);
    assert_eq!(h.block_size, 0);
    assert!(h.l1_collateral_coin_id.is_none());
    assert_ne!(
        h.timestamp, 0,
        "genesis uses wall-clock timestamp (SPEC §8.3)"
    );
}

/// **Test plan:** `test_genesis_height_zero` — genesis chain position is height 0 with `VERSION_V1` under default DFSP-off config.
#[test]
fn test_genesis_height_zero() {
    let h = L2BlockHeader::genesis(tag_byte(0x01), 0, tag_byte(0x02));
    assert_eq!(h.height, 0);
    assert_eq!(h.version, VERSION_V1);
}

/// **Test plan:** `test_no_manual_version` — version is not a constructor parameter; it matches [`protocol_version_for_height`].
///
/// Rust cannot execute a negative compile test in this file without `trybuild`. The **public API surface**
/// (`new`, `new_with_collateral`, `new_with_l1_proofs`, `genesis`) omits `version`; this test locks the
/// runtime invariant that `header.version == L2BlockHeader::protocol_version_for_height(header.height)` for
/// [`L2BlockHeader::new`] outputs.
#[test]
fn test_no_manual_version() {
    let h = L2BlockHeader::new(
        99,
        0,
        tag_byte(1),
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        0,
        tag_byte(2),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        EMPTY_ROOT,
    );
    assert_eq!(
        h.version,
        L2BlockHeader::protocol_version_for_height(h.height)
    );
}

/// **Regression:** when DFSP is disabled (`DFSP_ACTIVATION_HEIGHT == u64::MAX`), post-activation heights still use V1.
///
/// Documents the BLK-007 special case exercised by [`L2BlockHeader::protocol_version_for_height`]. If
/// governance lowers `DFSP_ACTIVATION_HEIGHT` in a future release, V2 coverage moves to BLK-007.
#[test]
fn test_version_v1_when_dfsp_disabled_constant() {
    assert_eq!(DFSP_ACTIVATION_HEIGHT, u64::MAX);
    assert_eq!(
        L2BlockHeader::protocol_version_for_height(u64::MAX - 1),
        VERSION_V1
    );
    assert_eq!(L2BlockHeader::protocol_version_for_height(0), VERSION_V1);
}
