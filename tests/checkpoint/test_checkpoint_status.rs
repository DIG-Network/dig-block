//! CKP-003: [`dig_block::CheckpointStatus`] — five-variant epoch lifecycle per NORMATIVE.
//!
//! **Normative:** `docs/requirements/domains/checkpoint/NORMATIVE.md` (CKP-003)  
//! **Spec + test plan:** `docs/requirements/domains/checkpoint/specs/CKP-003.md`  
//! **Implementation:** `src/types/status.rs` ([`CheckpointStatus`]) — paired with [`dig_block::BlockStatus`] (ATT-003) in the same module for serialization symmetry (SER-001).
//!
//! ## How these tests prove CKP-003
//!
//! - **Variant surface:** Each test constructs one variant and matches it; Rust’s exhaustiveness checker ensures we cover every variant when we use a single `match` that must stay in sync with the enum ([CKP-003 acceptance — pattern matching](docs/requirements/domains/checkpoint/specs/CKP-003.md#acceptance-criteria)).
//! - **Payload shapes:** [`CheckpointStatus::WinnerSelected`] and [`CheckpointStatus::Finalized`] carry [`dig_block::Bytes32`] plus scalars exactly as the spec table describes; field-level assertions prove accessors / destructure bindings see the same values used at construction.
//! - **Unit vs data variants:** `Pending`, `Collecting`, and `Failed` are exercised as zero-field variants so regressions that accidentally add fields fail at construction or match sites.

use dig_block::{Bytes32, CheckpointStatus};

/// Test helper: deterministic [`Bytes32`] tagged by a byte (same pattern as receipt tests — RCP-002).
///
/// **Rationale:** Keeps assertions readable; any `u8` tag yields a distinct 32-byte pattern.
fn byte_tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// **Test plan:** “Create Pending” — unit variant, no payload ([CKP-003 § Test Plan](docs/requirements/domains/checkpoint/specs/CKP-003.md#test-plan)).
#[test]
fn ckp003_create_pending_matches() {
    let s = CheckpointStatus::Pending;
    assert!(matches!(s, CheckpointStatus::Pending));
}

/// **Test plan:** “Create Collecting”.
#[test]
fn ckp003_create_collecting_matches() {
    let s = CheckpointStatus::Collecting;
    assert!(matches!(s, CheckpointStatus::Collecting));
}

/// **Test plan:** “Create Failed” — terminal failure state, no payload.
#[test]
fn ckp003_create_failed_matches() {
    let s = CheckpointStatus::Failed;
    assert!(matches!(s, CheckpointStatus::Failed));
}

/// **Test plan:** “Create WinnerSelected” + “WinnerSelected data access”.
///
/// **Proves:** `winner_hash` is [`Bytes32`] and `winner_score` is `u64`, readable after construction ([CKP-003 § Specification](docs/requirements/domains/checkpoint/specs/CKP-003.md#specification)).
#[test]
fn ckp003_winner_selected_construct_and_extract_fields() {
    let hash = byte_tag(0x5a);
    let score = 0xdead_beef_cafe_babe;
    let s = CheckpointStatus::WinnerSelected {
        winner_hash: hash,
        winner_score: score,
    };
    match s {
        CheckpointStatus::WinnerSelected {
            winner_hash,
            winner_score,
        } => {
            assert_eq!(winner_hash, hash);
            assert_eq!(winner_score, score);
        }
        _ => panic!("expected WinnerSelected"),
    }
}

/// **Test plan:** “Create Finalized” + “Finalized data access”.
///
/// **Proves:** Same winner identity as off-chain selection, pinned with on-chain `l1_height: u32` ([SPEC §2.8](docs/resources/SPEC.md) via CKP-003 trace).
#[test]
fn ckp003_finalized_construct_and_extract_fields() {
    let hash = byte_tag(0x71);
    let height: u32 = 9_001;
    let s = CheckpointStatus::Finalized {
        winner_hash: hash,
        l1_height: height,
    };
    match s {
        CheckpointStatus::Finalized {
            winner_hash,
            l1_height,
        } => {
            assert_eq!(winner_hash, hash);
            assert_eq!(l1_height, height);
        }
        _ => panic!("expected Finalized"),
    }
}

/// **Exhaustive label match** — if a sixth variant is added without updating this test, the compiler errors (preferred guardrail over manual “variant count” constants).
///
/// **Proves:** Pattern matching works on **all** variants ([CKP-003 acceptance](docs/requirements/domains/checkpoint/specs/CKP-003.md#acceptance-criteria)).
#[test]
fn ckp003_exhaustive_match_all_variants() {
    let cases: [CheckpointStatus; 5] = [
        CheckpointStatus::Pending,
        CheckpointStatus::Collecting,
        CheckpointStatus::WinnerSelected {
            winner_hash: byte_tag(1),
            winner_score: 1,
        },
        CheckpointStatus::Finalized {
            winner_hash: byte_tag(2),
            l1_height: 2,
        },
        CheckpointStatus::Failed,
    ];
    for s in cases {
        let label = match s {
            CheckpointStatus::Pending => "pending",
            CheckpointStatus::Collecting => "collecting",
            CheckpointStatus::WinnerSelected { .. } => "winner_selected",
            CheckpointStatus::Finalized { .. } => "finalized",
            CheckpointStatus::Failed => "failed",
        };
        assert!(!label.is_empty());
    }
}

/// **`Copy` + `Eq` sanity** — [`CheckpointStatus`] is intended for hot-path comparisons in consensus-adjacent code; duplicating a value must not alias interior mutability (there is none) and equality must be structural.
///
/// **Rationale:** CKP-003 does not mandate `Copy`, but `src/types/status.rs` documents parity with [`BlockStatus`]; this test catches accidental removal of `Copy`/`Eq` that would ripple to downstream matches.
#[test]
fn ckp003_copy_and_eq_roundtrip() {
    let a = CheckpointStatus::WinnerSelected {
        winner_hash: byte_tag(0x0c),
        winner_score: 42,
    };
    let b = a;
    assert_eq!(a, b);
    assert_eq!(
        a,
        CheckpointStatus::WinnerSelected {
            winner_hash: byte_tag(0x0c),
            winner_score: 42,
        }
    );
}
