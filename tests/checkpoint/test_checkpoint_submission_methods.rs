//! CKP-005: [`dig_block::CheckpointSubmission`] query + L1 recording API per NORMATIVE.
//!
//! **Normative:** `docs/requirements/domains/checkpoint/NORMATIVE.md` (CKP-005)  
//! **Spec + test plan:** `docs/requirements/domains/checkpoint/specs/CKP-005.md`  
//! **Implementation:** `src/types/checkpoint.rs`
//!
//! ## How these tests prove CKP-005
//!
//! - **Delegation:** [`CheckpointSubmission::hash`] / [`CheckpointSubmission::epoch`] must match the nested
//!   [`Checkpoint`] without manual field drift ([CKP-005 § hash / epoch](docs/requirements/domains/checkpoint/specs/CKP-005.md#specification)). `hash()` requires [`Checkpoint::hash`] ([HSH-002](docs/requirements/domains/hashing/specs/HSH-002.md) / [SPEC §3.2](docs/resources/SPEC.md)).
//! - **Attestation:** [`CheckpointSubmission::signing_percentage`] and [`CheckpointSubmission::meets_threshold`]
//!   mirror [`SignerBitmap::signing_percentage`] and [`SignerBitmap::has_threshold`] (ATT-004) — tests compare
//!   directly so a broken delegate cannot pass silently.
//! - **L1 tracking:** [`CheckpointSubmission::record_submission`] fills both options; [`CheckpointSubmission::is_submitted`]
//!   keys only off `submission_height` per normative ([CKP-005 acceptance](docs/requirements/domains/checkpoint/specs/CKP-005.md#acceptance-criteria)).

use dig_block::{Bytes32, Checkpoint, CheckpointSubmission, PublicKey, Signature, SignerBitmap};

fn byte_tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// Non-trivial [`Checkpoint`] so [`Checkpoint::hash`] is not a constant empty preimage.
fn sample_checkpoint() -> Checkpoint {
    let mut c = Checkpoint::new();
    c.epoch = 42;
    c.state_root = byte_tag(0x11);
    c.block_root = byte_tag(0x22);
    c.block_count = 100;
    c.tx_count = 1_000;
    c.total_fees = 99;
    c.prev_checkpoint = byte_tag(0x33);
    c.withdrawals_root = byte_tag(0x44);
    c.withdrawal_count = 5;
    c
}

fn submission_with_bitmap(bitmap: SignerBitmap) -> CheckpointSubmission {
    CheckpointSubmission::new(
        sample_checkpoint(),
        bitmap,
        Signature::default(),
        PublicKey::default(),
        0,
        0,
    )
}

/// **Test plan:** “Hash delegation” — submission hash is exactly inner checkpoint hash ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md#test-plan)).
#[test]
fn ckp005_hash_delegates_to_checkpoint() {
    let cp = sample_checkpoint();
    let sub = CheckpointSubmission::new(
        cp.clone(),
        SignerBitmap::new(4),
        Signature::default(),
        PublicKey::default(),
        0,
        0,
    );
    assert_eq!(sub.hash(), cp.hash());
    assert_eq!(sub.hash(), sub.checkpoint.hash());
}

/// **Test plan:** “Epoch delegation”.
#[test]
fn ckp005_epoch_delegates_to_checkpoint() {
    let mut cp = sample_checkpoint();
    cp.epoch = 7_777;
    let sub = CheckpointSubmission::new(
        cp.clone(),
        SignerBitmap::new(1),
        Signature::default(),
        PublicKey::default(),
        0,
        0,
    );
    assert_eq!(sub.epoch(), cp.epoch);
    assert_eq!(sub.epoch(), 7_777);
}

/// **Test plan:** “Signing percentage” — matches bitmap after setting bits (10 validators, 7 signed → 70%).
#[test]
fn ckp005_signing_percentage_delegates_to_bitmap() {
    let mut bitmap = SignerBitmap::new(10);
    for i in 0..7 {
        bitmap.set_signed(i).expect("valid index");
    }
    let sub = submission_with_bitmap(bitmap.clone());
    assert_eq!(sub.signing_percentage(), bitmap.signing_percentage());
    assert_eq!(sub.signing_percentage(), 70);
}

/// **Test plan:** “Meets threshold true” — 70% ≥ 67%.
#[test]
fn ckp005_meets_threshold_true_when_above() {
    let mut bitmap = SignerBitmap::new(10);
    for i in 0..7 {
        bitmap.set_signed(i).expect("valid index");
    }
    let sub = submission_with_bitmap(bitmap);
    assert!(sub.meets_threshold(67));
}

/// **Test plan:** “Meets threshold false” — 50% < 67%.
#[test]
fn ckp005_meets_threshold_false_when_below() {
    let mut bitmap = SignerBitmap::new(10);
    for i in 0..5 {
        bitmap.set_signed(i).expect("valid index");
    }
    let sub = submission_with_bitmap(bitmap);
    assert!(!sub.meets_threshold(67));
}

/// **Test plan:** “Not submitted initially”.
#[test]
fn ckp005_is_submitted_false_before_record() {
    let sub = submission_with_bitmap(SignerBitmap::new(3));
    assert!(!sub.is_submitted());
}

/// **Test plan:** “Record and check submitted” + “Record submission values”.
#[test]
fn ckp005_record_submission_sets_height_coin_and_is_submitted() {
    let mut sub = submission_with_bitmap(SignerBitmap::new(2));
    let coin = byte_tag(0xab);
    sub.record_submission(100, coin);
    assert!(sub.is_submitted());
    assert_eq!(sub.submission_height, Some(100));
    assert_eq!(sub.submission_coin, Some(coin));
}
