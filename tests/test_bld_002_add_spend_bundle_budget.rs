//! BLD-002: [`dig_block::BlockBuilder::add_spend_bundle`] — CLVM **cost** and serialized **size** budgets before any
//! mutation, plus `remaining_cost` / `spend_bundle_count` ([SPEC §6](docs/resources/SPEC.md),
//! [NORMATIVE — BLD-002](docs/requirements/domains/block_production/NORMATIVE.md#bld-002-add_spend_bundle-with-budget-enforcement)).
//!
//! **Authoritative spec:** `docs/requirements/domains/block_production/specs/BLD-002.md` (pseudocode, acceptance
//! criteria, test-plan table). **Flat test path:** `tests/test_bld_002_add_spend_bundle_budget.rs` (STR-002 / project
//! rules — not `tests/block_production/…` from the spec’s example path).
//!
//! ## How these tests prove BLD-002
//!
//! - **Cost budget:** Exercising [`dig_block::BuilderError::CostBudgetExceeded`] with `current` / `addition` / `max`
//!   proves the strict inequality `total_cost + cost > MAX_COST_PER_BLOCK` is rejected **before** body mutation.
//! - **Size budget:** [`dig_block::BuilderError::SizeBudgetExceeded`] after a padded [`SpendBundle`] proves the builder
//!   uses the same **full `bincode(L2Block)`** shape as SVL-006 / [`dig_block::L2Block::compute_size`], not bundle-only
//!   byte counts.
//! - **No partial state:** Snapshot lengths and totals around a failing `add_spend_bundle` prove the all-or-nothing
//!   contract from the spec’s “before any mutation” rule.
//! - **Success path:** Additions from [`SpendBundle::additions`], removals from [`Coin::coin_id`], running totals, and
//!   append order match [`dig_block::L2Block::all_additions`] / [`dig_block::L2Block::all_removals`] semantics.
//! - **Helpers:** `remaining_cost` / `spend_bundle_count` mirror `MAX_COST_PER_BLOCK - total_cost` and `Vec::len`.
//!
//! **Tooling:** Repomix packs under `.repomix/` refreshed for `src/`, `tests/`, and `docs/requirements/domains/block_production`.
//! `npx gitnexus impact BlockBuilder` → **LOW** risk (no upstream callers). SocratiCode MCP was not configured here.

mod common;

use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use common::test_spend_bundle;
use dig_block::{BlockBuilder, BuilderError, Bytes32, Cost, MAX_BLOCK_SIZE, MAX_COST_PER_BLOCK};

fn mk_builder() -> BlockBuilder {
    BlockBuilder::new(
        1,
        0,
        Bytes32::new([0xab; 32]),
        1,
        Bytes32::new([0xcd; 32]),
        0,
    )
}

/// Minimal bundle whose `solution` carries `pad` raw CLVM-serialization bytes (`0x80` = empty atom).
///
/// **Rationale:** Growing `pad` grows `bincode(SpendBundle)` roughly linearly so we can cross [`MAX_BLOCK_SIZE`]
/// without tens of thousands of distinct fixtures (see BLD-002 test plan `test_add_bundle_exceeds_size_budget`).
fn padded_spend_bundle(pad: usize) -> SpendBundle {
    let coin = Coin::new(Bytes32::new([0x11; 32]), Bytes32::new([0x22; 32]), 1u64);
    let puzzle_reveal = Program::from(vec![0x01u8]);
    let solution = Program::from(std::iter::repeat(0x80u8).take(pad).collect::<Vec<_>>());
    SpendBundle::new(
        vec![CoinSpend::new(coin, puzzle_reveal, solution)],
        Default::default(),
    )
}

/// **Test plan:** `test_add_bundle_within_cost_budget`
#[test]
fn bld002_add_bundle_within_cost_budget() {
    let mut b = mk_builder();
    let cost = MAX_COST_PER_BLOCK - 1;
    b.add_spend_bundle(test_spend_bundle(), cost, 5)
        .expect("within cost budget");
    assert_eq!(b.total_cost, cost);
    assert_eq!(b.total_fees, 5u64);
    assert_eq!(b.spend_bundle_count(), 1);
}

/// **Test plan:** `test_add_bundle_exceeds_cost_budget`
#[test]
fn bld002_add_bundle_exceeds_cost_budget() {
    let mut b = mk_builder();
    let err = b
        .add_spend_bundle(test_spend_bundle(), MAX_COST_PER_BLOCK + 1, 0)
        .expect_err("must exceed cost budget");
    match err {
        BuilderError::CostBudgetExceeded {
            current,
            addition,
            max,
        } => {
            assert_eq!(current, 0);
            assert_eq!(addition, MAX_COST_PER_BLOCK + 1);
            assert_eq!(max, MAX_COST_PER_BLOCK);
        }
        e => panic!("unexpected error: {e:?}"),
    }
    assert!(b.spend_bundles.is_empty());
    assert_eq!(b.total_cost, 0);
}

/// **Test plan:** `test_add_bundle_exceeds_size_budget`
#[test]
fn bld002_add_bundle_exceeds_size_budget() {
    let mut b = mk_builder();
    let err = b
        .add_spend_bundle(padded_spend_bundle(MAX_BLOCK_SIZE as usize), 0, 0)
        .expect_err("padding at MAX_BLOCK_SIZE must push bincode(L2Block) past the cap");
    match err {
        BuilderError::SizeBudgetExceeded {
            current,
            addition,
            max,
        } => {
            assert_eq!(max, MAX_BLOCK_SIZE);
            assert!(addition > 0);
            assert!(current <= max);
        }
        e => panic!("unexpected error: {e:?}"),
    }
    assert!(b.spend_bundles.is_empty());
}

/// **Test plan:** `test_rejected_bundle_no_state_change`
#[test]
fn bld002_rejected_bundle_no_state_change() {
    let mut b = mk_builder();
    b.add_spend_bundle(test_spend_bundle(), 10, 3).unwrap();
    let snap_cost = b.total_cost;
    let snap_fees = b.total_fees;
    let snap_add = b.additions.len();
    let snap_rem = b.removals.len();
    let snap_sb = b.spend_bundles.len();

    let _ = b.add_spend_bundle(test_spend_bundle(), MAX_COST_PER_BLOCK, 0);

    assert_eq!(b.total_cost, snap_cost);
    assert_eq!(b.total_fees, snap_fees);
    assert_eq!(b.additions.len(), snap_add);
    assert_eq!(b.removals.len(), snap_rem);
    assert_eq!(b.spend_bundles.len(), snap_sb);
}

/// **Test plan:** `test_additions_extracted` / `test_removals_extracted` / `test_running_totals_updated`
#[test]
fn bld002_additions_removals_and_totals() {
    let mut b = mk_builder();
    let bundle = test_spend_bundle();
    // Same fallback as production [`dig_block::L2Block::all_additions`] when CLVM simulation errors.
    let expected_additions: Vec<Coin> = bundle.additions().unwrap_or_default();
    let expected_removals: Vec<Bytes32> = bundle
        .coin_spends
        .iter()
        .map(|cs| cs.coin.coin_id())
        .collect();
    b.add_spend_bundle(bundle, 42, 9).unwrap();
    assert_eq!(b.additions, expected_additions);
    assert_eq!(b.removals, expected_removals);
    assert_eq!(b.total_cost, 42 as Cost);
    assert_eq!(b.total_fees, 9u64);
}

/// **Test plan:** `test_remaining_cost`
#[test]
fn bld002_remaining_cost() {
    let mut b = mk_builder();
    assert_eq!(b.remaining_cost(), MAX_COST_PER_BLOCK);
    b.add_spend_bundle(test_spend_bundle(), 1_000, 0).unwrap();
    assert_eq!(b.remaining_cost(), MAX_COST_PER_BLOCK - 1_000);
}

/// **Test plan:** `test_multiple_bundles_accumulate`
#[test]
fn bld002_multiple_bundles_accumulate() {
    let mut b = mk_builder();
    b.add_spend_bundle(test_spend_bundle(), 100, 1).unwrap();
    b.add_spend_bundle(test_spend_bundle(), 50, 2).unwrap();
    assert_eq!(b.total_cost, 150);
    assert_eq!(b.total_fees, 3);
    assert_eq!(b.spend_bundle_count(), 2);
}

/// **Test plan:** `test_spend_bundle_count`
#[test]
fn bld002_spend_bundle_count_tracks_len() {
    let mut b = mk_builder();
    assert_eq!(b.spend_bundle_count(), 0);
    b.add_spend_bundle(test_spend_bundle(), 0, 0).unwrap();
    assert_eq!(b.spend_bundle_count(), b.spend_bundles.len());
}
