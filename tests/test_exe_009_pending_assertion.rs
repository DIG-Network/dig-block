//! EXE-009: `PendingAssertion` type — 8 height/time lock variants + `from_condition` factory.
//!
//! **Normative:** `docs/requirements/domains/execution_validation/NORMATIVE.md` (EXE-009)
//! **Spec:** `docs/requirements/domains/execution_validation/specs/EXE-009.md`
//! **Crate SPEC:** [SPEC §7.4.4 / §7.4.7 / §7.5.4](docs/resources/SPEC.md)
//!
//! ## What this proves
//!
//! - **Enum completeness:** [`AssertionKind`] has the 8 variants NORMATIVE requires — each of the
//!   four `ASSERT_*` opcodes (`HEIGHT_ABSOLUTE` / `HEIGHT_RELATIVE` / `SECONDS_ABSOLUTE` /
//!   `SECONDS_RELATIVE`) and the matching `BEFORE_*` opcodes. Every variant has a `u64` payload
//!   (heights widened from `u32`, seconds native) to unify Tier-3 comparisons in STV-005.
//!
//! - **Struct shape:** [`PendingAssertion`] has exactly two fields, `kind: AssertionKind` and
//!   `coin_id: Bytes32`. The `coin_id` is always populated because **any** of the 8 assertions can
//!   be relative — STV-005 looks up the coin's `created_height` / creation timestamp through
//!   [`dig_block::CoinLookup`].
//!
//! - **`from_condition` factory:** Maps each of the 8 `chia_sdk_types::Condition` height / time
//!   variants to the corresponding [`AssertionKind`], returning `None` for any other condition
//!   (including `AssertEphemeral`, which EXE-004 handles separately). This proves the mapping is
//!   total over the 8 variants and disjoint from everything else.
//!
//! - **Serialization:** Both types derive `Serialize` / `Deserialize` and round-trip through
//!   bincode ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)).
//!
//! - **Integration:** `PendingAssertion` is the element type of
//!   [`dig_block::ExecutionResult::pending_assertions`] ([EXE-008](docs/requirements/domains/execution_validation/specs/EXE-008.md))
//!   and is consumed by `L2Block::validate_state` per STV-005.
//!
//! ## How this satisfies EXE-009
//!
//! One test per bullet in the spec test plan (11 tests total); `coin_id_captured` is checked in
//! every variant test to avoid duplication. The mapping table in the spec is verified exhaustively.

use chia_protocol::{Bytes32, Coin, CoinSpend, Program};
use chia_sdk_types::conditions::{
    AssertBeforeHeightAbsolute, AssertBeforeHeightRelative, AssertBeforeSecondsAbsolute,
    AssertBeforeSecondsRelative, AssertEphemeral, AssertHeightAbsolute, AssertHeightRelative,
    AssertSecondsAbsolute, AssertSecondsRelative, CreateCoin,
};
use chia_sdk_types::Condition;

use dig_block::{AssertionKind, ExecutionResult, PendingAssertion};

/// Reusable test coin so every assertion has a deterministic `coin_id`.
fn test_spend() -> (CoinSpend, Bytes32) {
    let parent = Bytes32::new([0x11; 32]);
    let puzzle_hash = Bytes32::new([0x22; 32]);
    let coin = Coin::new(parent, puzzle_hash, 1);
    let coin_id = coin.coin_id();
    let spend = CoinSpend::new(coin, Program::from(vec![1]), Program::from(vec![0x80]));
    (spend, coin_id)
}

// ---------------------------------------------------------------------------
// Each of the 8 height/time variants — test plan: `*_variant` + `coin_id_captured`
// ---------------------------------------------------------------------------

/// **EXE-009 `height_absolute_variant`:** `ASSERT_HEIGHT_ABSOLUTE(h)` → `HeightAbsolute(h)` with
/// `u32` → `u64` widening. Also proves `coin_id` matches `coin.coin_id()`.
#[test]
fn height_absolute_variant() {
    let (spend, coin_id) = test_spend();
    let cond: Condition<()> = Condition::AssertHeightAbsolute(AssertHeightAbsolute { height: 123 });
    let p = PendingAssertion::from_condition(&cond, &spend).expect("must map");
    assert_eq!(p.kind, AssertionKind::HeightAbsolute(123));
    assert_eq!(p.coin_id, coin_id);
}

/// **EXE-009 `height_relative_variant`:** `ASSERT_HEIGHT_RELATIVE(h)` → `HeightRelative(h)`.
/// STV-005 will compare against `coin_confirmed_height + h`.
#[test]
fn height_relative_variant() {
    let (spend, coin_id) = test_spend();
    let cond: Condition<()> = Condition::AssertHeightRelative(AssertHeightRelative { height: 45 });
    let p = PendingAssertion::from_condition(&cond, &spend).expect("must map");
    assert_eq!(p.kind, AssertionKind::HeightRelative(45));
    assert_eq!(p.coin_id, coin_id);
}

/// **EXE-009 `seconds_absolute_variant`:** `ASSERT_SECONDS_ABSOLUTE(t)` → `SecondsAbsolute(t)`.
#[test]
fn seconds_absolute_variant() {
    let (spend, coin_id) = test_spend();
    let cond: Condition<()> = Condition::AssertSecondsAbsolute(AssertSecondsAbsolute {
        seconds: 1_700_000_000,
    });
    let p = PendingAssertion::from_condition(&cond, &spend).expect("must map");
    assert_eq!(p.kind, AssertionKind::SecondsAbsolute(1_700_000_000));
    assert_eq!(p.coin_id, coin_id);
}

/// **EXE-009 `seconds_relative_variant`:** `ASSERT_SECONDS_RELATIVE(t)` → `SecondsRelative(t)`.
/// STV-005 compares against `coin_timestamp + t`.
#[test]
fn seconds_relative_variant() {
    let (spend, coin_id) = test_spend();
    let cond: Condition<()> =
        Condition::AssertSecondsRelative(AssertSecondsRelative { seconds: 60 });
    let p = PendingAssertion::from_condition(&cond, &spend).expect("must map");
    assert_eq!(p.kind, AssertionKind::SecondsRelative(60));
    assert_eq!(p.coin_id, coin_id);
}

/// **EXE-009 `before_height_absolute_variant`:** `ASSERT_BEFORE_HEIGHT_ABSOLUTE(h)` →
/// `BeforeHeightAbsolute(h)`. STV-005 asserts `chain_height < h`.
#[test]
fn before_height_absolute_variant() {
    let (spend, coin_id) = test_spend();
    let cond: Condition<()> =
        Condition::AssertBeforeHeightAbsolute(AssertBeforeHeightAbsolute { height: 9000 });
    let p = PendingAssertion::from_condition(&cond, &spend).expect("must map");
    assert_eq!(p.kind, AssertionKind::BeforeHeightAbsolute(9000));
    assert_eq!(p.coin_id, coin_id);
}

/// **EXE-009 `before_height_relative_variant`:** `ASSERT_BEFORE_HEIGHT_RELATIVE(h)`.
#[test]
fn before_height_relative_variant() {
    let (spend, coin_id) = test_spend();
    let cond: Condition<()> =
        Condition::AssertBeforeHeightRelative(AssertBeforeHeightRelative { height: 10 });
    let p = PendingAssertion::from_condition(&cond, &spend).expect("must map");
    assert_eq!(p.kind, AssertionKind::BeforeHeightRelative(10));
    assert_eq!(p.coin_id, coin_id);
}

/// **EXE-009 `before_seconds_absolute_variant`:** `ASSERT_BEFORE_SECONDS_ABSOLUTE(t)`.
#[test]
fn before_seconds_absolute_variant() {
    let (spend, coin_id) = test_spend();
    let cond: Condition<()> = Condition::AssertBeforeSecondsAbsolute(AssertBeforeSecondsAbsolute {
        seconds: 2_000_000_000,
    });
    let p = PendingAssertion::from_condition(&cond, &spend).expect("must map");
    assert_eq!(p.kind, AssertionKind::BeforeSecondsAbsolute(2_000_000_000));
    assert_eq!(p.coin_id, coin_id);
}

/// **EXE-009 `before_seconds_relative_variant`:** `ASSERT_BEFORE_SECONDS_RELATIVE(t)`.
#[test]
fn before_seconds_relative_variant() {
    let (spend, coin_id) = test_spend();
    let cond: Condition<()> =
        Condition::AssertBeforeSecondsRelative(AssertBeforeSecondsRelative { seconds: 120 });
    let p = PendingAssertion::from_condition(&cond, &spend).expect("must map");
    assert_eq!(p.kind, AssertionKind::BeforeSecondsRelative(120));
    assert_eq!(p.coin_id, coin_id);
}

// ---------------------------------------------------------------------------
// Non-height/time conditions must return None — test plan: `non_assertion_returns_none`
// ---------------------------------------------------------------------------

/// **EXE-009 `non_assertion_returns_none`:** `CREATE_COIN` is a creation condition, not a
/// height/time lock — `from_condition` returns `None` and the condition is processed elsewhere
/// (EXE-004 Pass 1 collector).
#[test]
fn create_coin_returns_none() {
    let (spend, _) = test_spend();
    let cond: Condition<()> = Condition::CreateCoin(CreateCoin {
        puzzle_hash: Bytes32::new([0xaa; 32]),
        amount: 10,
        memos: chia_sdk_types::conditions::Memos::Some(()),
    });
    assert!(PendingAssertion::from_condition(&cond, &spend).is_none());
}

/// **EXE-009 `non_assertion_returns_none`:** `ASSERT_EPHEMERAL` is a consensus condition but not a
/// height/time lock (spec: EXE-004 has its own treatment). `from_condition` must return `None`.
#[test]
fn assert_ephemeral_returns_none() {
    let (spend, _) = test_spend();
    let cond: Condition<()> = Condition::AssertEphemeral(AssertEphemeral::default());
    assert!(
        PendingAssertion::from_condition(&cond, &spend).is_none(),
        "ASSERT_EPHEMERAL is handled by EXE-004 separately, not via PendingAssertion"
    );
}

// ---------------------------------------------------------------------------
// Serialization — test plan: `serialization_roundtrip`
// ---------------------------------------------------------------------------

/// **EXE-009 `serialization_roundtrip`:** `PendingAssertion` round-trips through bincode without
/// loss (SER-001). Value equality via derived `PartialEq`.
#[test]
fn pending_assertion_bincode_roundtrip() {
    let coin_id = Bytes32::new([0xaa; 32]);
    let original = PendingAssertion {
        kind: AssertionKind::HeightRelative(500),
        coin_id,
    };
    let bytes = bincode::serialize(&original).expect("encode");
    let decoded: PendingAssertion = bincode::deserialize(&bytes).expect("decode");
    assert_eq!(original, decoded);
}

/// **EXE-009 `serialization_roundtrip`:** `AssertionKind` round-trips for every variant.
#[test]
fn assertion_kind_all_variants_roundtrip() {
    let variants = [
        AssertionKind::HeightAbsolute(1),
        AssertionKind::HeightRelative(2),
        AssertionKind::SecondsAbsolute(3),
        AssertionKind::SecondsRelative(4),
        AssertionKind::BeforeHeightAbsolute(5),
        AssertionKind::BeforeHeightRelative(6),
        AssertionKind::BeforeSecondsAbsolute(7),
        AssertionKind::BeforeSecondsRelative(8),
    ];
    for v in variants {
        let bytes = bincode::serialize(&v).expect("encode");
        let decoded: AssertionKind = bincode::deserialize(&bytes).expect("decode");
        assert_eq!(v, decoded, "variant {:?} must round-trip", v);
    }
}

// ---------------------------------------------------------------------------
// Integration with EXE-008 — `PendingAssertion` as `ExecutionResult.pending_assertions` element.
// ---------------------------------------------------------------------------

/// **EXE-009 integration:** `Vec<PendingAssertion>` is the element type of
/// [`ExecutionResult::pending_assertions`] ([EXE-008](docs/requirements/domains/execution_validation/specs/EXE-008.md)).
/// Proves the Tier-2 → Tier-3 bridge accepts the EXE-009 value without conversion.
#[test]
fn pending_assertion_fits_execution_result() {
    let (spend, _) = test_spend();
    let cond: Condition<()> = Condition::AssertHeightAbsolute(AssertHeightAbsolute { height: 1 });
    let p = PendingAssertion::from_condition(&cond, &spend).unwrap();

    let ex = ExecutionResult {
        pending_assertions: vec![p.clone()],
        ..Default::default()
    };
    assert_eq!(ex.pending_assertions.len(), 1);
    assert_eq!(ex.pending_assertions[0], p);
}
