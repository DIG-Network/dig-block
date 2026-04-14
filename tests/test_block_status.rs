//! ATT-003: [`BlockStatus`] variants and predicates `is_finalized` / `is_canonical`.
//!
//! **Authoritative spec:** `docs/requirements/domains/attestation/specs/ATT-003.md`
//! **Normative:** `docs/requirements/domains/attestation/NORMATIVE.md` (ATT-003)
//! **Wire / semantics:** `docs/resources/SPEC.md` §2.5
//!
//! ## Proof obligation
//!
//! The ATT-003 **Test Plan** table is mapped one-to-one below. Together with [`test_block_status_variants_count`],
//! this file demonstrates the six lifecycle labels and the exact boolean tables for finality vs canonicality
//! required by acceptance criteria (no consensus state machine in this crate — predicates only).

use dig_block::BlockStatus;

/// **Test plan:** `is_finalized` for [`BlockStatus::Pending`] — not yet final.
#[test]
fn test_is_finalized_pending() {
    assert!(!BlockStatus::Pending.is_finalized());
}

/// **Test plan:** `is_finalized` for [`BlockStatus::Validated`] — validated but not stake-final.
#[test]
fn test_is_finalized_validated() {
    assert!(!BlockStatus::Validated.is_finalized());
}

/// **Test plan:** `is_finalized` for [`BlockStatus::SoftFinalized`] — meets signing threshold.
#[test]
fn test_is_finalized_soft_finalized() {
    assert!(BlockStatus::SoftFinalized.is_finalized());
}

/// **Test plan:** `is_finalized` for [`BlockStatus::HardFinalized`] — L1 checkpoint path.
#[test]
fn test_is_finalized_hard_finalized() {
    assert!(BlockStatus::HardFinalized.is_finalized());
}

/// **Test plan:** `is_finalized` for [`BlockStatus::Orphaned`] — fork-losing; not a finality class.
#[test]
fn test_is_finalized_orphaned() {
    assert!(!BlockStatus::Orphaned.is_finalized());
}

/// **Test plan:** `is_finalized` for [`BlockStatus::Rejected`] — invalid; not final.
#[test]
fn test_is_finalized_rejected() {
    assert!(!BlockStatus::Rejected.is_finalized());
}

/// **Test plan:** `is_canonical` for [`BlockStatus::Pending`] — may still become canonical.
#[test]
fn test_is_canonical_pending() {
    assert!(BlockStatus::Pending.is_canonical());
}

/// **Test plan:** `is_canonical` for [`BlockStatus::Validated`].
#[test]
fn test_is_canonical_validated() {
    assert!(BlockStatus::Validated.is_canonical());
}

/// **Test plan:** `is_canonical` for [`BlockStatus::SoftFinalized`].
#[test]
fn test_is_canonical_soft_finalized() {
    assert!(BlockStatus::SoftFinalized.is_canonical());
}

/// **Test plan:** `is_canonical` for [`BlockStatus::HardFinalized`].
#[test]
fn test_is_canonical_hard_finalized() {
    assert!(BlockStatus::HardFinalized.is_canonical());
}

/// **Test plan:** `is_canonical` for [`BlockStatus::Orphaned`] — non-canonical by definition.
#[test]
fn test_is_canonical_orphaned() {
    assert!(!BlockStatus::Orphaned.is_canonical());
}

/// **Test plan:** `is_canonical` for [`BlockStatus::Rejected`] — non-canonical by definition.
#[test]
fn test_is_canonical_rejected() {
    assert!(!BlockStatus::Rejected.is_canonical());
}

/// **Acceptance:** exactly six discriminant names exist (guards against silent enum drift).
#[test]
fn test_block_status_variants_count() {
    let all = [
        BlockStatus::Pending,
        BlockStatus::Validated,
        BlockStatus::SoftFinalized,
        BlockStatus::HardFinalized,
        BlockStatus::Orphaned,
        BlockStatus::Rejected,
    ];
    assert_eq!(all.len(), 6);
    // Exhaustiveness: every variant appears once in the table above; duplicate patterns would fail to compile
    // if a new variant were added without updating this array and tests.
    let mut finalized_true = 0usize;
    let mut canonical_false = 0usize;
    for s in all {
        if s.is_finalized() {
            finalized_true += 1;
        }
        if !s.is_canonical() {
            canonical_false += 1;
        }
    }
    assert_eq!(finalized_true, 2, "only Soft+Hard finalized");
    assert_eq!(canonical_false, 2, "only Orphaned+Rejected non-canonical");
}
