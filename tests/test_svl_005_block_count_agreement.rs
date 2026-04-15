//! SVL-005: [`L2Block::validate_structure`] must ensure the four header counters match what the block body actually
//! contains ([SPEC §5.2 steps 2, 4, 5, 13](docs/resources/SPEC.md),
//! [NORMATIVE — SVL-005](docs/requirements/domains/structural_validation/NORMATIVE.md#svl-005-block-count-agreement)).
//!
//! **Spec + test plan:** `docs/requirements/domains/structural_validation/specs/SVL-005.md`  
//! **Implementation:** [`dig_block::L2Block::validate_structure`] — `src/types/block.rs` (count phase; SVL-006 extends with Merkle/integrity).  
//! **Errors:** [`dig_block::BlockError::SpendBundleCountMismatch`], [`AdditionsCountMismatch`], [`RemovalsCountMismatch`],
//! [`SlashProposalCountMismatch`] ([ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md)) — each carries
//! `header` (declared) and `actual` (computed) `u32` values.
//!
//! ## How these tests prove SVL-005
//!
//! - **`svl005_zero_counts_match_empty_block`:** Empty vectors with all header counts at **0** ⇒ `Ok` (spec `test_zero_counts_match_empty_block`).
//! - **`svl005_all_counts_match_one_bundle`:** One real [`SpendBundle`] from the Chia-style hex fixture (same pattern as
//!   BLK-004 tests); header counts are set to the **true** derived values so all four gates pass together.
//! - **Mismatch tests:** clone that consistent header, poison **one** count at a time, expect the **distinct** error
//!   variant with matching `header` / `actual` fields — proving each counter is enforced independently and diagnostics
//!   stay structured.
//!
//! **Flat test path:** `tests/test_svl_005_block_count_agreement.rs` per [STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md)
//! (not `tests/structural_validation/…` from the spec prose).
//!
//! **Tooling:** Repomix packs under `.repomix/` were regenerated before edits. GitNexus CLI was not usable in this
//! session (`npx gitnexus` npm failure); blast radius was checked with repository search — only [`validate_structure`]
//! is new on [`L2Block`], and three [`BlockError`] variants gained fields (callers updated in-repo).

use chia_bls::G2Element;
use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use dig_block::{BlockError, Bytes32, Cost, L2Block, L2BlockHeader, Signature, EMPTY_ROOT};

/// Inert header base: counts and roots are adjusted per test; height/version follow [`L2BlockHeader::new`] rules.
fn base_header(
    spend_bundle_count: u32,
    additions_count: u32,
    removals_count: u32,
    slash_proposal_count: u32,
) -> L2BlockHeader {
    let mut h = L2BlockHeader::new(
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
        spend_bundle_count,
        0 as Cost,
        0,
        additions_count,
        removals_count,
        0,
        EMPTY_ROOT,
    );
    h.slash_proposal_count = slash_proposal_count;
    h
}

/// Chia-protocol single-`CREATE_COIN` fixture (see `tests/test_l2_block_helpers.rs` — BLK-004 provenance).
fn spend_single_create_hex_coin() -> SpendBundle {
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
    SpendBundle::new(vec![spend], G2Element::default())
}

/// **Test plan:** `test_zero_counts_match_empty_block`
#[test]
fn svl005_zero_counts_match_empty_block() {
    let h = base_header(0, 0, 0, 0);
    let b = L2Block::new(h, vec![], vec![], Signature::default());
    b.validate_structure()
        .expect("empty body with zero header counts must pass SVL-005");
}

/// Block with one fixture bundle and header counts matching [`L2Block::all_additions`] / coin-spend / slash lengths.
fn consistent_one_bundle_block() -> L2Block {
    let sb = spend_single_create_hex_coin();
    let mut b = L2Block::new(
        base_header(0, 0, 0, 0),
        vec![sb],
        vec![],
        Signature::default(),
    );
    let additions = b.all_additions().len() as u32;
    let removals: u32 = b
        .spend_bundles
        .iter()
        .map(|x| x.coin_spends.len() as u32)
        .sum();
    b.header.spend_bundle_count = 1;
    b.header.additions_count = additions;
    b.header.removals_count = removals;
    b.header.slash_proposal_count = 0;
    b
}

/// **Test plan:** `test_all_counts_match`
#[test]
fn svl005_all_counts_match() {
    let b = consistent_one_bundle_block();
    b.validate_structure()
        .expect("aligned counts for one-bundle block must pass");
}

/// **Test plan:** `test_spend_bundle_count_mismatch`
#[test]
fn svl005_spend_bundle_count_mismatch() {
    let mut b = consistent_one_bundle_block();
    b.header.spend_bundle_count = 2;
    match b
        .validate_structure()
        .expect_err("wrong spend_bundle_count must fail")
    {
        BlockError::SpendBundleCountMismatch { header, actual } => {
            assert_eq!(header, 2);
            assert_eq!(actual, 1);
        }
        e => panic!("expected SpendBundleCountMismatch, got {e:?}"),
    }
}

/// **Test plan:** `test_additions_count_mismatch`
#[test]
fn svl005_additions_count_mismatch() {
    let mut b = consistent_one_bundle_block();
    let true_add = b.header.additions_count;
    b.header.additions_count = true_add.saturating_add(1);
    match b
        .validate_structure()
        .expect_err("wrong additions_count must fail")
    {
        BlockError::AdditionsCountMismatch { header, actual } => {
            assert_eq!(actual, true_add);
            assert_eq!(header, true_add.saturating_add(1));
        }
        e => panic!("expected AdditionsCountMismatch, got {e:?}"),
    }
}

/// **Test plan:** `test_removals_count_mismatch`
#[test]
fn svl005_removals_count_mismatch() {
    let mut b = consistent_one_bundle_block();
    b.header.removals_count = 0;
    match b
        .validate_structure()
        .expect_err("wrong removals_count must fail")
    {
        BlockError::RemovalsCountMismatch { header, actual } => {
            assert_eq!(header, 0);
            assert_eq!(actual, 1);
        }
        e => panic!("expected RemovalsCountMismatch, got {e:?}"),
    }
}

/// **Test plan:** `test_slash_proposal_count_mismatch`
#[test]
fn svl005_slash_proposal_count_mismatch() {
    let mut b = consistent_one_bundle_block();
    b.slash_proposal_payloads.push(vec![0x01, 0x02]);
    b.header.slash_proposal_count = 0;
    match b
        .validate_structure()
        .expect_err("slash_proposal_count vs payloads must fail")
    {
        BlockError::SlashProposalCountMismatch { header, actual } => {
            assert_eq!(header, 0);
            assert_eq!(actual, 1);
        }
        e => panic!("expected SlashProposalCountMismatch, got {e:?}"),
    }
}
