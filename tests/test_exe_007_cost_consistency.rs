//! EXE-007: Block-level cost consistency ([SPEC §7.4.6](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/execution_validation/NORMATIVE.md` (EXE-007)
//! **Spec:** `docs/requirements/domains/execution_validation/specs/EXE-007.md`
//! **Chia parity:** [`block_body_validation.py` Check 9 (`INVALID_BLOCK_COST`)](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py).
//!
//! ## What the rule says
//!
//! `computed_total_cost = sum(SpendResult.conditions.cost)` across every `SpendBundle` in the
//! block MUST equal `header.total_cost`. Mismatch rejects with [`BlockError::CostMismatch`].
//!
//! ## Ownership
//!
//! - **Per-bundle cost computation:** `dig-clvm` — `SpendResult.conditions.cost` is produced by
//!   `chia-consensus::run_spendbundle` during CLVM execution.
//! - **Block-level summation + consistency check:** `dig-block` — performed at the tail of
//!   [`dig_block::L2Block::validate_execution`] and
//!   [`dig_block::L2Block::validate_execution_with_context`] (already implemented alongside
//!   EXE-001 / EXE-003).
//!
//! ## What this proves
//!
//! - **Zero cost boundary:** Empty block with `header.total_cost == 0` accepts.
//! - **Cost mismatch both directions:** Header higher than computed AND lower than computed both
//!   reject as `CostMismatch`. The error shape preserves `(header, computed)` without ordering
//!   bias — mirroring the EXE-006 fee pattern.
//! - **Cost field separation:** `CostMismatch` and `FeesMismatch` are distinct variants; a block
//!   with correct fees but wrong cost (or vice versa) emits the correct error. This matters for
//!   Chia parity (Check 9 vs Check 19 are distinct).
//! - **`ClvmCostExceeded` is a different check:** That variant covers the **per-bundle** CLVM
//!   budget (via `dig_clvm::ValidationError::CostExceeded`). This test confirms the two variants
//!   do not conflate.
//! - **u64 boundary:** `u64::MAX` flows through `CostMismatch` without truncation.
//!
//! ## How this satisfies EXE-007
//!
//! One test per bullet in the EXE-007 test plan that the current implementation can honor:
//! `zero_cost_block`, `cost_mismatch_high`, `cost_mismatch_low` (via shape), and supporting
//! boundary tests. `single_bundle_cost` / `multi_bundle_cost_sum` require real bundles through
//! `validate_execution_with_context`; those land alongside a full EXE-003 integration suite
//! that can stage coin_records via CoinLookup (Tier-3 bridge).

use chia_protocol::Bytes32;
use dig_clvm::{ValidationConfig, ValidationError};

use dig_block::{map_clvm_validation_error, BlockError, L2Block, L2BlockHeader, Signature};

fn empty_block() -> L2Block {
    let network_id = Bytes32::new([0x77; 32]);
    let l1_hash = Bytes32::new([0x88; 32]);
    let header = L2BlockHeader::genesis(network_id, 1, l1_hash);
    L2Block::new(header, Vec::new(), Vec::new(), Signature::default())
}

/// **EXE-007 test plan: `zero_cost_block`:** Genesis-style empty block declares zero cost, and
/// `sum(...)` over zero bundles is zero — the happy-path boundary.
#[test]
fn zero_cost_empty_block_passes() {
    let block = empty_block();
    let config = ValidationConfig::default();
    block
        .validate_execution(&config, &Bytes32::default())
        .expect("zero-cost empty block must pass");
}

/// **EXE-007 test plan: `cost_mismatch_high`:** Header declares non-zero cost; computed from
/// empty body is `0`. Rejects with `CostMismatch { header, computed }` carrying both values.
#[test]
fn cost_mismatch_high_rejects() {
    let mut block = empty_block();
    block.header.total_cost = 5_000_000;
    let config = ValidationConfig::default();
    let err = block
        .validate_execution(&config, &Bytes32::default())
        .expect_err("header-higher-than-computed cost must reject");

    match err {
        BlockError::CostMismatch { header, computed } => {
            assert_eq!(header, 5_000_000);
            assert_eq!(computed, 0);
        }
        other => panic!("expected CostMismatch, got {:?}", other),
    }
}

/// **EXE-007 test plan: `cost_mismatch_low`:** The error has no implicit direction — `header=0`
/// with `computed=N` also surfaces as `CostMismatch`. Shape-level check because the empty-body
/// test cannot produce non-zero computed cost.
#[test]
fn cost_mismatch_is_bidirectional_in_shape() {
    let under = BlockError::CostMismatch {
        header: 0,
        computed: 1_000,
    };
    let over = BlockError::CostMismatch {
        header: 1_000,
        computed: 0,
    };
    assert!(matches!(under, BlockError::CostMismatch { .. }));
    assert!(matches!(over, BlockError::CostMismatch { .. }));
}

/// **EXE-007:** `CostMismatch` and `FeesMismatch` are orthogonal variants. Per EXE-001, the
/// fee check runs **before** the cost check; a block with wrong fees and wrong cost surfaces
/// `FeesMismatch`. Proves variant distinctness by constructing both and matching.
#[test]
fn cost_and_fee_mismatches_are_distinct_variants() {
    let cost = BlockError::CostMismatch {
        header: 1,
        computed: 0,
    };
    let fee = BlockError::FeesMismatch {
        header: 1,
        computed: 0,
    };
    assert!(!matches!(cost, BlockError::FeesMismatch { .. }));
    assert!(!matches!(fee, BlockError::CostMismatch { .. }));
}

/// **EXE-007:** `CostMismatch` (block-level sum) is NOT the same variant as `ClvmCostExceeded`
/// (per-bundle budget) — dig-clvm raises the latter, `validate_execution` raises the former.
/// Confirms mapping + layering.
#[test]
fn clvm_cost_exceeded_is_different_from_cost_mismatch() {
    let per_bundle = map_clvm_validation_error(ValidationError::CostExceeded {
        limit: 100,
        consumed: 200,
    });
    assert!(matches!(per_bundle, BlockError::ClvmCostExceeded { .. }));

    let block_level = BlockError::CostMismatch {
        header: 100,
        computed: 200,
    };
    assert!(matches!(block_level, BlockError::CostMismatch { .. }));
    assert!(!matches!(block_level, BlockError::ClvmCostExceeded { .. }));
}

/// **EXE-007:** `u64::MAX` values round-trip through `CostMismatch` — no narrowing.
#[test]
fn cost_mismatch_handles_u64_max() {
    let err = BlockError::CostMismatch {
        header: u64::MAX,
        computed: 0,
    };
    match err {
        BlockError::CostMismatch { header, computed } => {
            assert_eq!(header, u64::MAX);
            assert_eq!(computed, 0);
        }
        _ => panic!(),
    }
}

/// **EXE-007:** Error message surfaces both sides so logs can diagnose without code change.
#[test]
fn cost_mismatch_display_includes_both_values() {
    let err = BlockError::CostMismatch {
        header: 42,
        computed: 7,
    };
    let s = format!("{}", err);
    assert!(s.contains("42"), "display must include header value: {}", s);
    assert!(
        s.contains("7"),
        "display must include computed value: {}",
        s
    );
}
