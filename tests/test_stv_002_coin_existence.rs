//! STV-002: Coin existence checks for every removal ([SPEC §7.5.1](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/state_validation/NORMATIVE.md` (STV-002)
//! **Spec:** `docs/requirements/domains/state_validation/specs/STV-002.md`
//! **Chia parity:** [`block_body_validation.py` Check 15 (UNKNOWN_UNSPENT / DOUBLE_SPEND)](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py)
//!
//! ## Rule
//!
//! For every coin ID in [`dig_block::ExecutionResult::removals`]:
//! 1. Look up via [`dig_block::CoinLookup::get_coin_state`].
//! 2. If `Some(coin_state)` and `coin_state.spent_height.is_some()`, reject with
//!    [`dig_block::BlockError::CoinAlreadySpent`].
//! 3. If `None`, the coin must be **ephemeral** — its ID must appear in
//!    [`dig_block::ExecutionResult::additions`]. If not, reject with
//!    [`dig_block::BlockError::CoinNotFound`].
//!
//! ## What this proves
//!
//! - **Persistent coin exists and unspent:** passes.
//! - **Persistent coin already spent:** `CoinAlreadySpent { coin_id }` with the correct id.
//! - **Missing coin, not ephemeral:** `CoinNotFound { coin_id }`.
//! - **Ephemeral coin (in `exec.additions`):** passes even when CoinLookup returns `None`.
//! - **Mixed removals:** one persistent + one ephemeral → both pass.
//! - **Double-spend attempt (same id twice):** second occurrence is caught on its own lookup
//!   (persistent → `CoinAlreadySpent` when spent_height was set by a prior block; within the
//!   same block, removals collected by Tier 2 would be the same value twice, and the validator
//!   treats the second visit identically to the first — dedup / double-spend within the block
//!   is caught structurally by SVL-006 before state validation runs).
//!
//! ## How this satisfies STV-002
//!
//! One test per acceptance-criteria bullet.

mod common;

use chia_protocol::{Bytes32, Coin, CoinState};
use dig_block::{BlockError, ExecutionResult, L2Block, L2BlockHeader, PublicKey, Signature};

/// Mock CoinLookup populated with an explicit coin map and chain context.
struct Coins {
    coins: std::collections::HashMap<Bytes32, CoinState>,
}

impl Coins {
    fn new() -> Self {
        Self {
            coins: std::collections::HashMap::new(),
        }
    }
    fn add(&mut self, coin: Coin, spent_height: Option<u32>) {
        let state = CoinState {
            coin,
            created_height: Some(1),
            spent_height,
        };
        self.coins.insert(coin.coin_id(), state);
    }
}

impl dig_block::CoinLookup for Coins {
    fn get_coin_state(&self, coin_id: &Bytes32) -> Option<CoinState> {
        self.coins.get(coin_id).cloned()
    }
    fn get_chain_height(&self) -> u64 {
        100
    }
    fn get_chain_timestamp(&self) -> u64 {
        1_700_000_000
    }
}

/// Build an empty L2Block with a matching proposer signature and return `(block, pk)` so
/// tests call `validate_state(..., &pk)` — STV-006 runs inside validate_state and requires
/// `chia_bls::verify(sig, pk, header.hash()) == true`.
fn empty_block_with_pk() -> (L2Block, PublicKey) {
    let network_id = Bytes32::new([0xAB; 32]);
    let l1_hash = Bytes32::new([0xCD; 32]);
    let header = L2BlockHeader::genesis(network_id, 1, l1_hash);
    let mut block = L2Block::new(header, Vec::new(), Vec::new(), Signature::default());
    common::sync_block_header_for_validate_structure(&mut block);
    let (sk, pk) = common::stv_test_proposer_keypair();
    common::stv_sign_proposer(&mut block, &sk);
    (block, pk)
}

/// **STV-002 `coin_exists_unspent`:** Coin exists in lookup with `spent_height=None` → passes.
#[test]
fn persistent_unspent_coin_passes() {
    let (block, pk) = empty_block_with_pk();
    let coin = Coin::new(Bytes32::new([1; 32]), Bytes32::new([2; 32]), 100);
    let mut coins = Coins::new();
    coins.add(coin, None);

    let exec = ExecutionResult {
        removals: vec![coin.coin_id()],
        ..Default::default()
    };

    block
        .validate_state(&exec, &coins, &pk)
        .expect("unspent persistent coin must pass");
}

/// **STV-002 `coin_exists_spent`:** Coin in lookup with `spent_height=Some(42)` → `CoinAlreadySpent`.
#[test]
fn persistent_already_spent_coin_rejected() {
    let (block, pk) = empty_block_with_pk();
    let coin = Coin::new(Bytes32::new([3; 32]), Bytes32::new([4; 32]), 50);
    let coin_id = coin.coin_id();
    let mut coins = Coins::new();
    coins.add(coin, Some(42));

    let exec = ExecutionResult {
        removals: vec![coin_id],
        ..Default::default()
    };

    let err = block
        .validate_state(&exec, &coins, &pk)
        .expect_err("already-spent coin must reject");
    match err {
        BlockError::CoinAlreadySpent {
            coin_id: got_id, ..
        } => {
            assert_eq!(got_id, coin_id);
        }
        other => panic!("expected CoinAlreadySpent, got {:?}", other),
    }
}

/// **STV-002 `coin_not_found`:** Coin not in lookup AND not in `exec.additions` → `CoinNotFound`.
#[test]
fn missing_non_ephemeral_coin_rejected() {
    let (block, pk) = empty_block_with_pk();
    let missing_id = Bytes32::new([0xEE; 32]);
    let coins = Coins::new(); // empty

    let exec = ExecutionResult {
        removals: vec![missing_id],
        ..Default::default()
    };

    let err = block
        .validate_state(&exec, &coins, &pk)
        .expect_err("missing non-ephemeral coin must reject");
    match err {
        BlockError::CoinNotFound { coin_id } => assert_eq!(coin_id, missing_id),
        other => panic!("expected CoinNotFound, got {:?}", other),
    }
}

/// **STV-002 `ephemeral_coin`:** Coin absent from lookup but present in `exec.additions` → passes.
/// Ephemeral = created and spent in the same block; never in persistent state.
#[test]
fn ephemeral_coin_passes_without_coin_lookup_entry() {
    let (block, pk) = empty_block_with_pk();
    let ephemeral = Coin::new(Bytes32::new([5; 32]), Bytes32::new([6; 32]), 7);
    let coins = Coins::new(); // empty

    let exec = ExecutionResult {
        additions: vec![ephemeral],
        removals: vec![ephemeral.coin_id()],
        ..Default::default()
    };

    block
        .validate_state(&exec, &coins, &pk)
        .expect("ephemeral coin must pass");
}

/// **STV-002 `mixed_removals`:** Persistent unspent + ephemeral coexist in the same block.
#[test]
fn mixed_persistent_and_ephemeral_all_pass() {
    let (block, pk) = empty_block_with_pk();
    let persistent = Coin::new(Bytes32::new([7; 32]), Bytes32::new([8; 32]), 10);
    let ephemeral = Coin::new(Bytes32::new([9; 32]), Bytes32::new([0xA; 32]), 20);

    let mut coins = Coins::new();
    coins.add(persistent, None);

    let exec = ExecutionResult {
        additions: vec![ephemeral],
        removals: vec![persistent.coin_id(), ephemeral.coin_id()],
        ..Default::default()
    };

    block
        .validate_state(&exec, &coins, &pk)
        .expect("mixed removals must pass");
}

/// **STV-002 — order doesn't matter:** Putting ephemeral before persistent in the removals
/// vector still works — the check iterates and each lookup is independent.
#[test]
fn removal_order_does_not_matter() {
    let (block, pk) = empty_block_with_pk();
    let persistent = Coin::new(Bytes32::new([0xB; 32]), Bytes32::new([0xC; 32]), 10);
    let ephemeral = Coin::new(Bytes32::new([0xD; 32]), Bytes32::new([0xE; 32]), 20);

    let mut coins = Coins::new();
    coins.add(persistent, None);

    let exec = ExecutionResult {
        additions: vec![ephemeral],
        removals: vec![ephemeral.coin_id(), persistent.coin_id()],
        ..Default::default()
    };

    block
        .validate_state(&exec, &coins, &pk)
        .expect("reverse removal order must still pass");
}

/// **STV-002 first-fail ordering:** Multiple bad removals — returns the first one encountered.
/// Implementation iterates `exec.removals` in order; this documents that behavior.
#[test]
fn first_failing_removal_is_reported() {
    let (block, pk) = empty_block_with_pk();
    let coin_a = Coin::new(Bytes32::new([0x10; 32]), Bytes32::new([0x11; 32]), 1);
    let coin_b = Coin::new(Bytes32::new([0x12; 32]), Bytes32::new([0x13; 32]), 2);

    let mut coins = Coins::new();
    coins.add(coin_a, Some(5)); // already spent
    // coin_b not added -> would be CoinNotFound

    let exec = ExecutionResult {
        removals: vec![coin_a.coin_id(), coin_b.coin_id()],
        ..Default::default()
    };

    let err = block
        .validate_state(&exec, &coins, &pk)
        .expect_err("must reject first failure");
    // First in list is coin_a (already spent), so CoinAlreadySpent is expected.
    match err {
        BlockError::CoinAlreadySpent { coin_id, .. } => {
            assert_eq!(coin_id, coin_a.coin_id());
        }
        other => panic!("expected CoinAlreadySpent for first-in-list, got {:?}", other),
    }
}
