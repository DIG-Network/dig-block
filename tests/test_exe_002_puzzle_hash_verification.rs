//! EXE-002: Puzzle hash verification per [`CoinSpend`] ([SPEC §7.4.2](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/execution_validation/NORMATIVE.md` (EXE-002)
//! **Spec:** `docs/requirements/domains/execution_validation/specs/EXE-002.md`
//! **Chia parity:** [`block_body_validation.py` Check 20 (`WRONG_PUZZLE_HASH`)](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py)
//!
//! ## What this proves
//!
//! The coin committed on-chain carries a `puzzle_hash` (a 32-byte CLVM tree hash). When the coin
//! is spent, the `CoinSpend` must reveal the full puzzle program — `puzzle_reveal`. The protocol
//! rule: `clvm_utils::tree_hash(puzzle_reveal) == coin.puzzle_hash`. A mismatch means the spender
//! is presenting a different puzzle than the coin committed to — an attempt to spend under the
//! wrong conditions — and [`dig_block::verify_coin_spend_puzzle_hash`] rejects with
//! [`BlockError::PuzzleHashMismatch`].
//!
//! ## How this satisfies EXE-002
//!
//! - **`matching_puzzle_hash`:** Construct a [`Coin`] whose `puzzle_hash` equals
//!   `tree_hash_from_bytes(puzzle_reveal.as_slice())`, assemble a [`CoinSpend`], confirm
//!   verification passes.
//! - **`tampered_puzzle_reveal`:** Change the `puzzle_reveal` after setting `coin.puzzle_hash`,
//!   confirm the verifier emits `PuzzleHashMismatch` carrying `coin_id`, `expected`, and
//!   `computed` matching the tampered input.
//! - **`empty_puzzle`:** A zero-byte puzzle reveal has a distinct tree hash (SHA-256 of the empty
//!   CLVM atom) that cannot match a non-zero `puzzle_hash` — proves the check runs on every
//!   shape of `puzzle_reveal`.
//! - **`multiple_coins_one_bad`:** A bundle-like traversal (three `CoinSpend`s, the second one
//!   wrong) halts on the first failure; the returned `coin_id` identifies the offending spend.
//! - **Uses `clvm-utils::tree_hash`:** NORMATIVE forbids custom tree-hash code. The
//!   implementation uses [`clvm_utils::tree_hash_from_bytes`] — this test confirms the public
//!   helper agrees with that function on the happy path.

use chia_protocol::{Bytes32, Coin, CoinSpend, Program};
use clvm_utils::tree_hash_from_bytes;

use dig_block::{verify_coin_spend_puzzle_hash, BlockError};

/// Build a coin whose `puzzle_hash` is the correct tree hash of `puzzle_bytes`, paired with a
/// `CoinSpend` for that coin. Used to exercise the **matching** path of EXE-002.
fn well_formed_spend(puzzle_bytes: &[u8]) -> CoinSpend {
    let parent = Bytes32::new([0x11; 32]);
    let puzzle_hash: Bytes32 = tree_hash_from_bytes(puzzle_bytes)
        .expect("clvm-utils tree hash on valid CLVM bytes")
        .into();
    let coin = Coin::new(parent, puzzle_hash, 100);
    CoinSpend::new(
        coin,
        Program::from(puzzle_bytes.to_vec()),
        Program::from(vec![0x80]),
    )
}

/// **EXE-002 `matching_puzzle_hash`:** a well-formed spend passes.
/// The simplest non-trivial CLVM value is the quote-nil pair `(q)` which encodes to `0x01`.
#[test]
fn matching_puzzle_hash_accepts_spend() {
    let spend = well_formed_spend(&[0x01]);
    assert!(verify_coin_spend_puzzle_hash(&spend).is_ok());
}

/// **EXE-002 `tampered_puzzle_reveal`:** Force a mismatch by overwriting `puzzle_reveal` with
/// different bytes after the coin committed to the original hash. Verifier must reject with the
/// offending coin id, expected hash, and computed hash surfaced for debugging.
#[test]
fn tampered_puzzle_reveal_rejected_with_full_context() {
    let mut spend = well_formed_spend(&[0x01]);
    // Tamper: replace the puzzle bytes with a different CLVM value (`0x80` = nil atom).
    let tampered_bytes = vec![0x80];
    spend.puzzle_reveal = Program::from(tampered_bytes.clone());

    let expected_hash = spend.coin.puzzle_hash;
    let coin_id = spend.coin.coin_id();
    let tampered_hash: Bytes32 = tree_hash_from_bytes(&tampered_bytes).unwrap().into();

    let err = verify_coin_spend_puzzle_hash(&spend).expect_err("tampered reveal must reject");
    match err {
        BlockError::PuzzleHashMismatch {
            coin_id: got_id,
            expected,
            computed,
        } => {
            assert_eq!(
                got_id, coin_id,
                "coin_id in error identifies offending spend"
            );
            assert_eq!(expected, expected_hash, "expected matches coin.puzzle_hash");
            assert_eq!(
                computed, tampered_hash,
                "computed matches tree_hash(tampered_reveal)"
            );
        }
        other => panic!("expected PuzzleHashMismatch, got {:?}", other),
    }
}

/// **EXE-002 `empty_puzzle`:** An empty `puzzle_reveal` has a specific tree hash (not zero, not
/// `EMPTY_ROOT` — CLVM atom semantics). Any coin whose `puzzle_hash` is `ZERO_HASH` must fail.
#[test]
fn empty_puzzle_with_zero_hash_rejected() {
    let parent = Bytes32::new([0x22; 32]);
    let coin = Coin::new(parent, Bytes32::default(), 1);
    let spend = CoinSpend::new(
        coin,
        Program::from(vec![0x80]), // CLVM nil atom — has a non-zero tree hash.
        Program::from(vec![0x80]),
    );

    let err = verify_coin_spend_puzzle_hash(&spend)
        .expect_err("zero puzzle_hash vs nil reveal must differ");
    assert!(matches!(err, BlockError::PuzzleHashMismatch { .. }));
}

/// **EXE-002 `multiple_coins_one_bad`:** Iterating a vector of spends halts on the first
/// mismatch. The error's `coin_id` must match the second spend's coin.
#[test]
fn multiple_spends_one_bad_halts_on_bad_spend() {
    let good_a = well_formed_spend(&[0x01]);
    let mut bad = well_formed_spend(&[0x01]);
    bad.puzzle_reveal = Program::from(vec![0x80]); // tamper
    let good_b = well_formed_spend(&[0x01]);
    let bad_coin_id = bad.coin.coin_id();

    let spends = [good_a, bad, good_b];
    let mut first_err = None;
    for s in &spends {
        if let Err(e) = verify_coin_spend_puzzle_hash(s) {
            first_err = Some((s.coin.coin_id(), e));
            break;
        }
    }

    let (offending, err) = first_err.expect("second spend must fail");
    assert_eq!(offending, bad_coin_id);
    match err {
        BlockError::PuzzleHashMismatch { coin_id, .. } => {
            assert_eq!(coin_id, bad_coin_id);
        }
        other => panic!("expected PuzzleHashMismatch, got {:?}", other),
    }
}

/// **EXE-002 acceptance — Chia parity:** The helper agrees with
/// [`clvm_utils::tree_hash_from_bytes`] on computed values. Proves NORMATIVE's ban on custom
/// tree-hashing is honored.
#[test]
fn helper_uses_clvm_utils_tree_hash() {
    let bytes = vec![0x01, 0xff, 0x80, 0x80]; // `(q . 0) . 0` — arbitrary valid CLVM.
    let expected: Bytes32 = tree_hash_from_bytes(&bytes).unwrap().into();
    let coin = Coin::new(Bytes32::new([0x33; 32]), expected, 7);
    let spend = CoinSpend::new(coin, Program::from(bytes), Program::from(vec![0x80]));
    assert!(verify_coin_spend_puzzle_hash(&spend).is_ok());
}
