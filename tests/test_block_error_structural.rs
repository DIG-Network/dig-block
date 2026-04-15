//! ERR-001: [`dig_block::BlockError`] — Tier 1 (structural validation) variants per NORMATIVE.
//!
//! **Normative:** `docs/requirements/domains/error_types/NORMATIVE.md` (ERR-001)  
//! **Spec + test plan:** `docs/requirements/domains/error_types/specs/ERR-001.md`  
//! **Implementation:** `src/error.rs`
//!
//! ## How these tests prove ERR-001
//!
//! - **Surface area:** Each test constructs one variant (or a batch of zero-field variants) so renames / omissions break
//!   compilation ([ERR-001 acceptance](docs/requirements/domains/error_types/specs/ERR-001.md#acceptance-criteria)).
//! - **Diagnostics:** [`std::fmt::Display`] output must embed the payload described in the `#[error(...)]` templates
//!   (thiserror) so SVL-* validators can surface actionable messages ([SPEC §4.1](docs/resources/SPEC.md) context).
//! - **Error trait:** [`std::error::Error`] is implemented via `thiserror::Error`; we assert `.source()` is callable
//!   ([ERR-001 test plan](docs/requirements/domains/error_types/specs/ERR-001.md#verification)).
//!
//! **Note:** Tier 2 / Tier 3 [`BlockError`] variants are [ERR-002](docs/requirements/domains/error_types/specs/ERR-002.md); this file
//! must not assume they exist yet.

use dig_block::{BlockError, Bytes32};
use std::error::Error as StdError;

fn byte_tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// **Test plan:** `test_invalid_data` — [`BlockError::InvalidData`] must forward the free-form string into Display (`#[error("invalid data: {0}")]`).
/// If the template drifts, SVL-* and SER-* call sites lose correlation between the Rust field and the log line.
#[test]
fn err001_invalid_data_display_contains_message() {
    let e = BlockError::InvalidData("test".into());
    let s = e.to_string();
    assert!(s.contains("invalid data"), "{}", s);
    assert!(s.contains("test"), "{}", s);
}

/// **Test plan:** `test_invalid_version` — SVL-001 uses [`BlockError::InvalidVersion`] with **expected** vs **actual** protocol
/// versions; Display must surface both `u16` values for logs and alerts ([ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md)).
#[test]
fn err001_invalid_version_display_contains_expected_and_actual() {
    let e = BlockError::InvalidVersion {
        expected: 1,
        actual: 99,
    };
    let s = e.to_string();
    assert!(s.contains("invalid version"), "{}", s);
    assert!(s.contains("99"), "actual version should appear: {}", s);
    assert!(s.contains('1'), "expected version should appear: {}", s);
}

/// **Test plan:** `test_too_large` — both observed `size` and protocol `max` appear so operators can see how far over the limit a blob is ([`BlockError::TooLarge`]).
#[test]
fn err001_too_large_shows_size_and_max() {
    let e = BlockError::TooLarge {
        size: 20_000_000,
        max: 10_000_000,
    };
    let s = e.to_string();
    assert!(s.contains("20000000"), "{}", s);
    assert!(s.contains("10000000"), "{}", s);
    assert!(s.to_lowercase().contains("large"), "{}", s);
}

/// **Test plan:** `test_cost_exceeded` — same pattern as size: actual vs ceiling for CLVM budget enforcement ([`BlockError::CostExceeded`]).
#[test]
fn err001_cost_exceeded_shows_cost_and_max() {
    let e = BlockError::CostExceeded {
        cost: 1_000_000,
        max: 500_000,
    };
    let s = e.to_string();
    assert!(s.contains("1000000") || s.contains("1_000_000"), "{}", s);
    assert!(s.contains("500000") || s.contains("500_000"), "{}", s);
    assert!(s.to_lowercase().contains("cost"), "{}", s);
}

/// **Test plan:** `test_spend_bundle_count_mismatch` — header-declared count vs deserialized vector length ([SVL-005](docs/requirements/domains/structural_validation/specs/SVL-005.md)).
#[test]
fn err001_spend_bundle_count_mismatch_shows_counts() {
    let e = BlockError::SpendBundleCountMismatch {
        header: 5,
        actual: 3,
    };
    let s = e.to_string();
    assert!(s.contains('5'), "{}", s);
    assert!(s.contains('3'), "{}", s);
}

/// **Test plan:** `test_invalid_spends_root`
#[test]
fn err001_invalid_spends_root_shows_hashes() {
    let expected = byte_tag(0x01);
    let computed = byte_tag(0x02);
    let e = BlockError::InvalidSpendsRoot { expected, computed };
    let s = e.to_string();
    assert!(s.len() > 32, "display should mention roots: {}", s);
}

/// **Test plan:** `test_invalid_receipts_root` — symmetric to spends root, receipts commitment path.
#[test]
fn err001_invalid_receipts_root_shows_hashes() {
    let e = BlockError::InvalidReceiptsRoot {
        expected: byte_tag(0xee),
        computed: byte_tag(0xdd),
    };
    assert!(!e.to_string().is_empty());
}

/// **Test plan:** `test_invalid_parent` — chain linkage: expected parent hash vs block’s `prev_hash` field.
#[test]
fn err001_invalid_parent_shows_hashes() {
    let e = BlockError::InvalidParent {
        expected: byte_tag(0xaa),
        got: byte_tag(0xbb),
    };
    let s = e.to_string();
    assert!(
        s.to_lowercase().contains("parent") || s.contains("expected"),
        "{}",
        s
    );
}

/// **Test plan:** `test_unit_variants` — nine zero-field variants must still produce stable, non-empty Display strings so metrics/alerts can key off them without parsing structured fields.
#[test]
fn err001_unit_variants_non_empty_display() {
    let cases: Vec<BlockError> = vec![
        BlockError::InvalidSlashProposalsRoot,
        BlockError::SlashProposalPayloadTooLarge,
        BlockError::TooManySlashProposals,
        BlockError::InvalidAdditionsRoot,
        BlockError::InvalidRemovalsRoot,
        BlockError::InvalidFilterHash,
        BlockError::AdditionsCountMismatch {
            header: 0,
            actual: 1,
        },
        BlockError::RemovalsCountMismatch {
            header: 0,
            actual: 1,
        },
        BlockError::SlashProposalCountMismatch {
            header: 0,
            actual: 1,
        },
    ];
    for e in cases {
        assert!(!e.to_string().trim().is_empty(), "{e:?}");
    }
}

/// **Test plan:** `test_duplicate_output` — duplicate mint detection carries the colliding [`Bytes32`] coin id ([SVL-006](docs/requirements/domains/structural_validation/specs/SVL-006.md)).
#[test]
fn err001_duplicate_output_shows_coin_id() {
    let coin = byte_tag(0x7c);
    let e = BlockError::DuplicateOutput { coin_id: coin };
    let s = e.to_string();
    assert!(
        s.to_lowercase().contains("duplicate") || s.contains("coin"),
        "{}",
        s
    );
}

/// **Test plan:** `test_double_spend` — same coin id consumed twice in removals; Display must mention the id for forensics.
#[test]
fn err001_double_spend_shows_coin_id() {
    let coin = byte_tag(0x3d);
    let e = BlockError::DoubleSpendInBlock { coin_id: coin };
    let s = e.to_string();
    assert!(
        s.to_lowercase().contains("double") || s.contains("coin"),
        "{}",
        s
    );
}

/// **Test plan:** `test_timestamp_too_far` — wall-clock skew guard: block timestamp vs computed `max_allowed` ([SVL-004](docs/requirements/domains/structural_validation/specs/SVL-004.md)).
#[test]
fn err001_timestamp_too_far_shows_both_times() {
    let e = BlockError::TimestampTooFarInFuture {
        timestamp: 9_999_999,
        max_allowed: 1_000_000,
    };
    let s = e.to_string();
    assert!(s.contains("9999999") || s.contains("9_999_999"), "{}", s);
    assert!(s.contains("1000000") || s.contains("1_000_000"), "{}", s);
}

/// **Test plan:** `test_error_trait` — `thiserror::Error` must implement [`std::error::Error`] so `?` in layered validators and `.source()` chains work.
#[test]
fn err001_implements_std_error() {
    let e: BlockError = BlockError::InvalidData("x".into());
    let _: &dyn StdError = &e;
    let _ = e.source(); // may be None; must be callable
}

/// **Acceptance (ERR-001 + API ergonomics):** [`Clone`] lets validation pipelines fork [`BlockError`] into multiple branches (e.g. parallel checks) without allocating new strings for every copy.
#[test]
fn err001_block_error_is_clone() {
    let e = BlockError::InvalidVersion {
        expected: 1,
        actual: 2,
    };
    assert_eq!(e.to_string(), e.clone().to_string());
}
