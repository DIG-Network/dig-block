//! CKP-002: [`dig_block::CheckpointSubmission`] — checkpoint + attestation + L1 tracking fields per NORMATIVE.
//!
//! **Normative:** `docs/requirements/domains/checkpoint/NORMATIVE.md` (CKP-002)  
//! **Spec + test plan:** `docs/requirements/domains/checkpoint/specs/CKP-002.md`  
//! **Implementation:** `src/types/checkpoint.rs`
//!
//! ## How these tests prove CKP-002
//!
//! - **Constructor wiring:** [`CheckpointSubmission::new`] must move in the six caller-supplied values and set
//!   `submission_height` / `submission_coin` to [`None`] until [`record_submission`](docs/requirements/domains/checkpoint/specs/CKP-005.md) (CKP-005).
//! - **Eight-field surface:** [`read_all_eight_fields`] destructures every `pub` field; a missing or renamed field breaks compilation ([CKP-002 acceptance](docs/requirements/domains/checkpoint/specs/CKP-002.md#acceptance-criteria)).
//! - **Types:** [`SignerBitmap`] (ATT-004), [`dig_block::Signature`] / [`dig_block::PublicKey`] (chia-bls re-exports, STR-003 / BLK-006), and nested [`Checkpoint`] (CKP-001) match the normative table.
//! - **Bincode:** SER-001 expects wire types to remain serde-stable; a round-trip catches field-order or type drift early.

use dig_block::{Bytes32, Checkpoint, CheckpointSubmission, PublicKey, Signature, SignerBitmap};

fn byte_tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// Sample checkpoint with non-default values so “stored correctly” is not vacuously true for [`Checkpoint::new`].
fn sample_checkpoint() -> Checkpoint {
    let mut c = Checkpoint::new();
    c.epoch = 99;
    c.state_root = byte_tag(0x01);
    c.block_root = byte_tag(0x02);
    c.block_count = 10;
    c.tx_count = 1_000;
    c.total_fees = 50;
    c.prev_checkpoint = byte_tag(0x03);
    c.withdrawals_root = byte_tag(0x04);
    c.withdrawal_count = 4;
    c
}

/// Compile-time proof that [`CheckpointSubmission`] exposes exactly the eight normative fields.
#[must_use]
fn read_all_eight_fields(
    s: &CheckpointSubmission,
) -> (
    &Checkpoint,
    &SignerBitmap,
    &Signature,
    &PublicKey,
    &u64,
    &u32,
    &Option<u32>,
    &Option<Bytes32>,
) {
    (
        &s.checkpoint,
        &s.signer_bitmap,
        &s.aggregate_signature,
        &s.aggregate_pubkey,
        &s.score,
        &s.submitter,
        &s.submission_height,
        &s.submission_coin,
    )
}

/// **Test plan:** “Construct with valid inputs” — each argument is retrievable from the struct ([CKP-002 § Test Plan](docs/requirements/domains/checkpoint/specs/CKP-002.md#test-plan)).
#[test]
fn ckp002_new_stores_checkpoint_bitmap_sig_pubkey() {
    let cp = sample_checkpoint();
    let bitmap = SignerBitmap::new(16);
    let agg_sig = Signature::default();
    let agg_pk = PublicKey::default();

    let sub = CheckpointSubmission::new(cp.clone(), bitmap.clone(), agg_sig, agg_pk, 12_345, 7);

    assert_eq!(sub.checkpoint, cp);
    assert_eq!(sub.signer_bitmap, bitmap);
    assert_eq!(sub.aggregate_signature, Signature::default());
    assert_eq!(sub.aggregate_pubkey, PublicKey::default());
    assert_eq!(sub.score, 12_345);
    assert_eq!(sub.submitter, 7);
}

/// **Test plan:** “No L1 submission on construction” — both optional L1 slots start empty ([CKP-002 implementation notes](docs/requirements/domains/checkpoint/specs/CKP-002.md#implementation-notes)).
#[test]
fn ckp002_new_sets_submission_height_and_coin_none() {
    let sub = CheckpointSubmission::new(
        Checkpoint::new(),
        SignerBitmap::new(8),
        Signature::default(),
        PublicKey::default(),
        0,
        0,
    );
    assert!(sub.submission_height.is_none());
    assert!(sub.submission_coin.is_none());
}

/// **Test plan:** “Submitter stored” / “Score stored” — scalar identity fields round-trip through `new`.
#[test]
fn ckp002_new_stores_score_and_submitter() {
    let score = 0xfeed_face_dead_beef;
    let submitter: u32 = 42;
    let sub = CheckpointSubmission::new(
        Checkpoint::new(),
        SignerBitmap::new(4),
        Signature::default(),
        PublicKey::default(),
        score,
        submitter,
    );
    assert_eq!(sub.score, score);
    assert_eq!(sub.submitter, submitter);
}

/// Tuple destructure touches all eight fields with default construction values.
#[test]
fn ckp002_eight_field_surface_readable() {
    let sub = CheckpointSubmission::new(
        sample_checkpoint(),
        SignerBitmap::new(2),
        Signature::default(),
        PublicKey::default(),
        1,
        1,
    );
    let (checkpoint, bitmap, sig, pk, score, submitter, height, coin) = read_all_eight_fields(&sub);
    assert_eq!(*checkpoint, sample_checkpoint());
    assert_eq!(*bitmap, SignerBitmap::new(2));
    assert_eq!(*sig, Signature::default());
    assert_eq!(*pk, PublicKey::default());
    assert_eq!(*score, 1);
    assert_eq!(*submitter, 1);
    assert_eq!(*height, None);
    assert_eq!(*coin, None);
}

/// SER-001 / bincode smoke: encode and decode must reproduce the same logical submission (including `None` L1 fields).
#[test]
fn ckp002_bincode_roundtrip() {
    let sub = CheckpointSubmission::new(
        sample_checkpoint(),
        SignerBitmap::new(5),
        Signature::default(),
        PublicKey::default(),
        999,
        3,
    );
    let bytes = bincode::serialize(&sub).expect("serialize");
    let back: CheckpointSubmission = bincode::deserialize(&bytes).expect("deserialize");
    assert_eq!(back.checkpoint, sub.checkpoint);
    assert_eq!(back.signer_bitmap, sub.signer_bitmap);
    assert_eq!(back.aggregate_signature, sub.aggregate_signature);
    assert_eq!(back.aggregate_pubkey, sub.aggregate_pubkey);
    assert_eq!(back.score, sub.score);
    assert_eq!(back.submitter, sub.submitter);
    assert_eq!(back.submission_height, None);
    assert_eq!(back.submission_coin, None);
}
