//! BLD-005: [`dig_block::BlockBuilder::build`] / [`dig_block::BlockBuilder::build_with_dfsp_activation`] — finalize
//! accumulated body + optional BLD-004 header fields into a signed [`dig_block::L2Block`]
//! ([SPEC §6.4](docs/resources/SPEC.md),
//! [NORMATIVE — BLD-005](docs/requirements/domains/block_production/NORMATIVE.md)).
//!
//! **Authoritative spec:** `docs/requirements/domains/block_production/specs/BLD-005.md` (pseudocode, acceptance
//! criteria, test-plan table). **Flat test path:** `tests/test_bld_005_build_pipeline.rs` (STR-002 / `.cursor/rules/tests-layout.mdc`
//! — not the spec’s illustrative `tests/block_production/…` path).
//!
//! ## How these tests prove BLD-005
//!
//! - **Empty body:** [`dig_block::BuilderError::EmptyBlock`] when `spend_bundles` is empty proves the builder refuses
//!   to emit a header-only block without an explicit product decision elsewhere ([`dig_block::BlockBuilder::build`] doc
//! in `src/builder/block_builder.rs`).
//! - **Merkle / filter commitments:** After a successful `build`, recomputing [`dig_block::L2Block::compute_spends_root`],
//!   [`dig_block::L2Block::compute_additions_root`], [`dig_block::L2Block::compute_removals_root`],
//!   [`dig_block::L2Block::compute_filter_hash`], and [`dig_block::L2Block::compute_slash_proposals_root`] on the
//!   returned block and comparing to **header** fields proves steps 1–5 of the pipeline match BLK-004 / HSH-003–006.
//! - **Counts:** Header `spend_bundle_count`, `additions_count`, `removals_count` (sum of `coin_spends` per bundle),
//!   and `slash_proposal_count` match the body — the same invariants later enforced by SVL-005 inside
//!   [`dig_block::L2Block::validate_structure`].
//! - **Version (BLK-007):** With crate [`dig_block::DFSP_ACTIVATION_HEIGHT`] at `u64::MAX`, `build()` selects
//!   [`dig_block::VERSION_V1`]. [`dig_block::BlockBuilder::build_with_dfsp_activation`] with a **finite** activation
//!   height `≤ height` forces [`dig_block::VERSION_V2`] when DFSP roots are non-empty; when all five DFSP roots remain
//!   [`dig_block::EMPTY_ROOT`], [`dig_block::BuilderError::MissingDfspRoots`] proves the SVL-002-aligned precondition
//!   from the spec.
//! - **Timestamp:** [`dig_block::L2BlockHeader::timestamp`] lies within a small window of `SystemTime::now()` (Unix
//!   seconds), proving wall-clock assignment (spec step 8).
//! - **Two-pass `block_size`:** [`dig_block::L2BlockHeader::block_size`] (as `u64` / `u32`) equals
//!   [`dig_block::L2Block::compute_size`] on the final block — the field participates in serialization so the second
//!   pass must match the measured `bincode` length (spec step 9).
//! - **Signing:** [`chia_bls::verify`] on `header.hash()`, [`dig_block::MockBlockSigner`], and
//!   [`dig_block::L2Block::proposer_signature`] in the full-pipeline test; dedicated BLD-006 coverage lives in
//!   `tests/test_bld_006_block_signer_integration.rs`.
//! - **Structural tier:** [`dig_block::L2Block::validate_structure`] on the success path — full BLD-007 matrix lives in
//!   `tests/test_bld_007_builder_validity_guarantee.rs`.
//!
//! **Tooling:** Repomix packs under `.repomix/` for `src/`, `tests/`, `docs/requirements/domains/block_production`.
//! `npx gitnexus impact BlockBuilder` → **LOW** (no upstream crate callers at index time).

mod common;

use std::time::{SystemTime, UNIX_EPOCH};

use chia_bls::verify;
use chia_protocol::Bytes32;

use dig_block::{BlockBuilder, BuilderError, EMPTY_ROOT, VERSION_V1, VERSION_V2};

use common::{test_spend_bundle, MockBlockSigner};

fn mk_builder_v1_height() -> BlockBuilder {
    BlockBuilder::new(
        1,
        0,
        Bytes32::new([0xab; 32]),
        1,
        Bytes32::new([0xcd; 32]),
        0,
    )
}

/// **Test plan:** `test_build_empty_block` — empty spend list must not silently produce a block.
#[test]
fn bld005_build_rejects_empty_spend_bundles() {
    let b = mk_builder_v1_height();
    let signer = MockBlockSigner::new();
    let err = b
        .build(EMPTY_ROOT, EMPTY_ROOT, &signer)
        .expect_err("empty body is rejected");
    assert!(matches!(err, BuilderError::EmptyBlock));
}

/// **Test plan:** `test_build_integration` + individual root/count/version/timestamp/size/signing rows.
#[test]
fn bld005_build_full_pipeline_matches_body_and_passes_validate_structure() {
    let state = Bytes32::new([0x11; 32]);
    let receipts = Bytes32::new([0x22; 32]);
    let ext = Bytes32::new([0x33; 32]);

    let mut b = mk_builder_v1_height();
    b.set_extension_data(ext);
    b.add_spend_bundle(test_spend_bundle(), 42, 7)
        .expect("fixture bundle within budgets");
    b.add_slash_proposal(vec![0xde, 0xad])
        .expect("within slash caps");

    let signer = MockBlockSigner::new();
    let block = b
        .build(state, receipts, &signer)
        .expect("build succeeds with non-empty body");

    assert_eq!(block.header.state_root, state);
    assert_eq!(block.header.receipts_root, receipts);
    assert_eq!(block.header.extension_data, ext);
    assert_eq!(block.header.version, VERSION_V1);
    assert_eq!(block.header.total_cost, 42);
    assert_eq!(block.header.total_fees, 7);

    assert_eq!(block.header.spends_root, block.compute_spends_root());
    assert_eq!(block.header.additions_root, block.compute_additions_root());
    assert_eq!(block.header.removals_root, block.compute_removals_root());
    assert_eq!(block.header.filter_hash, block.compute_filter_hash());
    assert_eq!(
        block.header.slash_proposals_root,
        block.compute_slash_proposals_root()
    );

    assert_eq!(block.header.spend_bundle_count, 1);
    assert_eq!(
        block.header.additions_count as usize,
        block.all_additions().len()
    );
    let removal_rows: u32 = block
        .spend_bundles
        .iter()
        .map(|sb| sb.coin_spends.len() as u32)
        .sum();
    assert_eq!(block.header.removals_count, removal_rows);
    assert_eq!(block.header.slash_proposal_count, 1);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    assert!(
        block.header.timestamp + 120 >= now && block.header.timestamp <= now + 120,
        "timestamp {:?} should be near now {}",
        block.header.timestamp,
        now
    );

    let measured = block.compute_size();
    assert_eq!(
        block.header.block_size as usize, measured,
        "two-pass block_size must match bincode(L2Block) length"
    );

    let h = block.header.hash();
    assert!(verify(
        &block.proposer_signature,
        &signer.public_key(),
        h.as_ref()
    ));

    block
        .validate_structure()
        .expect("builder-produced block is SVL-005/006 consistent");
}

/// **Test plan:** `test_build_auto_version_v2` — post-activation height with real DFSP roots.
#[test]
fn bld005_build_with_dfsp_activation_selects_v2_when_roots_set() {
    let mut b = BlockBuilder::new(
        10,
        0,
        Bytes32::new([0x01; 32]),
        1,
        Bytes32::new([0x02; 32]),
        0,
    );
    b.add_spend_bundle(test_spend_bundle(), 1, 0)
        .expect("fixture");
    // At least one non-EMPTY root so MissingDfspRoots does not trigger (BLD-005 / SVL-002 interaction).
    b.set_dfsp_roots(
        Bytes32::new([0x55; 32]),
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
    );

    let signer = MockBlockSigner::new();
    let block = b
        .build_with_dfsp_activation(EMPTY_ROOT, EMPTY_ROOT, &signer, 0)
        .expect("V2 path with partial DFSP roots");

    assert_eq!(block.header.version, VERSION_V2);
    assert_eq!(
        block.header.collateral_registry_root,
        Bytes32::new([0x55; 32])
    );
    block.validate_structure().expect("structural OK");
}

/// **Test plan:** `test_build_missing_dfsp_roots`
#[test]
fn bld005_build_missing_dfsp_roots_when_v2_required() {
    // Builder has default EMPTY_ROOT for all five DFSP fields from `new`.
    let mut b = BlockBuilder::new(
        5,
        0,
        Bytes32::new([0x03; 32]),
        1,
        Bytes32::new([0x04; 32]),
        0,
    );
    b.add_spend_bundle(test_spend_bundle(), 1, 0)
        .expect("fixture");

    let signer = MockBlockSigner::new();
    let err = b
        .build_with_dfsp_activation(EMPTY_ROOT, EMPTY_ROOT, &signer, 0)
        .expect_err("V2 without DFSP data must fail");
    assert!(matches!(err, BuilderError::MissingDfspRoots));
}

/// **Test plan:** `test_build_two_pass_block_size` — explicit relationship between header field and `compute_size`.
#[test]
fn bld005_block_size_matches_serialized_len_after_second_pass() {
    let mut b = mk_builder_v1_height();
    b.add_spend_bundle(test_spend_bundle(), 0, 0)
        .expect("fixture");
    let signer = MockBlockSigner::new();
    let block = b.build(EMPTY_ROOT, EMPTY_ROOT, &signer).expect("build ok");

    let n = block.compute_size();
    assert_eq!(u64::from(block.header.block_size), n as u64);
}
