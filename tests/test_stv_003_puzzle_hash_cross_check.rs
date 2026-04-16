//! STV-003: Puzzle hash cross-check (Tier 3) per [SPEC §7.5.2](docs/resources/SPEC.md).
//!
//! **Normative:** `docs/requirements/domains/state_validation/NORMATIVE.md` (STV-003)
//! **Spec:** `docs/requirements/domains/state_validation/specs/STV-003.md`
//! **Chia parity:** [`block_body_validation.py` Check 20 (`WRONG_PUZZLE_HASH`)](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py)
//!
//! ## Rule
//!
//! For every [`chia_protocol::CoinSpend`] in every [`chia_protocol::SpendBundle`]:
//! - If `CoinLookup.get_coin_state(coin_spend.coin.coin_id())` returns `Some(state)`:
//!   - `state.coin.puzzle_hash` MUST equal `coin_spend.coin.puzzle_hash`.
//!   - Mismatch → [`dig_block::BlockError::PuzzleHashMismatch`] with `expected` = state value,
//!     `computed` = declared value.
//! - If `None` (ephemeral coin), skip — STV-004 handles it.
//!
//! ## Complementary to EXE-002
//!
//! EXE-002 proved `tree_hash(puzzle_reveal) == coin_spend.coin.puzzle_hash`. STV-003 closes
//! the chain: `coin_state.puzzle_hash == coin_spend.coin.puzzle_hash`. Both together mean:
//! `tree_hash(puzzle_reveal) == coin_state.puzzle_hash` — the spender revealed the right puzzle
//! for the committed coin.
//!
//! ## What this proves
//!
//! - **Matching puzzle hash:** `state.coin.puzzle_hash` equals `coin_spend.coin.puzzle_hash` →
//!   passes.
//! - **Mismatched puzzle hash:** Different values → rejects with expected/computed context.
//! - **Ephemeral skip:** Coin not in lookup is not checked at this tier (STV-002 covers
//!   existence; ephemeral coins are spent same-block and need no cross-check since the
//!   producer already committed to `coin.puzzle_hash`).
//! - **Multiple spends, one bad:** Halts on the first mismatch; returned error identifies it.

mod common;

use chia_protocol::{Bytes32, Coin, CoinSpend, CoinState, Program, SpendBundle};
use dig_block::{
    BlockError, ExecutionResult, L2Block, L2BlockHeader, PublicKey, Signature,
};

struct Coins(std::collections::HashMap<Bytes32, CoinState>);
impl Coins {
    fn new() -> Self {
        Self(std::collections::HashMap::new())
    }
    fn add_with_puzzle(&mut self, parent: Bytes32, puzzle_hash: Bytes32, amount: u64) -> Coin {
        let coin = Coin::new(parent, puzzle_hash, amount);
        let state = CoinState {
            coin,
            created_height: Some(1),
            spent_height: None,
        };
        self.0.insert(coin.coin_id(), state);
        coin
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

/// Build a block with one `CoinSpend` whose committed `coin` uses `declared_puzzle_hash`
/// (may differ from whatever the CoinLookup returns) — isolates STV-003 behavior from the
/// coin-existence STV-002 path.
fn block_with_spend(coin: Coin) -> L2Block {
    let network_id = Bytes32::new([0x77; 32]);
    let l1_hash = Bytes32::new([0x88; 32]);
    let header = L2BlockHeader::genesis(network_id, 1, l1_hash);
    let cs = CoinSpend::new(coin, Program::from(vec![1]), Program::from(vec![0x80]));
    let bundle = SpendBundle::new(vec![cs], Signature::default());
    let mut block = L2Block::new(header, vec![bundle], Vec::new(), Signature::default());
    common::sync_block_header_for_validate_structure(&mut block);
    block
}

/// **STV-003 `matching_puzzle_hash`:** Database and spend agree — passes.
#[test]
fn matching_puzzle_hash_passes() {
    let mut coins = Coins::new();
    let parent = Bytes32::new([0x11; 32]);
    let puzzle = Bytes32::new([0x22; 32]);
    let coin = coins.add_with_puzzle(parent, puzzle, 50);

    let block = block_with_spend(coin);
    let exec = ExecutionResult {
        removals: vec![coin.coin_id()],
        ..Default::default()
    };
    let pk = PublicKey::default();

    block
        .validate_state(&exec, &coins, &pk)
        .expect("matching puzzle hash must pass");
}

/// **STV-003 `mismatched_puzzle_hash`:** Database stores a different puzzle hash than the
/// spend declares — rejects with expected (state) / computed (declared).
#[test]
fn mismatched_puzzle_hash_rejected() {
    let mut coins = Coins::new();
    let parent = Bytes32::new([0x33; 32]);
    let actual_puzzle = Bytes32::new([0x44; 32]);
    // The spend **would** commit to `actual_puzzle`, but we forge the CoinSpend to carry a
    // different puzzle_hash. Note: the coin_id of the on-chain coin depends on the ACTUAL
    // puzzle_hash, so coins must be keyed that way; the CoinSpend must reference that same
    // coin_id (otherwise STV-002 would fail first with CoinNotFound).
    let on_chain_coin = coins.add_with_puzzle(parent, actual_puzzle, 100);

    // Forge a CoinSpend claiming the same parent + amount but a different puzzle_hash.
    // The resulting CoinSpend.coin.coin_id() differs from on_chain_coin.coin_id(), so STV-002
    // would raise CoinNotFound first. To isolate STV-003 we need the spend to reference the
    // on_chain coin_id (same parent/puzzle_hash/amount as stored), but with the stored
    // CoinState entry tampered to have a different puzzle_hash. We thus swap at the CoinState
    // level:
    let wrong_puzzle = Bytes32::new([0x55; 32]);
    let state_with_wrong_puzzle = CoinState {
        coin: Coin::new(parent, wrong_puzzle, 100),
        created_height: Some(1),
        spent_height: None,
    };
    // Insert under the real coin_id so STV-002 finds it, but the puzzle_hash inside differs.
    coins.0.insert(on_chain_coin.coin_id(), state_with_wrong_puzzle);

    let block = block_with_spend(on_chain_coin);
    let exec = ExecutionResult {
        removals: vec![on_chain_coin.coin_id()],
        ..Default::default()
    };
    let pk = PublicKey::default();

    let err = block
        .validate_state(&exec, &coins, &pk)
        .expect_err("mismatched puzzle hash must reject");
    match err {
        BlockError::PuzzleHashMismatch {
            coin_id,
            expected,
            computed,
        } => {
            assert_eq!(coin_id, on_chain_coin.coin_id());
            // `expected` is what CoinState reports; `computed` is what the spend declared.
            assert_eq!(expected, wrong_puzzle);
            assert_eq!(computed, actual_puzzle);
        }
        other => panic!("expected PuzzleHashMismatch, got {:?}", other),
    }
}

/// **STV-003 `ephemeral_skipped`:** Coin not in lookup is not STV-003 rejected — STV-004 /
/// STV-002 handle ephemeral coins separately.
#[test]
fn ephemeral_coin_not_checked_by_stv003() {
    let coins = Coins::new(); // empty
    let ephemeral = Coin::new(Bytes32::new([0x66; 32]), Bytes32::new([0x77; 32]), 10);
    let block = block_with_spend(ephemeral);

    let exec = ExecutionResult {
        additions: vec![ephemeral], // makes STV-002 treat it as ephemeral
        removals: vec![ephemeral.coin_id()],
        ..Default::default()
    };
    let pk = PublicKey::default();

    // Should pass: STV-002 sees it as ephemeral; STV-003 skips when get_coin_state = None.
    block
        .validate_state(&exec, &coins, &pk)
        .expect("ephemeral spend must skip STV-003");
}

/// **STV-003 `multiple_spends_one_bad`:** Two bundles; second has a puzzle mismatch. Validator
/// halts on the offending spend; the error identifies it via `coin_id`.
#[test]
fn multiple_spends_one_bad_halts() {
    let mut coins = Coins::new();
    let good_coin =
        coins.add_with_puzzle(Bytes32::new([0xA1; 32]), Bytes32::new([0xA2; 32]), 10);

    // Bad path: insert a state whose puzzle_hash differs from what the spend declares.
    let bad_parent = Bytes32::new([0xB1; 32]);
    let bad_declared = Bytes32::new([0xB2; 32]);
    let bad_coin = Coin::new(bad_parent, bad_declared, 5);
    let tampered_state_puzzle = Bytes32::new([0xC3; 32]);
    coins.0.insert(
        bad_coin.coin_id(),
        CoinState {
            coin: Coin::new(bad_parent, tampered_state_puzzle, 5),
            created_height: Some(1),
            spent_height: None,
        },
    );

    // Block with two SpendBundles — good one first, then the bad one.
    let header = L2BlockHeader::genesis(Bytes32::new([0x99; 32]), 1, Bytes32::new([0xAA; 32]));
    let good_cs = CoinSpend::new(good_coin, Program::from(vec![1]), Program::from(vec![0x80]));
    let bad_cs = CoinSpend::new(bad_coin, Program::from(vec![1]), Program::from(vec![0x80]));
    let good_bundle = SpendBundle::new(vec![good_cs], Signature::default());
    let bad_bundle = SpendBundle::new(vec![bad_cs], Signature::default());
    let mut block = L2Block::new(
        header,
        vec![good_bundle, bad_bundle],
        Vec::new(),
        Signature::default(),
    );
    common::sync_block_header_for_validate_structure(&mut block);

    let exec = ExecutionResult {
        removals: vec![good_coin.coin_id(), bad_coin.coin_id()],
        ..Default::default()
    };
    let pk = PublicKey::default();

    let err = block
        .validate_state(&exec, &coins, &pk)
        .expect_err("second spend must trigger PuzzleHashMismatch");
    match err {
        BlockError::PuzzleHashMismatch { coin_id, .. } => {
            assert_eq!(
                coin_id,
                bad_coin.coin_id(),
                "error must identify offending spend"
            );
        }
        other => panic!("expected PuzzleHashMismatch, got {:?}", other),
    }
}
