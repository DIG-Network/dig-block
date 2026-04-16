//! ERR-005 (Receipt half): [`dig_block::ReceiptError`] — receipt parse and lookup failures.
//!
//! **Normative:** `docs/requirements/domains/error_types/NORMATIVE.md` (ERR-005)  
//! **Spec + test plan:** `docs/requirements/domains/error_types/specs/ERR-005.md`  
//! **Implementation:** `src/error.rs`  
//! **Crate spec:** [SPEC §4.4](docs/resources/SPEC.md)  
//! **Related domain:** [RCP NORMATIVE](docs/requirements/domains/receipt/NORMATIVE.md) — [`dig_block::Receipt`], [`dig_block::ReceiptList`].
//!
//! ## How these tests prove ERR-005
//!
//! - **Variant surface:** [`ReceiptError::InvalidData`] and [`ReceiptError::NotFound`] must exist with the spec field types (`String`, [`Bytes32`](dig_block::Bytes32)).
//! - **Diagnostics:** Display strings follow ERR-005 templates so indexer logs stay grep-friendly.
//! - **Error trait:** [`std::error::Error`] + `.source()` per verification table.
//! - **NotFound payload:** The hash must appear in `Display` so operators can correlate with Merkle keys / tx ids.

use dig_block::{Bytes32, ReceiptError};
use std::error::Error as StdError;

/// **Test plan:** `test_receipt_invalid_data` — bincode or field validation maps to free-form detail ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md) patterns).
#[test]
fn err005_receipt_invalid_data_display() {
    let e = ReceiptError::InvalidData("bad format".into());
    let s = e.to_string();
    assert!(s.contains("invalid receipt data"), "{}", s);
    assert!(s.contains("bad format"), "{}", s);
}

/// **Test plan:** `test_receipt_not_found` — lookup by receipt id / hash missed the map.
#[test]
fn err005_receipt_not_found_display_includes_hash() {
    let id = Bytes32::new([0x42; 32]);
    let e = ReceiptError::NotFound(id);
    let s = e.to_string();
    assert!(s.to_lowercase().contains("not found"), "{}", s);
    // Bytes32's Display is stable hex; prove the payload is not dropped.
    assert!(
        s.chars().filter(|c| c.is_ascii_hexdigit()).count() >= 32,
        "expected hash digits in display: {}",
        s
    );
}

/// **Test plan:** `test_receipt_error_trait`.
#[test]
fn err005_receipt_implements_std_error() {
    let e: ReceiptError = ReceiptError::InvalidData("x".into());
    let _: &dyn StdError = &e;
    let _ = e.source();
}

/// **Acceptance:** ERR-005 requires `Clone`; receipt pipelines may copy errors into worker channels.
#[test]
fn err005_receipt_error_is_clone() {
    let e = ReceiptError::NotFound(Bytes32::default());
    assert_eq!(e.to_string(), e.clone().to_string());
}
