//! ERR-004: [`dig_block::BuilderError`] — block production / [`dig_block::BlockBuilder`] construction errors.
//!
//! **Normative:** `docs/requirements/domains/error_types/NORMATIVE.md` (ERR-004)  
//! **Spec + test plan:** `docs/requirements/domains/error_types/specs/ERR-004.md`  
//! **Implementation:** `src/error.rs`  
//! **Crate spec:** [SPEC §6.5](docs/resources/SPEC.md)  
//! **Callers (future):** BLD-002, BLD-003, BLD-005, BLD-006 per [block_production NORMATIVE](docs/requirements/domains/block_production/NORMATIVE.md).
//!
//! ## How these tests prove ERR-004
//!
//! - **Variant surface:** Each test constructs one [`BuilderError`] variant; renames or missing fields fail at compile time
//!   ([acceptance criteria](docs/requirements/domains/error_types/specs/ERR-004.md#acceptance-criteria)).
//! - **Budget / payload fields:** `CostBudgetExceeded` and `SizeBudgetExceeded` expose `current`, `addition`, `max` with the
//!   spec’s `u64` / `u32` types so [`BlockBuilder::add_spend_bundle`](docs/resources/SPEC.md) can surface running totals vs limits.
//! - **Diagnostics:** [`std::fmt::Display`] must follow the `#[error(...)]` templates exactly so logs match the test plan and operator runbooks.
//! - **Error trait:** [`std::error::Error`] via `thiserror::Error`; `.source()` remains callable for `?` / tracing compatibility
//!   ([ERR-004 verification table](docs/requirements/domains/error_types/specs/ERR-004.md#verification)).
//! - **Clone:** Derived `Clone` matches the spec’s derive list and mirrors [`dig_block::CheckpointError`] ergonomics for async pipelines.

use dig_block::BuilderError;
use std::error::Error as StdError;

/// **Test plan:** `test_cost_budget_exceeded` — adding a spend would push cumulative CLVM cost past [`dig_block::MAX_COST_PER_BLOCK`](dig_block::MAX_COST_PER_BLOCK).
#[test]
fn err004_cost_budget_exceeded_display_shows_current_addition_max() {
    let e = BuilderError::CostBudgetExceeded {
        current: 500_000_000_000,
        addition: 100_000_000_000,
        max: 550_000_000_000,
    };
    let s = e.to_string();
    assert!(s.contains("500000000000"), "{}", s);
    assert!(s.contains("100000000000"), "{}", s);
    assert!(s.contains("550000000000"), "{}", s);
    assert!(
        s.to_lowercase().contains("cost") && s.contains("budget"),
        "{}",
        s
    );
}

/// **Test plan:** `test_size_budget_exceeded` — serialized block size would exceed [`dig_block::MAX_BLOCK_SIZE`](dig_block::MAX_BLOCK_SIZE).
#[test]
fn err004_size_budget_exceeded_display_shows_current_addition_max() {
    let e = BuilderError::SizeBudgetExceeded {
        current: 9_000_000,
        addition: 2_000_000,
        max: 10_000_000,
    };
    let s = e.to_string();
    assert!(s.contains("9000000"), "{}", s);
    assert!(s.contains("2000000"), "{}", s);
    assert!(s.contains("10000000"), "{}", s);
    assert!(
        s.to_lowercase().contains("size") && s.contains("budget"),
        "{}",
        s
    );
}

/// **Test plan:** `test_too_many_slash_proposals` — count would exceed protocol cap ([BLD-003](docs/requirements/domains/block_production/specs/BLD-003.md)).
#[test]
fn err004_too_many_slash_proposals_display_shows_max() {
    let e = BuilderError::TooManySlashProposals { max: 64 };
    let s = e.to_string();
    assert!(s.contains("64"), "{}", s);
    assert!(s.to_lowercase().contains("slash"), "{}", s);
}

/// **Test plan:** `test_slash_proposal_too_large` — single payload exceeds [`dig_block::MAX_SLASH_PROPOSAL_PAYLOAD_BYTES`](dig_block::MAX_SLASH_PROPOSAL_PAYLOAD_BYTES).
#[test]
fn err004_slash_proposal_too_large_display_shows_size_and_max() {
    let e = BuilderError::SlashProposalTooLarge {
        size: 100_000,
        max: 65_536,
    };
    let s = e.to_string();
    assert!(s.contains("100000"), "{}", s);
    assert!(s.contains("65536"), "{}", s);
}

/// **Test plan:** `test_signing_failed` — [`dig_block::BlockSigner`](dig_block::BlockSigner) returned an opaque failure string ([BLD-006](docs/requirements/domains/block_production/specs/BLD-006.md)).
#[test]
fn err004_signing_failed_display_includes_reason() {
    let e = BuilderError::SigningFailed("key not found".into());
    let s = e.to_string();
    assert!(s.contains("signing failed"), "{}", s);
    assert!(s.contains("key not found"), "{}", s);
}

/// **Test plan:** `test_empty_block` — `build()` must not emit a block with zero spend bundles ([ERR-004 implementation notes](docs/requirements/domains/error_types/specs/ERR-004.md#implementation-notes)).
#[test]
fn err004_empty_block_display_matches_spec() {
    let e = BuilderError::EmptyBlock;
    assert_eq!(
        e.to_string(),
        "empty block: no spend bundles added",
        "Display must match ERR-004 template for log stability"
    );
}

/// **Test plan:** `test_missing_dfsp_roots` — v2 header requires DFSP fields before finalize ([ERR-004 implementation notes](docs/requirements/domains/error_types/specs/ERR-004.md#implementation-notes)).
#[test]
fn err004_missing_dfsp_roots_display_matches_spec() {
    let e = BuilderError::MissingDfspRoots;
    assert_eq!(
        e.to_string(),
        "missing DFSP roots",
        "Display must match ERR-004 template for log stability"
    );
}

/// **Test plan:** `test_error_trait` — ensures `thiserror::Error` is wired so callers can use `dyn Error` and `.source()`.
#[test]
fn err004_implements_std_error() {
    let e: BuilderError = BuilderError::EmptyBlock;
    let _: &dyn StdError = &e;
    let _ = e.source();
}

/// **Acceptance:** `Clone` is required by ERR-004 derive list; duplicated errors can cross `.await` without reallocation.
#[test]
fn err004_builder_error_is_clone() {
    let e = BuilderError::MissingDfspRoots;
    assert_eq!(e.to_string(), e.clone().to_string());
}
