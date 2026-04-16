//! STV-001: `L2Block::validate_state` + `validate_full` API surface ([SPEC §7.5](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/state_validation/NORMATIVE.md` (STV-001)
//! **Spec:** `docs/requirements/domains/state_validation/specs/STV-001.md`
//!
//! ## What this proves
//!
//! - **`validate_state` signature:** `L2Block::validate_state(&self, &ExecutionResult, &dyn
//!   CoinLookup, &PublicKey) -> Result<Bytes32, BlockError>`. Compile-time function-pointer
//!   coercion check.
//! - **`validate_full` signature:** `L2Block::validate_full(&self, &ValidationConfig, &Bytes32,
//!   &dyn CoinLookup, &PublicKey) -> Result<Bytes32, BlockError>`.
//! - **Tier ordering:** `validate_full` runs Tier 1 → Tier 2 → Tier 3; a Tier 1 failure
//!   short-circuits (Tier 2/3 never run); Tier 2 failure short-circuits Tier 3.
//! - **Empty-block happy path:** Empty block + empty ExecutionResult + empty CoinLookup + default
//!   PublicKey → `validate_state` returns the block's declared `state_root` (Tier 3 computed
//!   root for a zero-delta block equals the parent's state root, which for genesis is `EMPTY_ROOT`).
//! - **Returns Bytes32 (state root):** On success, `validate_state` returns the computed state
//!   root, not just `()`. Tier-3 callers use this value as the parent-state commitment for the
//!   next block.
//!
//! ## Scope of this requirement
//!
//! STV-001 is the **API surface** — sub-checks STV-002 through STV-007 each have dedicated
//! requirements and will harden the behavior. For this commit, `validate_state` is a
//! functioning dispatcher with stub bodies (no-op on empty inputs); sub-checks will tighten
//! assertions in follow-on commits without changing the outer signature.

mod common;

use chia_protocol::Bytes32;
use dig_clvm::ValidationConfig;

use dig_block::{
    BlockError, CoinLookup, ExecutionResult, L2Block, L2BlockHeader, PublicKey, Signature,
};

/// Empty [`CoinLookup`] — no coins, fixed chain context.
struct EmptyCoins;
impl CoinLookup for EmptyCoins {
    fn get_coin_state(&self, _coin_id: &Bytes32) -> Option<chia_protocol::CoinState> {
        None
    }
    fn get_chain_height(&self) -> u64 {
        0
    }
    fn get_chain_timestamp(&self) -> u64 {
        0
    }
}

fn empty_block() -> L2Block {
    let network_id = Bytes32::new([0x55; 32]);
    let l1_hash = Bytes32::new([0x66; 32]);
    let header = L2BlockHeader::genesis(network_id, 1, l1_hash);
    let mut block = L2Block::new(header, Vec::new(), Vec::new(), Signature::default());
    // Align header fields with the empty body so Tier-1 structural validation passes.
    // Without this, `genesis()` leaves `filter_hash = EMPTY_ROOT` but the BIP-158 filter over
    // an empty body has a distinct hash; validate_structure would reject with InvalidFilterHash.
    common::sync_block_header_for_validate_structure(&mut block);
    block
}

/// **STV-001 acceptance:** Method signature matches NORMATIVE.
#[test]
fn validate_state_signature_matches_normative() {
    let _fn_ptr: fn(
        &L2Block,
        &ExecutionResult,
        &dyn CoinLookup,
        &PublicKey,
    ) -> Result<Bytes32, BlockError> = L2Block::validate_state;
}

/// **STV-001 acceptance:** `validate_full` signature matches NORMATIVE.
#[test]
fn validate_full_signature_matches_normative() {
    let _fn_ptr: fn(
        &L2Block,
        &ValidationConfig,
        &Bytes32,
        &dyn CoinLookup,
        &PublicKey,
    ) -> Result<Bytes32, BlockError> = L2Block::validate_full;
}

/// **STV-001 test plan: `valid_state_validation`:** Empty ExecutionResult, empty CoinLookup,
/// default proposer PublicKey on an empty block passes all sub-checks and returns the block's
/// committed state root.
#[test]
fn empty_block_validate_state_returns_committed_state_root() {
    let block = empty_block();
    let exec = ExecutionResult::default();
    let coins = EmptyCoins;
    let pk = PublicKey::default();

    let returned = block
        .validate_state(&exec, &coins, &pk)
        .expect("empty-block state validation must pass");

    assert_eq!(
        returned, block.header.state_root,
        "returned state root must match header.state_root on success"
    );
}

/// **STV-001 test plan: `validate_full_all_pass`:** Empty block through all three tiers.
#[test]
fn empty_block_validate_full_passes() {
    let block = empty_block();
    let config = ValidationConfig::default();
    let genesis = Bytes32::new([0x42; 32]);
    let coins = EmptyCoins;
    let pk = PublicKey::default();

    let returned = block
        .validate_full(&config, &genesis, &coins, &pk)
        .expect("empty-block full validation must pass");
    assert_eq!(returned, block.header.state_root);
}

/// **STV-001 test plan: `validate_full_tier1_fail`:** Tier-1 structural failure short-circuits;
/// Tier 2/3 never run. Force failure by mismatching spend_bundle_count.
#[test]
fn validate_full_short_circuits_on_tier1_failure() {
    let mut block = empty_block();
    block.header.spend_bundle_count = 99; // lie — body has 0 bundles → SVL-005 fails

    let config = ValidationConfig::default();
    let coins = EmptyCoins;
    let pk = PublicKey::default();

    let err = block
        .validate_full(&config, &Bytes32::default(), &coins, &pk)
        .expect_err("tier-1 failure must abort");
    match err {
        BlockError::SpendBundleCountMismatch { header, actual } => {
            assert_eq!(header, 99);
            assert_eq!(actual, 0);
        }
        other => panic!("expected SpendBundleCountMismatch, got {:?}", other),
    }
}

/// **STV-001 test plan: `validate_full_tier2_fail`:** Tier-2 (fee / cost consistency) failure.
/// Force by making header.total_fees non-zero on empty body — EXE-006 rejects with
/// `FeesMismatch` before Tier 3 runs.
#[test]
fn validate_full_short_circuits_on_tier2_failure() {
    let mut block = empty_block();
    block.header.total_fees = 123;

    let config = ValidationConfig::default();
    let coins = EmptyCoins;
    let pk = PublicKey::default();

    let err = block
        .validate_full(&config, &Bytes32::default(), &coins, &pk)
        .expect_err("tier-2 failure must abort");
    match err {
        BlockError::FeesMismatch { header, computed } => {
            assert_eq!(header, 123);
            assert_eq!(computed, 0);
        }
        other => panic!("expected FeesMismatch, got {:?}", other),
    }
}

/// **STV-001 test plan: `tier3_after_tier2`:** `validate_state` accepts an `ExecutionResult`
/// produced by Tier 2. Threading both sequentially yields the same result as `validate_full`.
#[test]
fn tier2_output_feeds_tier3_input() {
    let block = empty_block();
    let config = ValidationConfig::default();
    let coins = EmptyCoins;
    let pk = PublicKey::default();

    let exec = block
        .validate_execution(&config, &Bytes32::default())
        .expect("tier 2 on empty");

    let tier3 = block
        .validate_state(&exec, &coins, &pk)
        .expect("tier 3 on empty");
    let full = block
        .validate_full(&config, &Bytes32::default(), &coins, &pk)
        .expect("full on empty");

    assert_eq!(tier3, full, "split == combined for happy path");
}

/// **STV-001 acceptance:** Both methods return `Bytes32` on success, not `()` — callers use
/// the value as the parent-state commitment.
#[test]
fn success_returns_bytes32_state_root() {
    let block = empty_block();
    let exec = ExecutionResult::default();
    let coins = EmptyCoins;
    let pk = PublicKey::default();

    let out: Result<Bytes32, BlockError> = block.validate_state(&exec, &coins, &pk);
    assert!(out.is_ok());
    let _: Bytes32 = out.unwrap(); // type check
}

/// **STV-001 acceptance:** `validate_state` is object-safe via `&dyn CoinLookup` — confirmed by
/// passing `EmptyCoins` through a trait object.
#[test]
fn coin_lookup_dispatched_via_trait_object() {
    let block = empty_block();
    let exec = ExecutionResult::default();
    let boxed: Box<dyn CoinLookup> = Box::new(EmptyCoins);
    let pk = PublicKey::default();

    let out = block.validate_state(&exec, &*boxed, &pk);
    assert!(out.is_ok());
}
