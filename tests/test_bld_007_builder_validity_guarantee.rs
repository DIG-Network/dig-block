//! BLD-007: **Structural validity by construction** — every [`dig_block::L2Block`] successfully returned from
//! [`dig_block::BlockBuilder::build`] or [`dig_block::BlockBuilder::build_with_dfsp_activation`] MUST satisfy
//! [`dig_block::L2Block::validate_structure`] ([SPEC §1.1 / §12.2](docs/resources/SPEC.md) design principle,
//! [NORMATIVE — BLD-007](docs/requirements/domains/block_production/NORMATIVE.md#bld-007-builder-structural-validity-guarantee)).
//!
//! **Authoritative spec:** `docs/requirements/domains/block_production/specs/BLD-007.md`. **Flat test path:**
//! `tests/test_bld_007_builder_validity_guarantee.rs` (STR-002 — not `tests/block_production/…`).
//!
//! ## Scope vs [`dig_block::BuilderError::EmptyBlock`]
//!
//! [`dig_block::BlockBuilder::build`](dig_block::BlockBuilder::build) rejects **zero** spend bundles ([BLD-005](docs/requirements/domains/block_production/specs/BLD-005.md) /
//! [ERR-004](docs/requirements/domains/error_types/specs/ERR-004.md)) — there is no `L2Block` value to validate in that case.
//! BLD-007 therefore applies to **`Ok(block)`** paths only. Empty **body** + coherent header still passes
//! [`dig_block::L2Block::validate_structure`] ([SVL-005](docs/requirements/domains/structural_validation/specs/SVL-005.md) /
//! [SVL-006](docs/requirements/domains/structural_validation/specs/SVL-006.md)); the companion test uses
//! [`common::sync_block_header_for_validate_structure`] like SVL fixtures to prove the tier-1 checker itself, separate
//! from the builder’s empty-body policy.
//!
//! ## How these tests prove BLD-007
//!
//! - Each test calls `build` / `build_with_dfsp_activation`, then **only** asserts
//!   [`dig_block::L2Block::validate_structure`] — no manual resync of Merkle fields — so any drift between builder
//!   algorithms and SVL-005/006 recomputation fails the suite (the spec’s parity table: spends/additions/removals/filter/
//!   slash roots, counts, serialized size cap).
//! - **Multiple bundles:** [`mk_distinct_spend_bundle`] varies parent / puzzle_hash bytes so [`dig_block::L2Block::has_duplicate_outputs`]
//!   and [`dig_block::L2Block::has_double_spends`] stay clear while still exercising multi-row Merkle aggregation.
//! - **V2 path:** [`dig_block::BlockBuilder::build_with_dfsp_activation`] with a finite activation height and non-empty
//!   DFSP roots proves the invariant holds when [`dig_block::VERSION_V2`] is selected ([BLK-007](docs/requirements/domains/block_types/specs/BLK-007.md)).
//!
//! **Tooling:** Repomix `.repomix/pack-src.xml` / `pack-tests.xml` refreshed; `npx gitnexus impact BlockBuilder` → **LOW**.
//! **SocratiCode:** MCP `codebase_search` / `codebase_status` from `docs/prompt/start.md` were not available here.

mod common;

use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};

use dig_block::{
    BlockBuilder, BuilderError, Bytes32, Cost, L2Block, L2BlockHeader, Signature, EMPTY_ROOT,
    VERSION_V2,
};

use common::{sync_block_header_for_validate_structure, test_spend_bundle, MockBlockSigner};

/// Minimal [`SpendBundle`] whose removal / addition bytes vary by `tag` so multiple bundles in one block do not trip
/// SVL-006 duplicate-output or double-spend probes ([`dig_block::L2Block::validate_structure`]).
fn mk_distinct_spend_bundle(tag: u8) -> SpendBundle {
    let parent = Bytes32::new([tag; 32]);
    let mut ph = [0x77u8; 32];
    ph[0] = tag;
    let puzzle_hash = Bytes32::new(ph);
    let coin = Coin::new(parent, puzzle_hash, 1_000_000u64);
    let puzzle_reveal = Program::from(vec![0x01]);
    let solution = Program::from(vec![0x80]);
    SpendBundle::new(
        vec![CoinSpend::new(coin, puzzle_reveal, solution)],
        Signature::default(),
    )
}

fn mk_builder_at_height(height: u64) -> BlockBuilder {
    BlockBuilder::new(
        height,
        0,
        Bytes32::new([0xbe; 32]),
        1,
        Bytes32::new([0xef; 32]),
        0,
    )
}

/// **Invariant:** successful `build` ⇒ `validate_structure` is `Ok`.
fn assert_ok_build_validates(b: BlockBuilder) {
    let signer = MockBlockSigner::new();
    let block = b
        .build(EMPTY_ROOT, EMPTY_ROOT, &signer)
        .expect("BLD-007 tests require successful build");
    block
        .validate_structure()
        .expect("BLD-007: builder output must pass SVL-005/006 structural tier");
}

/// **Test plan:** `test_empty_block_round_trip` — builder does not emit empty spends; empty **shape** still validates.
#[test]
fn bld007_empty_spend_list_returns_empty_block_error() {
    let b = mk_builder_at_height(1);
    let signer = MockBlockSigner::new();
    let err = b
        .build(EMPTY_ROOT, EMPTY_ROOT, &signer)
        .expect_err("no spends ⇒ no L2Block value");
    assert!(matches!(err, BuilderError::EmptyBlock));
}

/// **Test plan:** structural reference for zero spends (SVL-005 / SVL-006); not produced by `build()`.
#[test]
fn bld007_empty_body_with_synced_header_passes_validate_structure() {
    let h = L2BlockHeader::new(
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
    );
    let mut block = L2Block::new(h, vec![], vec![], Signature::default());
    sync_block_header_for_validate_structure(&mut block);
    block
        .validate_structure()
        .expect("empty body is structurally valid when header matches body (SVL fixtures pattern)");
}

/// **Test plan:** `test_single_bundle_round_trip`
#[test]
fn bld007_single_spend_bundle_passes_validate_structure() {
    let mut b = mk_builder_at_height(1);
    b.add_spend_bundle(test_spend_bundle(), 1, 0)
        .expect("fixture within budgets");
    assert_ok_build_validates(b);
}

/// **Test plan:** `test_multi_bundle_round_trip`
#[test]
fn bld007_multiple_spend_bundles_passes_validate_structure() {
    let mut b = mk_builder_at_height(1);
    for tag in [1u8, 2, 3] {
        b.add_spend_bundle(mk_distinct_spend_bundle(tag), 1, 0)
            .expect("distinct bundles stay under caps");
    }
    assert_ok_build_validates(b);
}

/// **Test plan:** `test_with_slash_proposals_round_trip`
#[test]
fn bld007_with_slash_proposals_passes_validate_structure() {
    let mut b = mk_builder_at_height(1);
    b.add_spend_bundle(test_spend_bundle(), 2, 0)
        .expect("fixture");
    b.add_slash_proposal(vec![0x01, 0x02])
        .expect("slash within caps");
    b.add_slash_proposal(vec![0xaa]).expect("second slash");
    assert_ok_build_validates(b);
}

/// **Test plan:** `test_full_featured_block_round_trip` — BLD-004 optional header fields copied into [`L2BlockHeader`].
#[test]
fn bld007_full_optional_fields_passes_validate_structure() {
    let mut b = mk_builder_at_height(2);
    b.set_l1_proofs(
        Bytes32::new([0x61; 32]),
        Bytes32::new([0x62; 32]),
        Bytes32::new([0x63; 32]),
        Bytes32::new([0x64; 32]),
        Bytes32::new([0x65; 32]),
    );
    b.set_dfsp_roots(
        Bytes32::new([0x71; 32]),
        Bytes32::new([0x72; 32]),
        Bytes32::new([0x73; 32]),
        Bytes32::new([0x74; 32]),
        Bytes32::new([0x75; 32]),
    );
    b.set_extension_data(Bytes32::new([0x80; 32]));
    b.add_spend_bundle(mk_distinct_spend_bundle(0x10), 3, 1)
        .expect("bundle");
    b.add_slash_proposal(vec![0xde, 0xad, 0xbe, 0xef])
        .expect("slash");
    assert_ok_build_validates(b);
}

/// **Test plan:** `test_v1_block_round_trip` — crate [`dig_block::DFSP_ACTIVATION_HEIGHT`] sentinel keeps V1 at typical heights.
#[test]
fn bld007_v1_default_activation_passes_validate_structure() {
    let mut b = mk_builder_at_height(1);
    b.add_spend_bundle(test_spend_bundle(), 0, 0)
        .expect("fixture");
    let signer = MockBlockSigner::new();
    let block = b.build(EMPTY_ROOT, EMPTY_ROOT, &signer).expect("build");
    assert_eq!(block.header.version, dig_block::VERSION_V1);
    block
        .validate_structure()
        .expect("V1 builder block structural OK");
}

/// **Test plan:** `test_v2_block_round_trip`
#[test]
fn bld007_v2_post_activation_passes_validate_structure() {
    let mut b = mk_builder_at_height(20);
    b.set_dfsp_roots(
        Bytes32::new([0x01; 32]),
        Bytes32::new([0x02; 32]),
        Bytes32::new([0x03; 32]),
        Bytes32::new([0x04; 32]),
        Bytes32::new([0x05; 32]),
    );
    b.add_spend_bundle(mk_distinct_spend_bundle(0x20), 1, 0)
        .expect("bundle");
    let signer = MockBlockSigner::new();
    let block = b
        .build_with_dfsp_activation(EMPTY_ROOT, EMPTY_ROOT, &signer, 0)
        .expect("V2 build");
    assert_eq!(block.header.version, VERSION_V2);
    block.validate_structure().expect(
        "V2 builder block must still satisfy SVL-005/006 (no header.validate in this tier)",
    );
}
