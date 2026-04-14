//! ERR-005 (SignerBitmap half): [`dig_block::SignerBitmapError`] — structured failures for bitmap bounds, wire shape, and validator sets.
//!
//! **Normative:** `docs/requirements/domains/error_types/NORMATIVE.md` (ERR-005)  
//! **Spec + test plan:** `docs/requirements/domains/error_types/specs/ERR-005.md`  
//! **Implementation:** `src/error.rs`  
//! **Crate spec:** [SPEC §4.3](docs/resources/SPEC.md)  
//! **Runtime producers:** [`dig_block::SignerBitmap::set_signed`], [`dig_block::SignerBitmap::merge`] (ATT-004 / ATT-005); future
//! deserializers may use [`SignerBitmapError::InvalidLength`] / [`SignerBitmapError::TooManyValidators`].
//!
//! ## How these tests prove ERR-005
//!
//! - **Surface area:** Each variant is constructed here; renames or field type changes break compilation
//!   ([acceptance criteria](docs/requirements/domains/error_types/specs/ERR-005.md#acceptance-criteria)).
//! - **Diagnostics:** `thiserror` `#[error]` templates must match the spec so operators and tests see stable `Display` output.
//! - **Error trait:** [`std::error::Error`] is required for `?` and tracing; we call `.source()` like the ERR-005 test plan.
//! - **Separation:** This file is dedicated to ERR-005; ATT-004/ATT-005 integration tests live in `test_signer_bitmap_*.rs` and assert
//!   the same variants with concrete `index`/`max`/`expected`/`got` payloads after real API calls.

use dig_block::SignerBitmapError;
use std::error::Error as StdError;

/// **Test plan:** `test_index_out_of_bounds` — mirrors [`SignerBitmapError::IndexOutOfBounds`] shape used when `set_signed` rejects `index >= validator_count`.
#[test]
fn err005_signer_index_out_of_bounds_display() {
    let e = SignerBitmapError::IndexOutOfBounds {
        index: 100,
        max: 64,
    };
    let s = e.to_string();
    assert!(s.contains("100"), "{}", s);
    assert!(s.contains("64"), "{}", s);
    assert!(s.to_lowercase().contains("bounds"), "{}", s);
}

/// **Test plan:** `test_too_many_validators` — creation or decode path rejects cardinality above policy ([`dig_block::MAX_VALIDATORS`](dig_block::MAX_VALIDATORS) is the protocol cap).
#[test]
fn err005_signer_too_many_validators_display() {
    let e = SignerBitmapError::TooManyValidators(1000);
    let s = e.to_string();
    assert!(s.contains("too many validators"), "{}", s);
    assert!(s.contains("1000"), "{}", s);
}

/// **Test plan:** `test_invalid_length` — serialized byte slice length disagrees with implied validator count.
#[test]
fn err005_signer_invalid_length_display() {
    let e = SignerBitmapError::InvalidLength {
        expected: 8,
        got: 4,
    };
    let s = e.to_string();
    assert!(s.contains('8'), "{}", s);
    assert!(s.contains('4'), "{}", s);
    assert!(s.to_lowercase().contains("length"), "{}", s);
}

/// **Test plan:** `test_validator_count_mismatch` — [`dig_block::SignerBitmap::merge`] when `validator_count` differs ([ATT-005](docs/requirements/domains/attestation/specs/ATT-005.md)).
#[test]
fn err005_signer_validator_count_mismatch_display() {
    let e = SignerBitmapError::ValidatorCountMismatch {
        expected: 64,
        got: 32,
    };
    let s = e.to_string();
    assert!(s.contains("64"), "{}", s);
    assert!(s.contains("32"), "{}", s);
    assert!(s.to_lowercase().contains("mismatch"), "{}", s);
}

/// **Test plan:** `test_signer_bitmap_error_trait` — `thiserror::Error` wiring for `dyn Error` + `.source()`.
#[test]
fn err005_signer_bitmap_implements_std_error() {
    let e: SignerBitmapError = SignerBitmapError::TooManyValidators(1);
    let _: &dyn StdError = &e;
    let _ = e.source();
}

/// **Acceptance:** `Clone` is required by ERR-005; duplicates errors across tasks without reallocating strings.
#[test]
fn err005_signer_bitmap_error_is_clone() {
    let e = SignerBitmapError::InvalidLength {
        expected: 1,
        got: 2,
    };
    assert_eq!(e.to_string(), e.clone().to_string());
}
