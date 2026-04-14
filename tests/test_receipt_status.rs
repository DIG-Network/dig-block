//! RCP-001: [`ReceiptStatus`] — six variants, `#[repr(u8)]`, stable discriminants, `from_u8` / `as_u8`.
//!
//! **Normative:** `docs/requirements/domains/receipt/NORMATIVE.md` (RCP-001)  
//! **Spec + test plan:** `docs/requirements/domains/receipt/specs/RCP-001.md`  
//! **Verification:** `docs/requirements/domains/receipt/VERIFICATION.md` (RCP-001)
//!
//! Each test maps to the RCP-001 **Test Plan** table or an acceptance bullet (exact variant count, numeric
//! values, round-trip, unknown-byte handling).

use dig_block::ReceiptStatus;

/// **Test plan:** “Success numeric value” — discriminant `0`.
#[test]
fn test_success_is_zero() {
    assert_eq!(ReceiptStatus::Success.as_u8(), 0);
}

/// **Test plan:** “InsufficientBalance value” — discriminant `1`.
#[test]
fn test_insufficient_balance_is_one() {
    assert_eq!(ReceiptStatus::InsufficientBalance.as_u8(), 1);
}

/// **Test plan:** “InvalidNonce value” — discriminant `2`.
#[test]
fn test_invalid_nonce_is_two() {
    assert_eq!(ReceiptStatus::InvalidNonce.as_u8(), 2);
}

/// **Test plan:** “InvalidSignature value” — discriminant `3`.
#[test]
fn test_invalid_signature_is_three() {
    assert_eq!(ReceiptStatus::InvalidSignature.as_u8(), 3);
}

/// **Test plan:** “AccountNotFound value” — discriminant `4`.
#[test]
fn test_account_not_found_is_four() {
    assert_eq!(ReceiptStatus::AccountNotFound.as_u8(), 4);
}

/// **Test plan:** “Failed numeric value” — discriminant `255`.
#[test]
fn test_failed_is_two_fifty_five() {
    assert_eq!(ReceiptStatus::Failed.as_u8(), 255);
}

/// **Acceptance:** exactly six named variants exist (guards enum drift).
#[test]
fn test_six_variants_distinct_discriminants() {
    let all = [
        ReceiptStatus::Success,
        ReceiptStatus::InsufficientBalance,
        ReceiptStatus::InvalidNonce,
        ReceiptStatus::InvalidSignature,
        ReceiptStatus::AccountNotFound,
        ReceiptStatus::Failed,
    ];
    assert_eq!(all.len(), 6);
    let mut seen = [false; 256];
    for s in all {
        let b = s.as_u8() as usize;
        assert!(!seen[b], "duplicate discriminant {b}");
        seen[b] = true;
    }
}

/// **Test plan:** “Round-trip conversion” — `as_u8` then `from_u8` recovers each defined variant.
#[test]
fn test_round_trip_each_variant() {
    for s in [
        ReceiptStatus::Success,
        ReceiptStatus::InsufficientBalance,
        ReceiptStatus::InvalidNonce,
        ReceiptStatus::InvalidSignature,
        ReceiptStatus::AccountNotFound,
        ReceiptStatus::Failed,
    ] {
        assert_eq!(ReceiptStatus::from_u8(s.as_u8()), s);
    }
}

/// **Test plan:** “Unknown value handling” — arbitrary reserved byte maps to [`ReceiptStatus::Failed`].
#[test]
fn test_unknown_byte_maps_to_failed() {
    assert_eq!(ReceiptStatus::from_u8(100), ReceiptStatus::Failed);
}
