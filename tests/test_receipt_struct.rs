//! RCP-002: [`Receipt`] — seven public fields per NORMATIVE, construction, field round-trips.
//!
//! **Normative:** `docs/requirements/domains/receipt/NORMATIVE.md` (RCP-002)  
//! **Spec + test plan:** `docs/requirements/domains/receipt/specs/RCP-002.md`  
//! **Verification:** `docs/requirements/domains/receipt/VERIFICATION.md` (RCP-002)
//!
//! Tests align with the RCP-002 **Test Plan** table: construct with known values, read each field back, and
//! exercise [`ReceiptStatus`] on the same struct shape.

use dig_block::{Bytes32, Receipt, ReceiptStatus};

fn byte_tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

fn sample_receipt_with_status(status: ReceiptStatus) -> Receipt {
    Receipt::new(byte_tag(0x11), 42, 3, status, 5_000, byte_tag(0x22), 12_500)
}

/// **Test plan:** “Construct receipt” — [`Receipt::new`] exposes all seven fields for read access.
#[test]
fn test_receipt_construct_all_fields_accessible() {
    let r = sample_receipt_with_status(ReceiptStatus::Success);
    assert_eq!(r.tx_id, byte_tag(0x11));
    assert_eq!(r.block_height, 42);
    assert_eq!(r.tx_index, 3);
    assert_eq!(r.status, ReceiptStatus::Success);
    assert_eq!(r.fee_charged, 5_000);
    assert_eq!(r.post_state_root, byte_tag(0x22));
    assert_eq!(r.cumulative_fees, 12_500);
}

/// **Test plan:** “Verify tx_id” — store and read [`Bytes32`] identity.
#[test]
fn test_receipt_tx_id_roundtrip() {
    let id = byte_tag(0x7e);
    let r = Receipt::new(id, 0, 0, ReceiptStatus::Failed, 0, Bytes32::default(), 0);
    assert_eq!(r.tx_id, id);
}

/// **Test plan:** “Verify block_height”.
#[test]
fn test_receipt_block_height_roundtrip() {
    let r = Receipt::new(
        Bytes32::default(),
        9_876_543_210,
        0,
        ReceiptStatus::Success,
        0,
        Bytes32::default(),
        0,
    );
    assert_eq!(r.block_height, 9_876_543_210);
}

/// **Test plan:** “Verify tx_index” (zero-based position in block).
#[test]
fn test_receipt_tx_index_roundtrip() {
    let r = Receipt::new(
        Bytes32::default(),
        0,
        99,
        ReceiptStatus::Success,
        0,
        Bytes32::default(),
        0,
    );
    assert_eq!(r.tx_index, 99);
}

/// **Test plan:** “Verify status” — each [`ReceiptStatus`] variant can be stored (RCP-001 linkage).
#[test]
fn test_receipt_status_each_variant() {
    for status in [
        ReceiptStatus::Success,
        ReceiptStatus::InsufficientBalance,
        ReceiptStatus::InvalidNonce,
        ReceiptStatus::InvalidSignature,
        ReceiptStatus::AccountNotFound,
        ReceiptStatus::Failed,
    ] {
        let r = sample_receipt_with_status(status);
        assert_eq!(r.status, status);
    }
}

/// **Test plan:** “Verify fee_charged”.
#[test]
fn test_receipt_fee_charged_roundtrip() {
    let r = Receipt::new(
        Bytes32::default(),
        1,
        0,
        ReceiptStatus::Success,
        1_234_567_890,
        Bytes32::default(),
        0,
    );
    assert_eq!(r.fee_charged, 1_234_567_890);
}

/// **Test plan:** “Verify post_state_root”.
#[test]
fn test_receipt_post_state_root_roundtrip() {
    let root = byte_tag(0x55);
    let r = Receipt::new(Bytes32::default(), 0, 0, ReceiptStatus::Success, 0, root, 0);
    assert_eq!(r.post_state_root, root);
}

/// **Test plan:** “Verify cumulative_fees” — running total field is stored verbatim (RCP-002: monotonic sum enforced by producer).
#[test]
fn test_receipt_cumulative_fees_roundtrip() {
    let r = Receipt::new(
        Bytes32::default(),
        100,
        2,
        ReceiptStatus::Success,
        100,
        Bytes32::default(),
        350,
    );
    assert_eq!(r.cumulative_fees, 350);
}
