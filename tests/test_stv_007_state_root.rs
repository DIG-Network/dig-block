//! STV-007: State root verification ([SPEC §7.5.6](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/state_validation/NORMATIVE.md` (STV-007)
//! **Spec:** `docs/requirements/domains/state_validation/specs/STV-007.md`
//!
//! ## Rule
//!
//! After applying the block's additions and removals to the parent state, the recomputed
//! state-trie root MUST equal `self.header.state_root`. On success, `validate_state` returns
//! the computed root; on mismatch it rejects with
//! [`dig_block::BlockError::InvalidStateRoot { header, computed }`].
//!
//! ## Implementation caveat — interim state-root formula
//!
//! NORMATIVE STV-007 assumes the existence of a state tree ("sparse Merkle tree or Patricia
//! trie"); chia-protocol `CoinState` does not carry a root and [`dig_block::CoinLookup`] does
//! not yet expose `get_state_tree()`. The implementation here computes a deterministic delta
//! hash over the sorted addition coin ids and sorted removal coin ids using the same SHA-256
//! + tagged-Merkle primitives already in dig-block (HSH-007). This suffices for:
//!
//! 1. **Match / mismatch detection** — the block header's declared `state_root` must equal the
//!    computed delta hash. Producers and validators using the same helper agree deterministically.
//! 2. **Empty block** — zero additions, zero removals yields [`dig_block::EMPTY_ROOT`].
//! 3. **Determinism** — sort-before-hash ensures ordering in `exec.additions` /
//!    `exec.removals` does not change the computed value.
//!
//! Full sparse-Merkle state-root computation is an extension to `CoinLookup` tracked as a
//! follow-on; this requirement's acceptance criteria (comparison + mismatch rejection) are met
//! by the interim formula.
//!
//! ## What this proves
//!
//! - **Empty-block root:** `additions.is_empty() && removals.is_empty()` → `EMPTY_ROOT`. Header
//!   must declare `EMPTY_ROOT` (or set via genesis) to pass.
//! - **Mismatch rejects:** Changing any addition or removal without updating `header.state_root`
//!   triggers `InvalidStateRoot { header, computed }` with both values surfaced.
//! - **Determinism:** Same inputs produce same root; reordering additions / removals does not
//!   affect the computed value.
//! - **`validate_state` returns the computed root on success:** Callers receive the state-root
//!   to use as the parent commitment for the next block.

mod common;

use chia_protocol::{Bytes32, Coin};
use dig_block::{
    compute_state_root_from_delta, BlockError, CoinLookup, ExecutionResult, L2Block,
    L2BlockHeader, PublicKey, Signature, EMPTY_ROOT,
};

struct NoCoins;
impl CoinLookup for NoCoins {
    fn get_coin_state(&self, _coin_id: &Bytes32) -> Option<chia_protocol::CoinState> {
        None
    }
    fn get_chain_height(&self) -> u64 {
        100
    }
    fn get_chain_timestamp(&self) -> u64 {
        1_700_000_000
    }
}

fn make_block(state_root: Bytes32) -> (L2Block, PublicKey) {
    let network_id = Bytes32::new([0x77; 32]);
    let l1_hash = Bytes32::new([0x88; 32]);
    let mut header = L2BlockHeader::genesis(network_id, 1, l1_hash);
    header.state_root = state_root;

    let mut block = L2Block::new(header, Vec::new(), Vec::new(), Signature::default());
    common::sync_block_header_for_validate_structure(&mut block);
    let (sk, pk) = common::stv_test_proposer_keypair();
    common::stv_sign_proposer(&mut block, &sk);
    (block, pk)
}

/// **STV-007 `empty_block_root`:** Zero additions + zero removals → `EMPTY_ROOT`. When the
/// header declares `EMPTY_ROOT` (as genesis does), validation returns `EMPTY_ROOT`.
#[test]
fn empty_block_returns_empty_root() {
    let (block, pk) = make_block(EMPTY_ROOT);
    let exec = ExecutionResult::default();
    let returned = block
        .validate_state(&exec, &NoCoins, &pk)
        .expect("empty delta -> EMPTY_ROOT");
    assert_eq!(returned, EMPTY_ROOT);
    assert_eq!(returned, block.header.state_root);
}

/// **STV-007 `invalid_state_root`:** Header declares a state root that doesn't match the
/// computed delta — `InvalidStateRoot { header, computed }` surfaces both values.
#[test]
fn mismatched_header_state_root_rejected() {
    let wrong_root = Bytes32::new([0x99; 32]);
    let (block, pk) = make_block(wrong_root);
    let exec = ExecutionResult::default();

    let err = block
        .validate_state(&exec, &NoCoins, &pk)
        .expect_err("mismatched state root must reject");

    match err {
        BlockError::InvalidStateRoot { expected, computed } => {
            // `expected` = what the header claimed; `computed` = what we derived.
            assert_eq!(expected, wrong_root);
            assert_eq!(computed, EMPTY_ROOT);
        }
        other => panic!("expected InvalidStateRoot, got {:?}", other),
    }
}

/// **STV-007 `additions_only`:** Block with only new coins. Helper produces a non-empty root.
/// Header must declare that exact value to pass.
#[test]
fn additions_only_header_must_match() {
    let a = Coin::new(Bytes32::new([1; 32]), Bytes32::new([2; 32]), 10);
    let b = Coin::new(Bytes32::new([3; 32]), Bytes32::new([4; 32]), 20);
    let expected = compute_state_root_from_delta(&[a, b], &[]);
    let (block, pk) = make_block(expected);
    let exec = ExecutionResult {
        additions: vec![a, b],
        ..Default::default()
    };
    let returned = block.validate_state(&exec, &NoCoins, &pk).expect("match");
    assert_eq!(returned, expected);
}

/// **STV-007 `removals_only`:** Block with only spent coin ids. Helper's sorted-hash is the
/// deterministic root. Each removal must satisfy STV-002 — the mock CoinLookup records the
/// coins as existing + unspent.
#[test]
fn removals_only_header_must_match() {
    let coin1 = Coin::new(Bytes32::new([0xA1; 32]), Bytes32::new([0xA2; 32]), 1);
    let coin2 = Coin::new(Bytes32::new([0xB1; 32]), Bytes32::new([0xB2; 32]), 2);
    let expected = compute_state_root_from_delta(&[], &[coin1.coin_id(), coin2.coin_id()]);
    let (block, pk) = make_block(expected);

    // CoinLookup must report these as existing + unspent for STV-002 to pass.
    struct WithCoins {
        coins: std::collections::HashMap<Bytes32, chia_protocol::CoinState>,
    }
    impl CoinLookup for WithCoins {
        fn get_coin_state(&self, coin_id: &Bytes32) -> Option<chia_protocol::CoinState> {
            self.coins.get(coin_id).cloned()
        }
        fn get_chain_height(&self) -> u64 {
            100
        }
        fn get_chain_timestamp(&self) -> u64 {
            1_700_000_000
        }
    }
    let mut coins = std::collections::HashMap::new();
    for c in [coin1, coin2] {
        coins.insert(
            c.coin_id(),
            chia_protocol::CoinState {
                coin: c,
                created_height: Some(1),
                spent_height: None,
            },
        );
    }

    let exec = ExecutionResult {
        removals: vec![coin1.coin_id(), coin2.coin_id()],
        ..Default::default()
    };
    let returned = block
        .validate_state(&exec, &WithCoins { coins }, &pk)
        .expect("match");
    assert_eq!(returned, expected);
}

/// **STV-007 determinism:** Reordering additions / removals must not change the computed root.
#[test]
fn reordering_does_not_change_root() {
    let a = Coin::new(Bytes32::new([1; 32]), Bytes32::new([2; 32]), 10);
    let b = Coin::new(Bytes32::new([3; 32]), Bytes32::new([4; 32]), 20);
    let r1 = Bytes32::new([0xA; 32]);
    let r2 = Bytes32::new([0xB; 32]);

    let root_1 = compute_state_root_from_delta(&[a, b], &[r1, r2]);
    let root_2 = compute_state_root_from_delta(&[b, a], &[r2, r1]);
    assert_eq!(root_1, root_2, "sort-before-hash makes ordering irrelevant");
}

/// **STV-007:** Empty slices produce `EMPTY_ROOT` (module contract).
#[test]
fn empty_delta_produces_empty_root_helper() {
    assert_eq!(compute_state_root_from_delta(&[], &[]), EMPTY_ROOT);
}

/// **STV-007 `ephemeral_handling`:** Coin created and spent in the same block appears in both
/// vectors. Header state root must reflect the combined set.
#[test]
fn ephemeral_coin_included_in_root_calculation() {
    let eph = Coin::new(Bytes32::new([5; 32]), Bytes32::new([6; 32]), 7);
    let expected = compute_state_root_from_delta(&[eph], &[eph.coin_id()]);
    let (block, pk) = make_block(expected);
    let exec = ExecutionResult {
        additions: vec![eph],
        removals: vec![eph.coin_id()],
        ..Default::default()
    };
    let returned = block.validate_state(&exec, &NoCoins, &pk).expect("match");
    assert_eq!(returned, expected);
}

/// **STV-007:** `validate_state` returning `Bytes32` means callers can chain blocks — use the
/// returned value as the `header.state_root` for the child.
#[test]
fn return_value_is_usable_as_parent_state_commitment() {
    let (block, pk) = make_block(EMPTY_ROOT);
    let exec = ExecutionResult::default();
    let root: Bytes32 = block.validate_state(&exec, &NoCoins, &pk).unwrap();
    assert_eq!(root, EMPTY_ROOT);
    // Chain: next block would set `header.parent_hash = block.hash()` and
    // `header.state_root = root` (or recompute after its own delta).
}
