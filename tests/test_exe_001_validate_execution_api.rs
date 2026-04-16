//! EXE-001: `L2Block::validate_execution` API surface ([SPEC §7.4](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/execution_validation/NORMATIVE.md` (EXE-001)
//! **Spec:** `docs/requirements/domains/execution_validation/specs/EXE-001.md`
//!
//! ## What this proves
//!
//! - **Method signature:** `L2Block::validate_execution(&self, &ValidationConfig, &Bytes32) ->
//!   Result<ExecutionResult, BlockError>` exists and type-checks. Compile-time proof of the API
//!   contract required by NORMATIVE EXE-001.
//!
//! - **ValidationConfig from dig-clvm:** The config parameter is the `dig_clvm::ValidationConfig`
//!   re-export, not a dig-block custom type. NORMATIVE is explicit: "`ValidationConfig` is sourced
//!   from `dig-clvm`" (EXE-001 / EXE-003 — see `docs/prompt/start.md` Hard Requirement 2).
//!
//! - **Empty block succeeds:** A block with zero `SpendBundle`s and zero totals returns
//!   `Ok(ExecutionResult)` with every collection empty and both scalars `0` (the EXE-001 test plan
//!   `empty_block` case). This matches the EXE-008 "empty_block_result" invariant.
//!
//! - **Fee consistency:** EXE-006 — when the header declares `total_fees != 0` on an empty block,
//!   `validate_execution` rejects with `BlockError::FeesMismatch { header, computed }` because
//!   computed fees on zero bundles is 0.
//!
//! - **Cost consistency:** EXE-007 — similarly, non-zero `header.total_cost` on an empty block
//!   rejects with `BlockError::CostMismatch { header, computed }`.
//!
//! - **Block-order processing:** NORMATIVE requires bundles be "process[ed] ... in block order".
//!   The `genesis_challenge` is accepted without panic; bundles-in-order semantics are exercised
//!   in EXE-003/EXE-004 when CLVM execution lands (this requirement only pins the API shape).
//!
//! ## Why non-empty bodies are not exercised here
//!
//! `dig_clvm::validate_spend_bundle` requires a `ValidationContext` with per-coin
//! `chia_sdk_coinset::CoinRecord` entries (coin existence check precedes CLVM). That state comes
//! from Tier 3 (STV-002 / `CoinLookup`). EXE-001 NORMATIVE only specifies the API and empty-block
//! behavior; full CLVM coverage is EXE-003 and will take a `CoinLookup` handle. The test plan's
//! `invalid_spend_bundle` and `multiple_spend_bundles` cases belong to EXE-003 (tracked there).
//!
//! ## How this satisfies EXE-001
//!
//! One test per bullet in the EXE-001 test plan that the API-shape-only implementation can honor:
//! `valid_block_execution` (empty), `empty_block`, and negative cases for the block-level
//! fee/cost consistency checks (EXE-006 / EXE-007) which `validate_execution` performs directly.

use chia_protocol::Bytes32;
use dig_clvm::ValidationConfig;

use dig_block::{BlockError, L2Block, L2BlockHeader, Signature};

/// Construct an empty L2Block anchored to a plausible genesis header.
fn empty_block() -> L2Block {
    let network_id = Bytes32::new([0xaa; 32]);
    let l1_hash = Bytes32::new([0xbb; 32]);
    let header = L2BlockHeader::genesis(network_id, 1, l1_hash);
    L2Block::new(header, Vec::new(), Vec::new(), Signature::default())
}

/// Like [`empty_block`] but overrides `header.total_fees` — used to force EXE-006 rejection.
fn empty_block_with_fees(fees: u64) -> L2Block {
    let mut block = empty_block();
    block.header.total_fees = fees;
    block
}

/// Like [`empty_block`] but overrides `header.total_cost` — used to force EXE-007 rejection.
fn empty_block_with_cost(cost: u64) -> L2Block {
    let mut block = empty_block();
    block.header.total_cost = cost;
    block
}

/// **EXE-001 test plan: `empty_block`** — zero-body block returns a populated-but-empty
/// `ExecutionResult`. `genesis_challenge` is irrelevant here (no bundles to verify against), but
/// is still passed per NORMATIVE signature.
#[test]
fn empty_block_returns_empty_execution_result() {
    let block = empty_block();
    let config = ValidationConfig::default();
    let genesis_challenge = Bytes32::new([0x42; 32]);

    let result = block
        .validate_execution(&config, &genesis_challenge)
        .expect("empty block validates");

    assert!(result.additions.is_empty());
    assert!(result.removals.is_empty());
    assert!(result.pending_assertions.is_empty());
    assert_eq!(result.total_cost, 0);
    assert_eq!(result.total_fees, 0);
    assert!(result.receipts.is_empty());
}

/// **EXE-001 test plan: `valid_block_execution`** — the signature
/// `(&L2Block, &ValidationConfig, &Bytes32) -> Result<ExecutionResult, BlockError>` matches
/// NORMATIVE exactly. Compile-time check via explicit function pointer coercion.
#[test]
fn signature_matches_normative() {
    // If the free-function pointer coerces, the `&self` method has the required shape.
    let _fn_ptr: fn(
        &L2Block,
        &ValidationConfig,
        &Bytes32,
    ) -> Result<dig_block::ExecutionResult, BlockError> = L2Block::validate_execution;
}

/// **EXE-001 + EXE-006:** Mismatched `header.total_fees` causes `FeesMismatch` on an otherwise
/// empty block where the computed total is 0.
#[test]
fn fee_mismatch_rejected() {
    let block = empty_block_with_fees(100);
    let config = ValidationConfig::default();
    let err = block
        .validate_execution(&config, &Bytes32::default())
        .expect_err("non-zero header fees on empty body must reject");
    match err {
        BlockError::FeesMismatch { header, computed } => {
            assert_eq!(header, 100);
            assert_eq!(computed, 0);
        }
        other => panic!("expected FeesMismatch, got {:?}", other),
    }
}

/// **EXE-001 + EXE-007:** Mismatched `header.total_cost` causes `CostMismatch` on an otherwise
/// empty block. EXE-007 asserts `sum(SpendResult.conditions.cost) == header.total_cost`; on an
/// empty body the sum is 0.
#[test]
fn cost_mismatch_rejected() {
    let block = empty_block_with_cost(999);
    let config = ValidationConfig::default();
    let err = block
        .validate_execution(&config, &Bytes32::default())
        .expect_err("non-zero header cost on empty body must reject");
    match err {
        BlockError::CostMismatch { header, computed } => {
            assert_eq!(header, 999);
            assert_eq!(computed, 0);
        }
        other => panic!("expected CostMismatch, got {:?}", other),
    }
}

/// **EXE-001 acceptance:** Validation is deterministic — calling twice on the same block yields
/// the same outcome.
#[test]
fn validation_is_deterministic() {
    let block = empty_block();
    let config = ValidationConfig::default();
    let genesis = Bytes32::new([0x01; 32]);

    let r1 = block.validate_execution(&config, &genesis).unwrap();
    let r2 = block.validate_execution(&config, &genesis).unwrap();
    assert_eq!(r1, r2);
}

/// **EXE-001 acceptance:** Accepting [`ValidationConfig::default`] — proves the public API
/// consumes the dig-clvm re-export without custom wrappers.
#[test]
fn accepts_dig_clvm_validation_config() {
    let block = empty_block();
    // `ValidationConfig` is pub re-exported from dig-clvm; its Default uses L2_MAX_COST_PER_BLOCK.
    let config = ValidationConfig::default();
    assert_eq!(config.max_cost_per_block, dig_clvm::L2_MAX_COST_PER_BLOCK);

    let _ = block.validate_execution(&config, &Bytes32::default()).unwrap();
}
