//! CKP-004: [`Checkpoint::compute_score`] — stake × block coverage per NORMATIVE.
//!
//! **Normative:** `docs/requirements/domains/checkpoint/NORMATIVE.md` (CKP-004)  
//! **Spec + test plan:** `docs/requirements/domains/checkpoint/specs/CKP-004.md`  
//! **Implementation:** `src/types/checkpoint.rs`
//!
//! ## How these tests prove CKP-004
//!
//! - **Formula:** Every case asserts `compute_score(stake_percentage) == stake_percentage * block_count` with
//!   `block_count` promoted to `u64` as in the spec ([CKP-004 § Specification](docs/requirements/domains/checkpoint/specs/CKP-004.md#specification)).
//! - **Zeros:** [CKP-004 acceptance](docs/requirements/domains/checkpoint/specs/CKP-004.md#acceptance-criteria) requires `0` when either factor is `0` — dedicated tests avoid accidentally using a different identity element.
//! - **Representative magnitudes:** The test plan’s “typical”, “full stake”, and “minimal” rows mirror how competition code compares [`CheckpointSubmission::score`](docs/requirements/domains/checkpoint/specs/CKP-002.md) inputs derived from this helper ([CKP-004 implementation notes](docs/requirements/domains/checkpoint/specs/CKP-004.md#implementation-notes)).
//! - **Wide product:** One test uses a large-but-safe product to show the method behaves on values that still fit in
//!   `u64` (protocol bounds are tighter; this guards against accidental narrowing or wrong operand order).

use dig_block::Checkpoint;

/// Build a checkpoint whose only scoring input is [`Checkpoint::block_count`] (other fields irrelevant to CKP-004).
fn checkpoint_with_block_count(block_count: u32) -> Checkpoint {
    let mut c = Checkpoint::new();
    c.block_count = block_count;
    c
}

/// **Test plan:** “Score with typical values” — stake `67`, `block_count` `100` → `6700`.
#[test]
fn ckp004_score_typical_values() {
    let c = checkpoint_with_block_count(100);
    assert_eq!(c.compute_score(67), 6_700);
}

/// **Test plan:** “Score with zero stake” — multiplicative zero annihilates.
#[test]
fn ckp004_score_zero_stake() {
    let c = checkpoint_with_block_count(100);
    assert_eq!(c.compute_score(0), 0);
}

/// **Test plan:** “Score with zero blocks” — empty epoch yields zero score for any stake input.
#[test]
fn ckp004_score_zero_blocks() {
    let c = checkpoint_with_block_count(0);
    assert_eq!(c.compute_score(67), 0);
}

/// **Test plan:** “Score with full stake” — `100 × 225 == 22500` (illustrative epoch length per spec notes).
#[test]
fn ckp004_score_full_stake_epoch_length() {
    let c = checkpoint_with_block_count(225);
    assert_eq!(c.compute_score(100), 22_500);
}

/// **Test plan:** “Score with minimal values” — `1 × 1 == 1`.
#[test]
fn ckp004_score_minimal_nonzero() {
    let c = checkpoint_with_block_count(1);
    assert_eq!(c.compute_score(1), 1);
}

/// **Acceptance:** “Handles large values without overflow **within** `u64` range” — product stays below `u64::MAX`.
///
/// **Rationale:** We pick operands whose product is large vs. toy examples but still far from wrap/panic territory;
/// callers with absurd inputs rely on protocol limits ([CKP-004 implementation notes](docs/requirements/domains/checkpoint/specs/CKP-004.md#implementation-notes)).
#[test]
fn ckp004_score_large_product_within_u64() {
    let c = checkpoint_with_block_count(2_000_000);
    let stake = 500_000u64;
    let expected = stake * 2_000_000u64;
    assert_eq!(c.compute_score(stake), expected);
    assert!(expected < u64::MAX);
}
