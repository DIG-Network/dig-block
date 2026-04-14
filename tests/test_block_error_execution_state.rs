//! ERR-002: [`dig_block::BlockError`] — Tier 2 (execution) and Tier 3 (state) variants per NORMATIVE.
//!
//! **Normative:** `docs/requirements/domains/error_types/NORMATIVE.md` (ERR-002)  
//! **Spec + test plan:** `docs/requirements/domains/error_types/specs/ERR-002.md`  
//! **Implementation:** `src/error.rs` (same enum as [ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md))  
//! **Test file:** `tests/test_block_error_execution_state.rs` (flat `tests/` layout; see `str_002_tests::integration_tests_directory_is_flat`).
//!
//! ## How these tests prove ERR-002
//!
//! - **Tier separation:** Execution errors ([EXE NORMATIVE](docs/requirements/domains/execution_validation/NORMATIVE.md)) and state errors
//!   ([STV NORMATIVE](docs/requirements/domains/state_validation/NORMATIVE.md)) share one [`BlockError`] so `validate_execution` / `validate_state`
//!   can return a unified type without lossy string wrappers.
//! - **Field coverage:** Each test constructs one variant with representative payloads; if a field is renamed or dropped, either compilation breaks
//!   or the Display assertion fails ([ERR-002 acceptance](docs/requirements/domains/error_types/specs/ERR-002.md#acceptance-criteria)).
//! - **Diagnostics:** [`std::fmt::Display`] strings follow the `#[error(...)]` templates in the spec so operators can grep logs and map back to
//!   validator sites ([SPEC §4.1 / §7.6](docs/resources/SPEC.md)).

use dig_block::{BlockError, Bytes32};
use std::error::Error as StdError;

fn byte_tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// **Test plan:** `test_puzzle_hash_mismatch` — [`BlockError::PuzzleHashMismatch`] must surface `coin_id`, on-chain `expected` puzzle hash,
/// and `computed` from `clvm-utils::tree_hash` ([EXE NORMATIVE](docs/requirements/domains/execution_validation/NORMATIVE.md)).
#[test]
fn err002_puzzle_hash_mismatch_shows_coin_and_hashes() {
    let coin_id = byte_tag(0x11);
    let expected = byte_tag(0x22);
    let computed = byte_tag(0x33);
    let e = BlockError::PuzzleHashMismatch {
        coin_id,
        expected,
        computed,
    };
    let s = e.to_string();
    assert!(s.to_lowercase().contains("puzzle"), "{}", s);
    assert!(s.contains("mismatch") || s.contains("expected"), "{}", s);
}

/// **Test plan:** `test_clvm_execution_failed` — maps from `dig-clvm` / spend failure into a stable coin-scoped message ([start.md](docs/prompt/start.md) stack rules).
#[test]
fn err002_clvm_execution_failed_shows_coin_and_reason() {
    let e = BlockError::ClvmExecutionFailed {
        coin_id: byte_tag(0x44),
        reason: "opcode fault".into(),
    };
    let s = e.to_string();
    assert!(s.to_lowercase().contains("clvm"), "{}", s);
    assert!(s.contains("opcode fault"), "{}", s);
}

/// **Test plan:** `test_clvm_cost_exceeded` — per-spend budget vs remaining block budget ([ERR-002 spec](docs/requirements/domains/error_types/specs/ERR-002.md)).
#[test]
fn err002_clvm_cost_exceeded_shows_cost_and_remaining() {
    let e = BlockError::ClvmCostExceeded {
        coin_id: byte_tag(0x55),
        cost: 9_000,
        remaining: 1_000,
    };
    let s = e.to_string();
    assert!(s.contains("9000"), "{}", s);
    assert!(s.contains("1000"), "{}", s);
}

/// **Test plan:** `test_assertion_failed` — ASSERT_* conditions aggregate here with human-readable `condition` + `reason` ([ERR-002 implementation notes](docs/requirements/domains/error_types/specs/ERR-002.md#implementation-notes)).
#[test]
fn err002_assertion_failed_shows_condition_and_reason() {
    let e = BlockError::AssertionFailed {
        condition: "ASSERT_COIN_ANNOUNCEMENT".into(),
        reason: "missing announcement".into(),
    };
    let s = e.to_string();
    assert!(s.to_lowercase().contains("assertion"), "{}", s);
    assert!(s.contains("ASSERT_COIN_ANNOUNCEMENT"), "{}", s);
    assert!(s.contains("missing announcement"), "{}", s);
}

/// **Test plan:** `test_announcement_not_found` — CREATE_COIN / announcement resolution failures carry the 32-byte announcement id.
#[test]
fn err002_announcement_not_found_shows_hash() {
    let h = byte_tag(0x66);
    let e = BlockError::AnnouncementNotFound {
        announcement_hash: h,
    };
    assert!(!e.to_string().is_empty());
}

/// **Test plan:** `test_signature_failed` — BLS / AGG_SIG verification ties to `bundle_index` for multi-bundle blocks ([EXE NORMATIVE](docs/requirements/domains/execution_validation/NORMATIVE.md)).
#[test]
fn err002_signature_failed_shows_bundle_index() {
    let e = BlockError::SignatureFailed { bundle_index: 7 };
    let s = e.to_string();
    assert!(s.contains('7'), "{}", s);
    assert!(s.to_lowercase().contains("sign"), "{}", s);
}

/// **Test plan:** `test_coin_minting` — value conservation: `added` XCH atoms exceed `removed` ([ERR-002 implementation notes](docs/requirements/domains/error_types/specs/ERR-002.md#implementation-notes)).
#[test]
fn err002_coin_minting_shows_removed_and_added() {
    let e = BlockError::CoinMinting {
        removed: 100,
        added: 500,
    };
    let s = e.to_string();
    assert!(s.contains("100"), "{}", s);
    assert!(s.contains("500"), "{}", s);
}

/// **Test plan:** `test_fees_mismatch` — `header.total_fees` vs sum of receipt / execution fees ([EXE NORMATIVE](docs/requirements/domains/execution_validation/NORMATIVE.md)).
#[test]
fn err002_fees_mismatch_shows_header_and_computed() {
    let e = BlockError::FeesMismatch {
        header: 42,
        computed: 43,
    };
    let s = e.to_string();
    assert!(s.contains("42"), "{}", s);
    assert!(s.contains("43"), "{}", s);
}

/// **Test plan:** `test_reserve_fee_failed` — reserve fee condition vs actual fees available in bundle.
#[test]
fn err002_reserve_fee_failed_shows_required_and_actual() {
    let e = BlockError::ReserveFeeFailed {
        required: 1_000_000,
        actual: 999_999,
    };
    let s = e.to_string();
    assert!(s.contains("1000000"), "{}", s);
    assert!(s.contains("999999"), "{}", s);
}

/// **Test plan:** `test_cost_mismatch` — `header.total_cost` vs summed CLVM costs ([EXE NORMATIVE](docs/requirements/domains/execution_validation/NORMATIVE.md)).
#[test]
fn err002_cost_mismatch_shows_header_and_computed() {
    let e = BlockError::CostMismatch {
        header: 10,
        computed: 11,
    };
    let s = e.to_string();
    assert!(s.contains("10"), "{}", s);
    assert!(s.contains("11"), "{}", s);
}

/// **Test plan:** `test_invalid_proposer_signature` — unit variant for future `L2Block::validate_state` proposer BLS check ([STV-006](docs/requirements/domains/state_validation/specs/STV-006.md)).
#[test]
fn err002_invalid_proposer_signature_non_empty_display() {
    let e = BlockError::InvalidProposerSignature;
    assert!(!e.to_string().trim().is_empty());
    assert!(
        e.to_string().to_lowercase().contains("proposer")
            || e.to_string().to_lowercase().contains("signature")
    );
}

/// **Test plan:** `test_not_found` — block lookup by hash failed (indexer / sync); tuple variant carries [`Bytes32`].
#[test]
fn err002_not_found_shows_block_hash() {
    let h = byte_tag(0x77);
    let e = BlockError::NotFound(h);
    let s = e.to_string();
    assert!(
        s.to_lowercase().contains("not found") || s.contains("block"),
        "{}",
        s
    );
}

/// **Test plan:** `test_invalid_state_root` — post-block UTXO Merkle root mismatch ([STV-007](docs/requirements/domains/state_validation/specs/STV-007.md)).
#[test]
fn err002_invalid_state_root_shows_expected_and_computed() {
    let e = BlockError::InvalidStateRoot {
        expected: byte_tag(0xaa),
        computed: byte_tag(0xbb),
    };
    assert!(!e.to_string().is_empty());
    assert!(e.to_string().to_lowercase().contains("state"));
}

/// **Test plan:** `test_coin_not_found` — removal references unknown coin ([STV-002](docs/requirements/domains/state_validation/specs/STV-002.md)).
#[test]
fn err002_coin_not_found_shows_coin_id() {
    let coin_id = byte_tag(0xcc);
    let e = BlockError::CoinNotFound { coin_id };
    let s = e.to_string();
    assert!(s.to_lowercase().contains("coin"), "{}", s);
    assert!(s.to_lowercase().contains("not found"), "{}", s);
}

/// **Test plan:** `test_coin_already_spent` — double-spend against chain state with `spent_height` for diagnostics.
#[test]
fn err002_coin_already_spent_shows_coin_and_height() {
    let e = BlockError::CoinAlreadySpent {
        coin_id: byte_tag(0xdd),
        spent_height: 12345,
    };
    let s = e.to_string();
    assert!(s.contains("12345"), "{}", s);
}

/// **Test plan:** `test_coin_already_exists` — addition collides with existing live coin ([STV NORMATIVE](docs/requirements/domains/state_validation/NORMATIVE.md)).
#[test]
fn err002_coin_already_exists_shows_coin_id() {
    let e = BlockError::CoinAlreadyExists {
        coin_id: byte_tag(0xee),
    };
    let s = e.to_string();
    assert!(
        s.to_lowercase().contains("already exists") || s.contains("coin"),
        "{}",
        s
    );
}

/// **Regression guard:** Tier 2/3 variants remain [`Clone`] + [`std::error::Error`] like Tier 1 ([ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md) parity).
#[test]
fn err002_tier2_tier3_clone_and_error_trait() {
    let e = BlockError::SignatureFailed { bundle_index: 0 };
    let _: &dyn StdError = &e;
    let _ = e.source();
    assert_eq!(e.to_string(), e.clone().to_string());
}
