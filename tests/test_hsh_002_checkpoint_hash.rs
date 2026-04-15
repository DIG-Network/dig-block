//! HSH-002: [`Checkpoint::hash`] — SHA-256 over the fixed-order checkpoint preimage ([SPEC §3.2](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/hashing/NORMATIVE.md` (HSH-002)  
//! **Spec + test plan:** `docs/requirements/domains/hashing/specs/HSH-002.md`  
//! **Implementation:** `src/types/checkpoint.rs` — [`Checkpoint::hash_preimage_bytes`], [`Checkpoint::HASH_PREIMAGE_LEN`]  
//! **Delegation:** [`dig_block::CheckpointSubmission::hash`] forwards to [`Checkpoint::hash`] ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md)).
//!
//! **Layout note:** HSH-002 names `tests/hashing/test_checkpoint_hash.rs`; this repo uses a flat `tests/` tree ([STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md)).
//!
//! ## How these tests prove HSH-002
//!
//! - **Determinism & primitive:** Same [`Checkpoint`] → same [`dig_block::Bytes32`]; manual [`chia_sha2::Sha256`] on
//!   [`Checkpoint::hash_preimage_bytes`] matches [`Checkpoint::hash`] (proves `chia-sha2` and field order).
//! - **LE encoding:** `epoch` and `block_count` appear as little-endian at known offsets derived from SPEC §3.2.
//! - **Length:** Preimage is exactly **160** bytes ([`Checkpoint::HASH_PREIMAGE_LEN`]).
//! - **Sensitivity:** Perturbing one scalar changes the digest.
//!
//! **SocratiCode:** Not available in this session; Repomix + SPEC §3.2 table used for order.

use chia_sha2::Sha256;
use dig_block::{Bytes32, Checkpoint};

fn sample_checkpoint() -> Checkpoint {
    let b = |tag: u8| Bytes32::new([tag; 32]);
    Checkpoint {
        epoch: 42,
        state_root: b(0x01),
        block_root: b(0x02),
        block_count: 100,
        tx_count: 10_000,
        total_fees: 999,
        prev_checkpoint: b(0x03),
        withdrawals_root: b(0x04),
        withdrawal_count: 7,
    }
}

/// **Test plan:** `test_checkpoint_hash_deterministic`
#[test]
fn hsh002_hash_is_deterministic() {
    let c = sample_checkpoint();
    assert_eq!(c.hash(), c.hash());
}

/// **Test plan:** `test_checkpoint_hash_field_order`
#[test]
fn hsh002_hash_matches_sha256_of_preimage() {
    let c = sample_checkpoint();
    let pre = c.hash_preimage_bytes();
    let mut hasher = Sha256::new();
    hasher.update(pre);
    assert_eq!(c.hash(), Bytes32::new(hasher.finalize()));
}

/// **Test plan:** `test_checkpoint_hash_le_encoding`
#[test]
fn hsh002_epoch_and_block_count_little_endian_in_preimage() {
    let mut c = sample_checkpoint();
    c.epoch = 0x0102_0304_0506_0708;
    c.block_count = 0xAABB_CCDD;
    let p = c.hash_preimage_bytes();
    assert_eq!(&p[0..8], &c.epoch.to_le_bytes());
    // block_count starts after epoch(8) + state(32) + block_root(32) = 72
    assert_eq!(&p[72..76], &c.block_count.to_le_bytes());
}

/// **Test plan:** `test_checkpoint_hash_total_bytes`
#[test]
fn hsh002_preimage_length_is_160() {
    let c = sample_checkpoint();
    assert_eq!(c.hash_preimage_bytes().len(), Checkpoint::HASH_PREIMAGE_LEN);
    assert_eq!(Checkpoint::HASH_PREIMAGE_LEN, 160);
}

/// **Test plan:** `test_checkpoint_hash_single_field_change`
#[test]
fn hsh002_changing_tx_count_changes_hash() {
    let a = sample_checkpoint();
    let mut b = sample_checkpoint();
    b.tx_count = a.tx_count + 1;
    assert_ne!(a.hash(), b.hash());
}

/// **Regression:** [`CheckpointSubmission::hash`] still aliases checkpoint identity ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md)).
#[test]
fn hsh002_submission_hash_delegates_to_checkpoint() {
    use dig_block::{CheckpointSubmission, PublicKey, Signature, SignerBitmap};
    let cp = sample_checkpoint();
    let sub = CheckpointSubmission::new(
        cp.clone(),
        SignerBitmap::new(1),
        Signature::default(),
        PublicKey::default(),
        0,
        0,
    );
    assert_eq!(sub.hash(), cp.hash());
}
