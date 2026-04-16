//! STV-004: Addition non-existence ([SPEC §7.5.3](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/state_validation/NORMATIVE.md` (STV-004)
//! **Spec:** `docs/requirements/domains/state_validation/specs/STV-004.md`
//!
//! ## Rule
//!
//! For every [`chia_protocol::Coin`] in [`dig_block::ExecutionResult::additions`]:
//! - If [`dig_block::CoinLookup::get_coin_state`] returns `Some(_)`, the coin already exists in
//!   persistent state — reject with [`dig_block::BlockError::CoinAlreadyExists`].
//! - **Ephemeral exception:** If the coin id also appears in [`dig_block::ExecutionResult::removals`],
//!   it is ephemeral (created and spent in the same block) and MAY coexist with a pre-existing
//!   persistent coin of the same id (rare, but allowed by the ephemeral semantics).
//!
//! ## Why coin-id collisions can happen
//!
//! Coin id = `SHA256(parent_id || puzzle_hash || amount)`. Producing two coins with identical
//! `(parent, puzzle_hash, amount)` would yield the same id. Chia's structural rules forbid
//! duplicate CREATE_COIN outputs within a block (SVL-006 `DuplicateOutput`), so collisions can
//! only come from cross-block collisions — STV-004 catches those at state-validation time.
//!
//! ## What this proves
//!
//! - **New coin not in database:** passes.
//! - **Coin already exists (non-ephemeral):** rejects with `CoinAlreadyExists { coin_id }`.
//! - **Ephemeral exception:** id present in both additions and removals + exists in database →
//!   still passes (weird but legal).
//! - **Multiple additions, none in database:** all pass.
//! - **One duplicate in batch:** rejects on the offender; error carries the right coin_id.

mod common;

use chia_protocol::{Bytes32, Coin, CoinState};
use dig_block::{
    BlockError, ExecutionResult, L2Block, L2BlockHeader, PublicKey, Signature,
};

struct Coins(std::collections::HashMap<Bytes32, CoinState>);
impl Coins {
    fn new() -> Self {
        Self(std::collections::HashMap::new())
    }
    fn insert(&mut self, coin: Coin) {
        self.0.insert(
            coin.coin_id(),
            CoinState {
                coin,
                created_height: Some(1),
                spent_height: None,
            },
        );
    }
}
impl dig_block::CoinLookup for Coins {
    fn get_coin_state(&self, coin_id: &Bytes32) -> Option<CoinState> {
        self.0.get(coin_id).cloned()
    }
    fn get_chain_height(&self) -> u64 {
        100
    }
    fn get_chain_timestamp(&self) -> u64 {
        1_700_000_000
    }
}

fn empty_block_with_pk() -> (L2Block, PublicKey) {
    let network_id = Bytes32::new([0x55; 32]);
    let l1_hash = Bytes32::new([0x66; 32]);
    let header = L2BlockHeader::genesis(network_id, 1, l1_hash);
    let mut block = L2Block::new(header, Vec::new(), Vec::new(), Signature::default());
    common::sync_block_header_for_validate_structure(&mut block);
    let (sk, pk) = common::stv_test_proposer_keypair();
    common::stv_sign_proposer(&mut block, &sk);
    (block, pk)
}

/// **STV-004 `new_coin_not_in_db`:** Addition not present in CoinLookup → passes.
#[test]
fn new_addition_not_in_db_passes() {
    let (block, pk) = empty_block_with_pk();
    let new_coin = Coin::new(Bytes32::new([1; 32]), Bytes32::new([2; 32]), 100);
    let coins = Coins::new();

    let exec = ExecutionResult {
        additions: vec![new_coin],
        ..Default::default()
    };

    block
        .validate_state(&exec, &coins, &pk)
        .expect("new addition must pass");
}

/// **STV-004 `coin_already_exists`:** Addition id already in database (not in removals) →
/// `CoinAlreadyExists`.
#[test]
fn existing_non_ephemeral_addition_rejected() {
    let (block, pk) = empty_block_with_pk();
    let collide_coin = Coin::new(Bytes32::new([3; 32]), Bytes32::new([4; 32]), 50);
    let mut coins = Coins::new();
    coins.insert(collide_coin); // pre-existing in persistent state

    let exec = ExecutionResult {
        additions: vec![collide_coin],
        ..Default::default()
    };

    let err = block
        .validate_state(&exec, &coins, &pk)
        .expect_err("existing addition must reject");
    match err {
        BlockError::CoinAlreadyExists { coin_id } => {
            assert_eq!(coin_id, collide_coin.coin_id());
        }
        other => panic!("expected CoinAlreadyExists, got {:?}", other),
    }
}

/// **STV-004 `ephemeral_allowed`:** Addition id appears in both `exec.additions` and
/// `exec.removals` — ephemeral. Even if a coin with the same id exists in the database, STV-004
/// allows it (the ephemeral coin is produced + consumed within this block).
#[test]
fn ephemeral_addition_is_allowed_even_when_id_in_db() {
    let (block, pk) = empty_block_with_pk();
    let eph = Coin::new(Bytes32::new([5; 32]), Bytes32::new([6; 32]), 10);
    let mut coins = Coins::new();
    coins.insert(eph); // pre-existing

    let exec = ExecutionResult {
        additions: vec![eph],
        removals: vec![eph.coin_id()],
        ..Default::default()
    };

    block
        .validate_state(&exec, &coins, &pk)
        .expect("ephemeral addition must pass STV-004");
}

/// **STV-004 `multiple_additions`:** Several new coins, none in database → all pass.
#[test]
fn multiple_new_additions_all_pass() {
    let (block, pk) = empty_block_with_pk();
    let a = Coin::new(Bytes32::new([7; 32]), Bytes32::new([8; 32]), 1);
    let b = Coin::new(Bytes32::new([9; 32]), Bytes32::new([0xA; 32]), 2);
    let c = Coin::new(Bytes32::new([0xB; 32]), Bytes32::new([0xC; 32]), 3);
    let coins = Coins::new();

    let exec = ExecutionResult {
        additions: vec![a, b, c],
        ..Default::default()
    };

    block
        .validate_state(&exec, &coins, &pk)
        .expect("all new additions must pass");
}

/// **STV-004 `one_duplicate_in_batch`:** 3 additions; the middle one collides with persistent
/// state and is not ephemeral. Halts on the offender; coin_id identifies it.
#[test]
fn batch_with_one_duplicate_rejects_on_offender() {
    let (block, pk) = empty_block_with_pk();
    let good_a = Coin::new(Bytes32::new([0x11; 32]), Bytes32::new([0x12; 32]), 1);
    let bad = Coin::new(Bytes32::new([0x13; 32]), Bytes32::new([0x14; 32]), 2);
    let good_b = Coin::new(Bytes32::new([0x15; 32]), Bytes32::new([0x16; 32]), 3);

    let mut coins = Coins::new();
    coins.insert(bad); // middle addition already exists

    let exec = ExecutionResult {
        additions: vec![good_a, bad, good_b],
        ..Default::default()
    };

    let err = block
        .validate_state(&exec, &coins, &pk)
        .expect_err("duplicate must reject");
    match err {
        BlockError::CoinAlreadyExists { coin_id } => {
            assert_eq!(coin_id, bad.coin_id(), "error must identify offender");
        }
        other => panic!("expected CoinAlreadyExists, got {:?}", other),
    }
}
