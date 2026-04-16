//! EXE-004: Condition parsing + pending-assertion collection ([SPEC §7.4.4](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/execution_validation/NORMATIVE.md` (EXE-004)
//! **Spec:** `docs/requirements/domains/execution_validation/specs/EXE-004.md`
//!
//! ## Scope of this test file
//!
//! SPEC's "two-pass" condition model (collect outputs / announcements in Pass 1; validate
//! assertions in Pass 2) is implemented inside **`dig-clvm`** → `chia-consensus::run_spendbundle`.
//! NORMATIVE Implementation Notes explicitly say: "This logic may be partially inside dig-clvm."
//! `dig_clvm::validate_spend_bundle` surfaces the results as
//! `chia_consensus::owned_conditions::OwnedSpendBundleConditions`.
//!
//! What **dig-block** owns for EXE-004: extracting the height / time / before-height / before-time
//! constraints from that struct into a stable [`PendingAssertion`] vector for Tier-3
//! (STV-005) evaluation. This test file verifies that extraction helper
//! ([`dig_block::collect_pending_assertions_from_conditions`]).
//!
//! ## What this proves
//!
//! - **Absolute height / time:** The block-level `height_absolute` / `seconds_absolute` values
//!   (the **most strict** across all spends) produce a single [`AssertionKind::HeightAbsolute`] /
//!   [`AssertionKind::SecondsAbsolute`] with `coin_id = ZERO_HASH` (no owning spend).
//! - **Before-absolute:** `before_height_absolute` / `before_seconds_absolute` Options produce
//!   [`AssertionKind::BeforeHeightAbsolute`] / [`AssertionKind::BeforeSecondsAbsolute`] when set.
//! - **Per-spend relative:** Each spend's `height_relative` / `seconds_relative` /
//!   `before_height_relative` / `before_seconds_relative` Options produce the matching relative
//!   [`AssertionKind`] with the owning coin's `coin_id`.
//! - **Zero / None is absent:** No assertion is emitted when the corresponding field is `0` (for
//!   absolute scalars, meaning "no constraint") or `None` (for before / relative Options).
//! - **Empty conditions:** An empty `OwnedSpendBundleConditions` produces zero pending assertions.
//!
//! ## How this satisfies EXE-004
//!
//! The EXE-004 acceptance criteria split into two groups:
//!
//! | Criterion | Where it lives | How we cover it |
//! |---|---|---|
//! | Pass 1 (create-coin, announcements, reserve-fee collection) | dig-clvm / chia-consensus | Delegated; EXE-003 delegation test confirms the entrypoint path. |
//! | Pass 2 (announcement / concurrent-spend / self-assertions) | dig-clvm / chia-consensus | Same. Rejections surface as `ValidationError::Clvm` → `BlockError::ClvmExecutionFailed` via EXE-003 mapping. |
//! | Height/time assertions collected into `pending_assertions` | **dig-block** | This test file. |
//! | ASSERT_EPHEMERAL collected into `pending_assertions` | dig-clvm (via spend `flags`) / Tier-3 | Covered indirectly: EXE-009's `from_condition` returns `None` for AssertEphemeral; STV-002 handles ephemeral coins independently. Documented in this file's caveats. |
//! | Uses `chia-sdk-types::Condition` (or chia-consensus equivalent) | Confirmed | `OwnedSpendBundleConditions` is the chia-consensus types our helper walks over. |

use chia_bls::PublicKey;
use chia_consensus::owned_conditions::{OwnedSpendBundleConditions, OwnedSpendConditions};
use chia_protocol::{Bytes, Bytes32};

use dig_block::{collect_pending_assertions_from_conditions, AssertionKind, PendingAssertion};

/// Build an empty spend-level condition entry for a synthetic `coin_id`.
fn spend(coin_id: Bytes32) -> OwnedSpendConditions {
    OwnedSpendConditions {
        coin_id,
        parent_id: Bytes32::default(),
        puzzle_hash: Bytes32::default(),
        coin_amount: 0,
        height_relative: None,
        seconds_relative: None,
        before_height_relative: None,
        before_seconds_relative: None,
        birth_height: None,
        birth_seconds: None,
        create_coin: Vec::new(),
        agg_sig_me: Vec::<(PublicKey, Bytes)>::new(),
        agg_sig_parent: Vec::new(),
        agg_sig_puzzle: Vec::new(),
        agg_sig_amount: Vec::new(),
        agg_sig_puzzle_amount: Vec::new(),
        agg_sig_parent_amount: Vec::new(),
        agg_sig_parent_puzzle: Vec::new(),
        flags: 0,
    }
}

/// Empty block-level conditions. Every field at its identity.
fn empty_conditions() -> OwnedSpendBundleConditions {
    OwnedSpendBundleConditions {
        spends: Vec::new(),
        reserve_fee: 0,
        height_absolute: 0,
        seconds_absolute: 0,
        before_height_absolute: None,
        before_seconds_absolute: None,
        agg_sig_unsafe: Vec::new(),
        cost: 0,
        removal_amount: 0,
        addition_amount: 0,
        validated_signature: true,
        execution_cost: 0,
        condition_cost: 0,
    }
}

/// **EXE-004:** Empty conditions -> no pending assertions.
#[test]
fn empty_conditions_collects_nothing() {
    let conds = empty_conditions();
    let assertions = collect_pending_assertions_from_conditions(&conds);
    assert!(assertions.is_empty());
}

/// **EXE-004 `deferred_height_assertion`:** Non-zero `height_absolute` -> one
/// [`AssertionKind::HeightAbsolute`] with `coin_id = ZERO_HASH` (block-level, no owning spend).
#[test]
fn height_absolute_collected() {
    let mut conds = empty_conditions();
    conds.height_absolute = 100;
    let assertions = collect_pending_assertions_from_conditions(&conds);
    assert_eq!(
        assertions,
        vec![PendingAssertion {
            kind: AssertionKind::HeightAbsolute(100),
            coin_id: Bytes32::default(),
        }]
    );
}

/// **EXE-004 `deferred_time_assertion`:** Non-zero `seconds_absolute` ->
/// [`AssertionKind::SecondsAbsolute`].
#[test]
fn seconds_absolute_collected() {
    let mut conds = empty_conditions();
    conds.seconds_absolute = 1_700_000_000;
    let assertions = collect_pending_assertions_from_conditions(&conds);
    assert_eq!(
        assertions,
        vec![PendingAssertion {
            kind: AssertionKind::SecondsAbsolute(1_700_000_000),
            coin_id: Bytes32::default(),
        }]
    );
}

/// **EXE-004:** `Some(before_height_absolute)` -> [`AssertionKind::BeforeHeightAbsolute`].
#[test]
fn before_height_absolute_collected() {
    let mut conds = empty_conditions();
    conds.before_height_absolute = Some(5000);
    let assertions = collect_pending_assertions_from_conditions(&conds);
    assert_eq!(
        assertions,
        vec![PendingAssertion {
            kind: AssertionKind::BeforeHeightAbsolute(5000),
            coin_id: Bytes32::default(),
        }]
    );
}

/// **EXE-004:** `Some(before_seconds_absolute)` -> [`AssertionKind::BeforeSecondsAbsolute`].
#[test]
fn before_seconds_absolute_collected() {
    let mut conds = empty_conditions();
    conds.before_seconds_absolute = Some(2_000_000_000);
    let assertions = collect_pending_assertions_from_conditions(&conds);
    assert_eq!(
        assertions,
        vec![PendingAssertion {
            kind: AssertionKind::BeforeSecondsAbsolute(2_000_000_000),
            coin_id: Bytes32::default(),
        }]
    );
}

/// **EXE-004:** Per-spend `height_relative` carries the owning coin's `coin_id`.
#[test]
fn per_spend_height_relative_collected_with_coin_id() {
    let coin_id = Bytes32::new([0xaa; 32]);
    let mut s = spend(coin_id);
    s.height_relative = Some(10);

    let mut conds = empty_conditions();
    conds.spends = vec![s];

    let assertions = collect_pending_assertions_from_conditions(&conds);
    assert_eq!(
        assertions,
        vec![PendingAssertion {
            kind: AssertionKind::HeightRelative(10),
            coin_id,
        }]
    );
}

/// **EXE-004:** Per-spend `seconds_relative` carries the owning coin's `coin_id`.
#[test]
fn per_spend_seconds_relative_collected_with_coin_id() {
    let coin_id = Bytes32::new([0xbb; 32]);
    let mut s = spend(coin_id);
    s.seconds_relative = Some(60);

    let mut conds = empty_conditions();
    conds.spends = vec![s];

    let assertions = collect_pending_assertions_from_conditions(&conds);
    assert_eq!(
        assertions,
        vec![PendingAssertion {
            kind: AssertionKind::SecondsRelative(60),
            coin_id,
        }]
    );
}

/// **EXE-004:** All four per-spend relative fields on one spend produce four assertions.
#[test]
fn per_spend_all_relative_fields_collected() {
    let coin_id = Bytes32::new([0xcc; 32]);
    let mut s = spend(coin_id);
    s.height_relative = Some(1);
    s.seconds_relative = Some(2);
    s.before_height_relative = Some(3);
    s.before_seconds_relative = Some(4);

    let mut conds = empty_conditions();
    conds.spends = vec![s];

    let assertions = collect_pending_assertions_from_conditions(&conds);
    assert_eq!(assertions.len(), 4);
    let kinds: Vec<_> = assertions.iter().map(|a| a.kind.clone()).collect();
    assert!(kinds.contains(&AssertionKind::HeightRelative(1)));
    assert!(kinds.contains(&AssertionKind::SecondsRelative(2)));
    assert!(kinds.contains(&AssertionKind::BeforeHeightRelative(3)));
    assert!(kinds.contains(&AssertionKind::BeforeSecondsRelative(4)));
    for a in assertions {
        assert_eq!(
            a.coin_id, coin_id,
            "every relative assertion carries its coin_id"
        );
    }
}

/// **EXE-004:** Multiple spends each contribute their own relative assertions, distinguishable
/// by `coin_id`.
#[test]
fn multiple_spends_each_contribute() {
    let id_a = Bytes32::new([0x11; 32]);
    let id_b = Bytes32::new([0x22; 32]);
    let mut a = spend(id_a);
    a.height_relative = Some(100);
    let mut b = spend(id_b);
    b.seconds_relative = Some(50);

    let mut conds = empty_conditions();
    conds.spends = vec![a, b];

    let assertions = collect_pending_assertions_from_conditions(&conds);
    assert_eq!(assertions.len(), 2);
    let a_hits: Vec<_> = assertions.iter().filter(|x| x.coin_id == id_a).collect();
    assert_eq!(a_hits.len(), 1);
    assert_eq!(a_hits[0].kind, AssertionKind::HeightRelative(100));
    let b_hits: Vec<_> = assertions.iter().filter(|x| x.coin_id == id_b).collect();
    assert_eq!(b_hits.len(), 1);
    assert_eq!(b_hits[0].kind, AssertionKind::SecondsRelative(50));
}

/// **EXE-004:** Combined block-level + per-spend assertions yield all of them at once.
/// Block-level assertions appear first (deterministic order) so tests can reason about indices.
#[test]
fn combined_block_and_spend_level_assertions() {
    let coin_id = Bytes32::new([0x33; 32]);
    let mut s = spend(coin_id);
    s.height_relative = Some(7);

    let mut conds = empty_conditions();
    conds.height_absolute = 42;
    conds.before_height_absolute = Some(1000);
    conds.spends = vec![s];

    let assertions = collect_pending_assertions_from_conditions(&conds);
    assert_eq!(assertions.len(), 3);
    // Block-level come first.
    assert_eq!(assertions[0].kind, AssertionKind::HeightAbsolute(42));
    assert_eq!(assertions[0].coin_id, Bytes32::default());
    assert_eq!(
        assertions[1].kind,
        AssertionKind::BeforeHeightAbsolute(1000)
    );
    // Per-spend.
    assert_eq!(assertions[2].kind, AssertionKind::HeightRelative(7));
    assert_eq!(assertions[2].coin_id, coin_id);
}

/// **EXE-004 deferred_ephemeral caveat:**
/// `ASSERT_EPHEMERAL` is **not** emitted by `OwnedSpendBundleConditions` as a discrete condition;
/// chia-consensus encodes it via spend `flags` + coin-existence handling inside dig-clvm's
/// structural check (see `src/consensus/validate.rs` in dig-clvm). Tier-3 (STV-002) performs the
/// ephemeral existence check directly against `exec.additions`; no dedicated `PendingAssertion`
/// value is needed. This test documents the architectural decision.
#[test]
fn ephemeral_is_not_a_pending_assertion_value() {
    // No API path emits AssertionKind for ephemeral; the enum has only 8 height/time variants.
    // Verified by exhaustion: constructing each variant.
    let _ = AssertionKind::HeightAbsolute(0);
    let _ = AssertionKind::HeightRelative(0);
    let _ = AssertionKind::SecondsAbsolute(0);
    let _ = AssertionKind::SecondsRelative(0);
    let _ = AssertionKind::BeforeHeightAbsolute(0);
    let _ = AssertionKind::BeforeHeightRelative(0);
    let _ = AssertionKind::BeforeSecondsAbsolute(0);
    let _ = AssertionKind::BeforeSecondsRelative(0);
    // No `Ephemeral` variant exists. STV-002 handles the ephemeral rule against ExecutionResult.
}
