//! STV-006: Proposer signature verification ([SPEC §7.5.5](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/state_validation/NORMATIVE.md` (STV-006)
//! **Spec:** `docs/requirements/domains/state_validation/specs/STV-006.md`
//!
//! ## Rule
//!
//! `chia_bls::verify(proposer_pubkey, header.hash(), block.proposer_signature)` MUST return
//! `true`. Mismatch / tampering / wrong key → [`dig_block::BlockError::InvalidProposerSignature`].
//!
//! ## Why this matters
//!
//! The proposer signature binds a block to the validator that produced it. Without this check,
//! any party could submit a forged block claiming to be from a given proposer. The check is
//! parameterized on `&PublicKey` (not embedded in the block) so the consensus layer can
//! rotate / select the expected key without rebuilding the block type.
//!
//! ## What this proves
//!
//! - **Valid signature:** A signature produced by `chia_bls::sign(secret_key, header.hash())`
//!   verifies against `secret_key.public_key()`. (Test harness signs; dig-block never calls
//!   `chia_bls::sign` per EXE-005 lint.)
//! - **Tampered signature bytes:** Modifying the signature fails verification.
//! - **Wrong public key:** A valid signature under key A fails under key B.
//! - **Tampered header:** If the header is modified after signing, the signature no longer
//!   matches `header.hash()`.
//! - **Default (zero) signature:** A freshly-default `Signature` fails unless the pubkey is
//!   also default (identity element edge case).

mod common;

use chia_bls::SecretKey;
use chia_protocol::Bytes32;
use dig_block::{
    BlockError, CoinLookup, ExecutionResult, L2Block, L2BlockHeader, PublicKey, Signature,
};

/// Empty CoinLookup — STV-006 runs regardless of coin state.
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

/// Reusable deterministic BLS key pair for test signatures. `SecretKey::from_seed` expects a
/// 32-byte seed; we use a simple constant to keep results reproducible across test runs.
fn test_key_pair() -> (SecretKey, PublicKey) {
    let seed: [u8; 32] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ];
    let sk = SecretKey::from_seed(&seed);
    let pk = sk.public_key();
    (sk, pk)
}

/// Build an empty block signed by `sk` over `header.hash()`.
fn block_signed_by(sk: &SecretKey) -> L2Block {
    let network_id = Bytes32::new([0x55; 32]);
    let l1_hash = Bytes32::new([0x66; 32]);
    let header = L2BlockHeader::genesis(network_id, 1, l1_hash);
    // Pre-sync so Tier-1 passes inside validate_full chained scenarios if used.
    let mut block = L2Block::new(header, Vec::new(), Vec::new(), Signature::default());
    common::sync_block_header_for_validate_structure(&mut block);

    let header_hash: Bytes32 = block.header.hash();
    block.proposer_signature = chia_bls::sign(sk, header_hash.as_ref());
    block
}

/// **STV-006 `valid_signature`:** proposer signs `header.hash()`; verification under the
/// matching public key succeeds.
#[test]
fn valid_signature_passes() {
    let (sk, pk) = test_key_pair();
    let block = block_signed_by(&sk);
    let exec = ExecutionResult::default();
    block
        .validate_state(&exec, &NoCoins, &pk)
        .expect("valid proposer signature must pass");
}

/// **STV-006 `invalid_signature`:** Tamper the signature bytes by swapping in a
/// `Signature::default()` (the BLS identity element, which is never a valid signature over a
/// non-trivial message). Verification fails.
#[test]
fn tampered_signature_rejected() {
    let (sk, pk) = test_key_pair();
    let mut block = block_signed_by(&sk);
    block.proposer_signature = Signature::default();

    let exec = ExecutionResult::default();
    let err = block
        .validate_state(&exec, &NoCoins, &pk)
        .expect_err("tampered signature must reject");
    assert!(matches!(err, BlockError::InvalidProposerSignature));
}

/// **STV-006 `wrong_pubkey`:** Signature is valid under key A but verified against key B → reject.
#[test]
fn wrong_pubkey_rejected() {
    let (sk_a, _pk_a) = test_key_pair();
    // Different seed -> different key pair
    let mut seed_b = [0u8; 32];
    seed_b[0] = 0xFF;
    let sk_b = SecretKey::from_seed(&seed_b);
    let pk_b = sk_b.public_key();

    let block = block_signed_by(&sk_a);
    let exec = ExecutionResult::default();
    let err = block
        .validate_state(&exec, &NoCoins, &pk_b)
        .expect_err("wrong pubkey must reject");
    assert!(matches!(err, BlockError::InvalidProposerSignature));
}

/// **STV-006 `tampered_header`:** Sign under the original header, then modify the header; the
/// hash changes and the signature is no longer valid for the new message.
#[test]
fn tampered_header_rejected() {
    let (sk, pk) = test_key_pair();
    let mut block = block_signed_by(&sk);
    // Tamper: change epoch after signing.
    block.header.epoch = 99;

    let exec = ExecutionResult::default();
    let err = block
        .validate_state(&exec, &NoCoins, &pk)
        .expect_err("tampered header must invalidate signature");
    assert!(matches!(err, BlockError::InvalidProposerSignature));
}

/// **STV-006 `zero_signature`:** All-zero Signature bytes cannot be a valid BLS signature over
/// a structured 32-byte hash under any non-identity key. Reject.
#[test]
fn zero_signature_rejected() {
    let (_sk, pk) = test_key_pair();
    let network_id = Bytes32::new([0xAA; 32]);
    let l1_hash = Bytes32::new([0xBB; 32]);
    let header = L2BlockHeader::genesis(network_id, 1, l1_hash);
    let mut block = L2Block::new(header, Vec::new(), Vec::new(), Signature::default());
    common::sync_block_header_for_validate_structure(&mut block);
    // proposer_signature is Signature::default() (all zeros)

    let exec = ExecutionResult::default();
    let err = block
        .validate_state(&exec, &NoCoins, &pk)
        .expect_err("zero signature must reject");
    assert!(matches!(err, BlockError::InvalidProposerSignature));
}
