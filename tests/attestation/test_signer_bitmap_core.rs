//! ATT-004: [`SignerBitmap`] core API — allocation, bit access, counts, thresholds, raw bytes.
//!
//! **Normative:** `docs/requirements/domains/attestation/NORMATIVE.md` (ATT-004)  
//! **Spec + test plan table:** `docs/requirements/domains/attestation/specs/ATT-004.md`  
//! **Verification row:** `docs/requirements/domains/attestation/VERIFICATION.md` (ATT-004)
//!
//! Each `#[test]` maps to a row in the ATT-004 **Test Plan** (or directly to an acceptance bullet). Comments
//! state the obligation: what behavior is required and how the assertion proves it.

use dig_block::{SignerBitmap, SignerBitmapError, MAX_VALIDATORS};

/// **Test plan:** “New bitmap is empty” — `new(n)` must start with zero signers.
#[test]
fn test_new_bitmap_signer_count_zero() {
    let bm = SignerBitmap::new(10);
    assert_eq!(bm.signer_count(), 0);
    assert_eq!(bm.signing_percentage(), 0);
}

/// **Test plan:** “Correct byte allocation” — `ceil(10/8) == 2` bytes for 10 validators.
#[test]
fn test_new_correct_byte_length_for_10_validators() {
    let bm = SignerBitmap::new(10);
    assert_eq!(bm.as_bytes().len(), 2);
}

/// **Test plan:** “Set and check single signer” — `set_signed(3)` then `has_signed(3)`.
#[test]
fn test_set_and_check_single_signer() {
    let mut bm = SignerBitmap::new(10);
    assert!(!bm.has_signed(3));
    bm.set_signed(3).unwrap();
    assert!(bm.has_signed(3));
}

/// **Test plan:** “Check unsigned index” — `has_signed(5)` on empty bitmap (10 validators) is `false`.
#[test]
fn test_has_signed_false_when_bit_clear() {
    let bm = SignerBitmap::new(10);
    assert!(!bm.has_signed(5));
}

/// **Test plan:** “Out of bounds set_signed” — `index == validator_count` must error (NORMATIVE / ATT-004).
#[test]
fn test_set_signed_rejects_index_equal_to_validator_count() {
    let mut bm = SignerBitmap::new(10);
    let err = bm.set_signed(10).unwrap_err();
    assert_eq!(err, SignerBitmapError::IndexOutOfBounds);
}

/// **Test plan:** “Signer count accuracy” — five distinct indices raised → popcount 5.
#[test]
fn test_signer_count_five_distinct_indices() {
    let mut bm = SignerBitmap::new(32);
    for i in [0_u32, 7, 8, 31, 16] {
        bm.set_signed(i).unwrap();
    }
    assert_eq!(bm.signer_count(), 5);
}

/// **Test plan:** “Percentage computation” — 3 of 10 → integer `30` (`300/10`).
#[test]
fn test_signing_percentage_three_of_ten() {
    let mut bm = SignerBitmap::new(10);
    for i in [0_u32, 1, 2] {
        bm.set_signed(i).unwrap();
    }
    assert_eq!(bm.signing_percentage(), 30);
}

/// **Test plan:** “Percentage with zero validators” — avoid division by zero; defined as `0`.
#[test]
fn test_signing_percentage_zero_validators() {
    let bm = SignerBitmap::new(0);
    assert_eq!(bm.validator_count(), 0);
    assert_eq!(bm.signing_percentage(), 0);
    assert_eq!(bm.signer_count(), 0);
}

/// **Test plan:** “Threshold met” — ≥67% of 10 validators means at least 7 signers (`70 >= 67`).
#[test]
fn test_has_threshold_met_at_67_percent() {
    let mut bm = SignerBitmap::new(10);
    for i in 0..7 {
        bm.set_signed(i).unwrap();
    }
    assert_eq!(bm.signing_percentage(), 70);
    assert!(bm.has_threshold(67));
}

/// **Test plan:** “Threshold not met” — 50% &lt; 67%.
#[test]
fn test_has_threshold_not_met_at_50_percent() {
    let mut bm = SignerBitmap::new(10);
    for i in 0..5 {
        bm.set_signed(i).unwrap();
    }
    assert_eq!(bm.signing_percentage(), 50);
    assert!(!bm.has_threshold(67));
}

/// **Test plan:** “from_bytes round-trip” — structural equality after `as_bytes` / `from_bytes`.
#[test]
fn test_from_bytes_round_trip_equivalent() {
    let mut original = SignerBitmap::new(24);
    for i in [0_u32, 3, 11, 23] {
        original.set_signed(i).unwrap();
    }
    let restored = SignerBitmap::from_bytes(original.as_bytes(), original.validator_count());
    assert_eq!(restored, original);
}

/// **Test plan:** “MAX_VALIDATORS boundary” — 65536 validators → `8192` bytes, no error.
#[test]
fn test_max_validators_boundary_allocation() {
    let bm = SignerBitmap::new(MAX_VALIDATORS);
    assert_eq!(bm.validator_count(), MAX_VALIDATORS);
    assert_eq!(bm.as_bytes().len(), 8192);
    assert_eq!(bm.signer_count(), 0);
}

/// **Acceptance / invariants:** `as_bytes` borrows internal storage; lengths stay consistent after `new`.
#[test]
fn test_as_bytes_slice_matches_internal_length() {
    let bm = SignerBitmap::new(9);
    assert_eq!(bm.as_bytes().len(), 2);
    let ptr = bm.as_bytes().as_ptr();
    let bm2 = bm;
    assert_eq!(bm2.as_bytes().as_ptr(), ptr);
}
