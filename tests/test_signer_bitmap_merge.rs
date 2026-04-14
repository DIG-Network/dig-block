//! ATT-005: [`SignerBitmap::merge`] (bitwise OR) and [`SignerBitmap::signer_indices`] (sorted enumeration).
//!
//! **Normative:** `docs/requirements/domains/attestation/NORMATIVE.md` (ATT-005)  
//! **Spec + test plan:** `docs/requirements/domains/attestation/specs/ATT-005.md`  
//! **Verification:** `docs/requirements/domains/attestation/VERIFICATION.md` (ATT-005)
//!
//! Comments tie each test to the ATT-005 **Test Plan** table and acceptance bullets: merge semantics,
//! mismatch errors, ascending `signer_indices`, and post-merge union behavior.

use dig_block::{SignerBitmap, SignerBitmapError};

/// **Test plan:** “Merge disjoint bitmaps” — A `{0,2}`, B `{1,3}` → merge B into A → union `{0,1,2,3}`.
#[test]
fn test_merge_disjoint_bitmaps_union() {
    let mut a = SignerBitmap::new(4);
    a.set_signed(0).unwrap();
    a.set_signed(2).unwrap();
    let mut b = SignerBitmap::new(4);
    b.set_signed(1).unwrap();
    b.set_signed(3).unwrap();
    a.merge(&b).unwrap();
    assert_eq!(a.signer_indices(), vec![0, 1, 2, 3]);
    assert_eq!(a.signer_count(), 4);
}

/// **Test plan:** “Merge overlapping bitmaps” — A `{0,1}`, B `{1,2}` → `{0,1,2}` (OR, not add).
#[test]
fn test_merge_overlapping_bitmaps() {
    let mut a = SignerBitmap::new(4);
    a.set_signed(0).unwrap();
    a.set_signed(1).unwrap();
    let mut b = SignerBitmap::new(4);
    b.set_signed(1).unwrap();
    b.set_signed(2).unwrap();
    a.merge(&b).unwrap();
    assert_eq!(a.signer_indices(), vec![0, 1, 2]);
    assert_eq!(a.signer_count(), 3);
}

/// **Test plan:** “Merge with empty bitmap” — B contributes no bits; A unchanged as a set.
#[test]
fn test_merge_with_empty_other() {
    let mut a = SignerBitmap::new(8);
    a.set_signed(0).unwrap();
    a.set_signed(1).unwrap();
    let b = SignerBitmap::new(8);
    a.merge(&b).unwrap();
    assert_eq!(a.signer_indices(), vec![0, 1]);
}

/// **Test plan:** “Merge mismatched validator_count” — MUST error (NORMATIVE); neither bitmap is assumed valid to combine.
#[test]
fn test_merge_rejects_validator_count_mismatch() {
    let mut a = SignerBitmap::new(10);
    let b = SignerBitmap::new(20);
    let err = a.merge(&b).unwrap_err();
    assert_eq!(
        err,
        SignerBitmapError::ValidatorCountMismatch {
            expected: 10,
            got: 20
        }
    );
}

/// **Test plan:** “Signer indices empty bitmap” — no bits set → empty vec (not an error).
#[test]
fn test_signer_indices_empty_bitmap() {
    let bm = SignerBitmap::new(16);
    assert!(bm.signer_indices().is_empty());
}

/// **Test plan:** “Signer indices after set” — set order `5, 2, 8` but output MUST be ascending `[2, 5, 8]`.
#[test]
fn test_signer_indices_sorted_after_non_sequential_sets() {
    let mut bm = SignerBitmap::new(16);
    for i in [5_u32, 2, 8] {
        bm.set_signed(i).unwrap();
    }
    assert_eq!(bm.signer_indices(), vec![2, 5, 8]);
}

/// **Test plan:** “Signer indices after merge” — indices equal sorted set union of both operands’ signers.
#[test]
fn test_signer_indices_after_merge_matches_union() {
    let mut a = SignerBitmap::new(8);
    a.set_signed(1).unwrap();
    a.set_signed(4).unwrap();
    let mut b = SignerBitmap::new(8);
    b.set_signed(3).unwrap();
    b.set_signed(4).unwrap();
    a.merge(&b).unwrap();
    assert_eq!(a.signer_indices(), vec![1, 3, 4]);
}

/// **Acceptance:** `merge` succeeds when counts match (paired with mismatch test above).
#[test]
fn test_merge_ok_when_validator_counts_equal() {
    let mut a = SignerBitmap::new(5);
    let b = SignerBitmap::new(5);
    assert!(a.merge(&b).is_ok());
}
