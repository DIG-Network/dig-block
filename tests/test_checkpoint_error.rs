//! ERR-003: [`dig_block::CheckpointError`] — checkpoint lifecycle errors per NORMATIVE.
//!
//! **Normative:** `docs/requirements/domains/error_types/NORMATIVE.md` (ERR-003)  
//! **Spec + test plan:** `docs/requirements/domains/error_types/specs/ERR-003.md`  
//! **Implementation:** `src/error.rs`  
//! **Crate spec:** [SPEC §4.2](docs/resources/SPEC.md)
//!
//! ## How these tests prove ERR-003
//!
//! - **Variant surface:** Each test constructs one [`CheckpointError`] variant; missing or renamed variants fail compilation
//!   ([acceptance criteria](docs/requirements/domains/error_types/specs/ERR-003.md#acceptance-criteria)).
//! - **Diagnostics:** [`std::fmt::Display`] must match the `#[error(...)]` templates (thiserror) so checkpoint validators and
//!   L1 bridge code can log actionable messages ([CKP domain](docs/requirements/domains/checkpoint/NORMATIVE.md) context).
//! - **Error trait:** [`std::error::Error`] via `thiserror::Error`; we assert `.source()` is callable
//!   ([ERR-003 verification](docs/requirements/domains/error_types/specs/ERR-003.md#verification)).
//!
//! **Separation:** [`CheckpointError`] is distinct from [`dig_block::BlockError`] — epochs, finalization, and submission scoring
//! follow a different state machine ([ERR-003 implementation notes](docs/requirements/domains/error_types/specs/ERR-003.md#implementation-notes)).

use dig_block::CheckpointError;
use std::error::Error as StdError;

/// **Test plan:** `test_invalid_data` — decode / field validation failures map to free-form text ([SER NORMATIVE](docs/requirements/domains/serialization/NORMATIVE.md) uses `InvalidData` for `from_bytes`).
#[test]
fn err003_invalid_data_display_contains_message() {
    let e = CheckpointError::InvalidData("bad field".into());
    let s = e.to_string();
    assert!(s.contains("invalid checkpoint data"), "{}", s);
    assert!(s.contains("bad field"), "{}", s);
}

/// **Test plan:** `test_not_found` — indexer or node has no checkpoint for the requested epoch.
#[test]
fn err003_not_found_display_contains_epoch() {
    let e = CheckpointError::NotFound(42);
    let s = e.to_string();
    assert!(s.contains("checkpoint not found"), "{}", s);
    assert!(s.contains("42"), "{}", s);
}

/// **Test plan:** `test_invalid` — semantic validation failed (e.g. hash mismatch) with human-readable detail.
#[test]
fn err003_invalid_display_contains_reason() {
    let e = CheckpointError::Invalid("hash mismatch".into());
    let s = e.to_string();
    assert!(s.contains("invalid checkpoint"), "{}", s);
    assert!(s.contains("hash mismatch"), "{}", s);
}

/// **Test plan:** `test_score_not_higher` — anti-regression: submitted score must exceed current winner ([ERR-003 notes](docs/requirements/domains/error_types/specs/ERR-003.md#implementation-notes)).
#[test]
fn err003_score_not_higher_shows_current_and_submitted() {
    let e = CheckpointError::ScoreNotHigher {
        current: 100,
        submitted: 50,
    };
    let s = e.to_string();
    assert!(s.contains("100"), "{}", s);
    assert!(s.contains("50"), "{}", s);
    assert!(s.to_lowercase().contains("score"), "{}", s);
}

/// **Test plan:** `test_epoch_mismatch` — submission targets wrong epoch ([ERR-003 notes](docs/requirements/domains/error_types/specs/ERR-003.md#implementation-notes)).
#[test]
fn err003_epoch_mismatch_shows_expected_and_got() {
    let e = CheckpointError::EpochMismatch {
        expected: 5,
        got: 3,
    };
    let s = e.to_string();
    assert!(s.contains('5'), "{}", s);
    assert!(s.contains('3'), "{}", s);
}

/// **Test plan:** `test_already_finalized` — L1-committed checkpoint must not accept further mutations.
#[test]
fn err003_already_finalized_non_empty_display() {
    let e = CheckpointError::AlreadyFinalized;
    let s = e.to_string();
    assert!(!s.trim().is_empty());
    assert!(s.to_lowercase().contains("finalized"), "{}", s);
}

/// **Test plan:** `test_not_started` — epoch collector or finalizer invoked before process start.
#[test]
fn err003_not_started_non_empty_display() {
    let e = CheckpointError::NotStarted;
    let s = e.to_string();
    assert!(!s.trim().is_empty());
    assert!(
        s.to_lowercase().contains("not started") || s.to_lowercase().contains("process"),
        "{}",
        s
    );
}

/// **Test plan:** `test_error_trait` — `?` and error chains in checkpoint pipelines.
#[test]
fn err003_implements_std_error() {
    let e: CheckpointError = CheckpointError::InvalidData("x".into());
    let _: &dyn StdError = &e;
    let _ = e.source();
}

/// **Acceptance:** [`Clone`] allows duplicating errors across async / parallel validation without string round-trips (parity with [`dig_block::BlockError`]).
#[test]
fn err003_checkpoint_error_is_clone() {
    let e = CheckpointError::NotFound(7);
    assert_eq!(e.to_string(), e.clone().to_string());
}
