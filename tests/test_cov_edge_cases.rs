//! Coverage-fill: genuinely-reachable own-logic edge branches not exercised by the
//! per-requirement suites.
//!
//! These are real behavioural edge cases — empty-list predicates, out-of-range /
//! short-tail bitmap reads, the lazy resize in [`SignerBitmap::set_signed`], the
//! `Default` impl for [`Checkpoint`], and bincode decode-error paths — each asserting
//! the documented contract rather than merely touching a line.
//!
//! Spec links: ATT-004 ([`SignerBitmap`]), RCP-004 ([`ReceiptList::is_empty`]),
//! CKP-001 ([`Checkpoint::default`]), SER-002 (bincode `from_bytes` decode errors).

use dig_block::{
    AttestedBlock, BlockError, Bytes32, Checkpoint, CheckpointError, Receipt, ReceiptList,
    ReceiptStatus, SignerBitmap,
};

// ---------------------------------------------------------------------------
// RCP-004 — ReceiptList::is_empty
// ---------------------------------------------------------------------------

#[test]
fn receipt_list_is_empty_true_when_no_receipts() {
    let list = ReceiptList::new();
    assert!(list.is_empty(), "fresh ReceiptList must be empty");
    assert_eq!(list.len(), 0);
}

#[test]
fn receipt_list_is_empty_false_after_push() {
    let mut list = ReceiptList::new();
    list.push(Receipt::new(
        Bytes32::new([0x11; 32]),
        100,
        0,
        ReceiptStatus::Success,
        50,
        Bytes32::new([0xcc; 32]),
        1_000,
    ));
    assert!(!list.is_empty(), "ReceiptList with one entry is not empty");
    assert_eq!(list.len(), 1);
}

// ---------------------------------------------------------------------------
// ATT-004 — SignerBitmap::has_signed boundary + short-tail behaviour
// ---------------------------------------------------------------------------

#[test]
fn signer_bitmap_has_signed_out_of_range_is_false() {
    // index >= validator_count returns false without panicking (signer_bitmap.rs:86).
    let bitmap = SignerBitmap::new(8);
    assert!(
        !bitmap.has_signed(8),
        "index == validator_count is out of range"
    );
    assert!(
        !bitmap.has_signed(100),
        "index far past the set is out of range"
    );
}

#[test]
fn signer_bitmap_has_signed_short_tail_reads_as_zero() {
    // A deserialized bitmap whose `bits` is shorter than ceil(validator_count/8)
    // reads missing bytes as zero rather than panicking (signer_bitmap.rs:91).
    // validator_count = 16 expects 2 bytes; supply only 1.
    let bitmap = SignerBitmap::from_bytes(&[0xFF], 16);
    // Index 0..8 live in the present byte and are set.
    assert!(bitmap.has_signed(0));
    assert!(bitmap.has_signed(7));
    // Index 8..16 fall in the absent second byte -> false.
    assert!(
        !bitmap.has_signed(8),
        "byte beyond the short tail must read as zero"
    );
    assert!(!bitmap.has_signed(15));
}

#[test]
fn signer_bitmap_set_signed_resizes_short_bits() {
    // set_signed on a bitmap whose `bits` is shorter than the canonical length must
    // grow `bits` to ceil(validator_count/8) before writing (signer_bitmap.rs:108).
    let mut bitmap = SignerBitmap::from_bytes(&[], 16); // 0 bytes, canonical = 2
    bitmap.set_signed(9).expect("index 9 < 16 is in range");
    assert!(
        bitmap.has_signed(9),
        "bit must be set after the lazy resize"
    );
    assert_eq!(
        bitmap.as_bytes().len(),
        2,
        "bits grew to canonical width (ceil(16/8))"
    );
    // Untouched bits stay zero.
    assert!(!bitmap.has_signed(0));
    assert!(!bitmap.has_signed(8));
}

// ---------------------------------------------------------------------------
// CKP-001 — Checkpoint::default delegates to Checkpoint::new
// ---------------------------------------------------------------------------

#[test]
fn checkpoint_default_equals_new() {
    assert_eq!(
        Checkpoint::default(),
        Checkpoint::new(),
        "Checkpoint::default() must delegate to Checkpoint::new()"
    );
}

// ---------------------------------------------------------------------------
// SER-002 — bincode decode-error mapping
// ---------------------------------------------------------------------------

#[test]
fn attested_block_from_bytes_rejects_garbage() {
    // Truncated / non-bincode bytes map to BlockError::InvalidData, never a panic.
    let err = AttestedBlock::from_bytes(&[0xDE, 0xAD]).expect_err("garbage must not decode");
    assert!(matches!(err, BlockError::InvalidData(_)));
}

#[test]
fn checkpoint_from_bytes_rejects_garbage() {
    let err = Checkpoint::from_bytes(&[0x00, 0x01, 0x02]).expect_err("garbage must not decode");
    assert!(matches!(err, CheckpointError::InvalidData(_)));
}

#[test]
fn checkpoint_roundtrips_through_bytes() {
    // Happy-path companion so the error test isn't the only exercise of the codec.
    let cp = Checkpoint::new();
    let bytes = cp.to_bytes();
    let decoded = Checkpoint::from_bytes(&bytes).expect("valid bincode must decode");
    assert_eq!(cp, decoded);
}
