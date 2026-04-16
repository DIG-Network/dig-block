//! SER-003: Genesis L2 header via [`L2BlockHeader::genesis`](dig_block::L2BlockHeader::genesis)
//! ([NORMATIVE § SER-003](docs/requirements/domains/serialization/NORMATIVE.md#ser-003-genesis-block-construction),
//! [spec](docs/requirements/domains/serialization/specs/SER-003.md), SPEC §8.3).
//!
//! ## Scope
//!
//! This file is the **dedicated** integration gate for SER-003 (flat `tests/test_ser_003_genesis_header.rs` per STR-002).
//! BLK-002 / `tests/test_l2_block_header_constructors.rs` already smoke-tests genesis for constructor coverage; here we
//! enumerate **every** NORMATIVE bullet and the SER-003 test-plan rows so a reader can trace requirement → proof in one place.
//!
//! ## Proof strategy
//!
//! | NORMATIVE / spec obligation | How we prove it |
//! |------------------------------|-----------------|
//! | `height == 0`, `epoch == 0` | Direct equality on [`L2BlockHeader::height`] / [`L2BlockHeader::epoch`] |
//! | `parent_hash == network_id` | Compare to caller-supplied [`Bytes32`] |
//! | Merkle / DFSP roots + `filter_hash` == [`EMPTY_ROOT`](dig_block::EMPTY_ROOT) | Field-by-field equality |
//! | `extension_data == `[`ZERO_HASH`](dig_block::ZERO_HASH) | Equality |
//! | Counts, fees, cost, `block_size` == 0 | Equality on numeric fields |
//! | All optional L1 proof slots == `None` | [`Option::is_none`] on five ids |
//! | `timestamp` ≈ wall clock | Compare to [`std::time::SystemTime::now`] within **5 seconds** (spec test plan) |
//! | `version` follows BLK-007 | [`dig_block::VERSION_V1`] at height 0 with default `DFSP_ACTIVATION_HEIGHT`, and matches [`L2BlockHeader::protocol_version_for_height`](dig_block::L2BlockHeader::protocol_version_for_height)(`0`) |
//! | Wire round-trip | [`L2BlockHeader::to_bytes`] / [`L2BlockHeader::from_bytes`] ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md)) |

use std::time::{SystemTime, UNIX_EPOCH};

use dig_block::{
    Bytes32, L2BlockHeader, DFSP_ACTIVATION_HEIGHT, EMPTY_ROOT, VERSION_V1, ZERO_HASH,
};

fn tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// **Test plan:** `genesis_height`, `genesis_epoch`, `genesis_parent_hash`, `genesis_l1` fields — chain position and anchors.
#[test]
fn ser003_genesis_height_epoch_parent_and_l1_anchors() {
    let network_id = tag(0x77);
    let l1_h = 9_001_u32;
    let l1_hash = tag(0x55);
    let h = L2BlockHeader::genesis(network_id, l1_h, l1_hash);
    assert_eq!(h.height, 0, "SER-003 / NORMATIVE: genesis height must be 0");
    assert_eq!(h.epoch, 0, "SER-003 / NORMATIVE: genesis epoch must be 0");
    assert_eq!(
        h.parent_hash, network_id,
        "SER-003: parent_hash binds to network_id (no prior L2 parent)"
    );
    assert_eq!(h.l1_height, l1_h);
    assert_eq!(h.l1_hash, l1_hash);
}

/// **Test plan:** `genesis_roots_empty`, `genesis_extension_zero` — commitment roots and extension slot.
#[test]
fn ser003_genesis_roots_filter_dfsp_and_extension() {
    let h = L2BlockHeader::genesis(tag(0x01), 0, tag(0x02));
    assert_eq!(h.state_root, EMPTY_ROOT);
    assert_eq!(h.spends_root, EMPTY_ROOT);
    assert_eq!(h.additions_root, EMPTY_ROOT);
    assert_eq!(h.removals_root, EMPTY_ROOT);
    assert_eq!(h.receipts_root, EMPTY_ROOT);
    assert_eq!(h.filter_hash, EMPTY_ROOT);
    assert_eq!(h.slash_proposals_root, EMPTY_ROOT);
    assert_eq!(h.collateral_registry_root, EMPTY_ROOT);
    assert_eq!(h.cid_state_root, EMPTY_ROOT);
    assert_eq!(h.node_registry_root, EMPTY_ROOT);
    assert_eq!(h.namespace_update_root, EMPTY_ROOT);
    assert_eq!(h.dfsp_finalize_commitment_root, EMPTY_ROOT);
    assert_eq!(
        h.extension_data, ZERO_HASH,
        "SER-003: extension_data starts at ZERO_HASH"
    );
}

/// **Test plan:** `genesis_counts_zero`, `genesis_l1_proofs_none` — scalar zeros and unset L1 proof coins.
#[test]
fn ser003_genesis_counts_proposer_and_optional_l1_slots() {
    let h = L2BlockHeader::genesis(tag(0x03), 0, tag(0x04));
    assert_eq!(h.proposer_index, 0);
    assert_eq!(h.spend_bundle_count, 0);
    assert_eq!(h.total_cost, 0);
    assert_eq!(h.total_fees, 0);
    assert_eq!(h.additions_count, 0);
    assert_eq!(h.removals_count, 0);
    assert_eq!(h.block_size, 0);
    assert_eq!(h.slash_proposal_count, 0);
    assert!(h.l1_collateral_coin_id.is_none());
    assert!(h.l1_reserve_coin_id.is_none());
    assert!(h.l1_prev_epoch_finalizer_coin_id.is_none());
    assert!(h.l1_curr_epoch_finalizer_coin_id.is_none());
    assert!(h.l1_network_coin_id.is_none());
}

/// **Test plan:** `genesis_timestamp_recent` — wall-clock binding (SPEC §8.3).
#[test]
fn ser003_genesis_timestamp_within_five_seconds_of_wall_clock() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("host clock after epoch")
        .as_secs();
    let h = L2BlockHeader::genesis(tag(0xab), 0, tag(0xcd));
    let skew = h.timestamp.abs_diff(now);
    assert!(
        skew <= 5,
        "SER-003 test plan: timestamp within 5s of wall clock (skew={skew}, now={now}, ts={})",
        h.timestamp
    );
}

/// **Test plan:** `genesis_version_detected` — BLK-007 auto-version at height 0 (NORMATIVE errata: not `CARGO_PKG_VERSION`).
#[test]
fn ser003_genesis_version_matches_protocol_height_zero() {
    assert_eq!(
        DFSP_ACTIVATION_HEIGHT,
        u64::MAX,
        "fixture assumes DFSP-off sentinel so height 0 stays on VERSION_V1 (BLK-007)"
    );
    let h = L2BlockHeader::genesis(tag(0xee), 0, tag(0xff));
    assert_eq!(h.version, VERSION_V1);
    assert_eq!(
        h.version,
        L2BlockHeader::protocol_version_for_height(0),
        "genesis must use the same version rule as every other header at height 0"
    );
}

/// **Test plan:** `genesis_serializable` — bincode helpers from SER-002 preserve genesis layout.
#[test]
fn ser003_genesis_round_trips_through_to_bytes() {
    let h = L2BlockHeader::genesis(tag(0x11), 42, tag(0x22));
    let bytes = h.to_bytes();
    let back = L2BlockHeader::from_bytes(&bytes).expect("genesis header must decode");
    assert_eq!(
        h, back,
        "SER-003: genesis must survive SER-002 wire round-trip"
    );
}
