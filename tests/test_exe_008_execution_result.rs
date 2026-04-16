//! EXE-008: `ExecutionResult` output type — Tier-2 → Tier-3 bridge ([SPEC §7.4.7](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/execution_validation/NORMATIVE.md` (EXE-008)
//! **Spec:** `docs/requirements/domains/execution_validation/specs/EXE-008.md`
//!
//! ## What this proves
//!
//! - **Field shape:** `ExecutionResult` exposes six public fields in the exact shape NORMATIVE
//!   mandates: `additions: Vec<Coin>`, `removals: Vec<Bytes32>`,
//!   `pending_assertions: Vec<PendingAssertion>`, `total_cost: Cost`, `total_fees: u64`,
//!   `receipts: Vec<Receipt>`. A struct with the wrong shape fails compile here.
//!
//! - **Empty construction:** `ExecutionResult::default()` yields every collection empty and both
//!   scalars `0`. This matches the "empty block produces empty ExecutionResult" case from the
//!   EXE-008 test plan and is what the EXE-001 pipeline produces when no spend bundles are present.
//!
//! - **Populated construction:** All six fields can be constructed, read, and iterated — proves
//!   they are `pub` and hold the correct types (compile-time surface test).
//!
//! - **Serde:** `ExecutionResult` derives `Serialize`/`Deserialize` and round-trips through
//!   bincode ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)), so Tier-2
//!   outputs can be cached / replayed.
//!
//! - **Tier-2 → Tier-3 bridge:** The struct can be passed by reference (`&ExecutionResult`) to
//!   a function, matching the [`L2Block::validate_state`] signature promised by
//!   [STV-001](docs/requirements/domains/state_validation/specs/STV-001.md).
//!
//! ## How this satisfies EXE-008
//!
//! Each acceptance-criterion bullet in the spec test plan maps to one test below.

use chia_protocol::{Bytes32, Coin};

use dig_block::{
    AssertionKind, Cost, ExecutionResult, PendingAssertion, Receipt, ReceiptStatus,
};

/// Build a sample [`PendingAssertion`] via the EXE-009 enum without touching CLVM.
fn sample_assertion(coin_id: Bytes32, height: u64) -> PendingAssertion {
    PendingAssertion {
        kind: AssertionKind::HeightAbsolute(height),
        coin_id,
    }
}

/// **EXE-008 test plan: `empty_block_result`** — Default instance has every field at its identity.
/// Matches what EXE-001 produces for a block with zero spend bundles.
#[test]
fn default_is_empty_and_zero() {
    let ex = ExecutionResult::default();
    assert!(ex.additions.is_empty(), "additions must be empty");
    assert!(ex.removals.is_empty(), "removals must be empty");
    assert!(
        ex.pending_assertions.is_empty(),
        "pending_assertions must be empty"
    );
    assert_eq!(ex.total_cost, 0, "total_cost must be 0");
    assert_eq!(ex.total_fees, 0, "total_fees must be 0");
    assert!(ex.receipts.is_empty(), "receipts must be empty");
}

/// **EXE-008 test plan: `all_fields_populated`** — Every field is `pub` and accepts the
/// NORMATIVE-specified type. Construction via struct literal (not a constructor) proves field
/// visibility and type shape.
#[test]
fn all_fields_populated_and_public() {
    let parent = Bytes32::new([0xaa; 32]);
    let puzzle = Bytes32::new([0xbb; 32]);
    let addition = Coin::new(parent, puzzle, 100);
    let removal = Bytes32::new([0xcc; 32]);
    let assertion = sample_assertion(removal, 42);
    let receipt = Receipt::new(
        Bytes32::new([0xdd; 32]),
        1,
        0,
        ReceiptStatus::Success,
        10,
        Bytes32::new([0xee; 32]),
        10,
    );

    let ex = ExecutionResult {
        additions: vec![addition],
        removals: vec![removal],
        pending_assertions: vec![assertion.clone()],
        total_cost: 1_000 as Cost,
        total_fees: 10,
        receipts: vec![receipt.clone()],
    };

    // additions: Vec<Coin>
    assert_eq!(ex.additions.len(), 1);
    assert_eq!(ex.additions[0].amount, 100);
    assert_eq!(ex.additions[0].puzzle_hash, puzzle);
    assert_eq!(ex.additions[0].parent_coin_info, parent);

    // removals: Vec<Bytes32>
    assert_eq!(ex.removals, vec![removal]);

    // pending_assertions: Vec<PendingAssertion>
    assert_eq!(ex.pending_assertions, vec![assertion]);

    // total_cost: Cost (alias of u64)
    assert_eq!(ex.total_cost, 1_000);

    // total_fees: u64
    assert_eq!(ex.total_fees, 10);

    // receipts: Vec<Receipt>
    assert_eq!(ex.receipts.len(), 1);
    assert_eq!(ex.receipts[0].tx_id, receipt.tx_id);
}

/// **EXE-008 test plan: `additions_match_create_coin`** — The `additions` field carries concrete
/// [`Coin`] values (parent, puzzle_hash, amount). This is the exact shape `dig_clvm::SpendResult`
/// returns; EXE-001 will forward each addition as-is.
#[test]
fn additions_are_concrete_coins() {
    let a = Coin::new(Bytes32::new([1; 32]), Bytes32::new([2; 32]), 50);
    let b = Coin::new(Bytes32::new([3; 32]), Bytes32::new([4; 32]), 200);
    let ex = ExecutionResult {
        additions: vec![a, b],
        ..Default::default()
    };
    assert_eq!(ex.additions[0].amount + ex.additions[1].amount, 250);
}

/// **EXE-008 test plan: `removals_match_spends`** — `removals` carries coin IDs (32-byte hashes),
/// not full `Coin` records. STV-002 looks each ID up in `CoinLookup`, so the compact form is sufficient.
#[test]
fn removals_are_coin_ids() {
    let a = Bytes32::new([0x11; 32]);
    let b = Bytes32::new([0x22; 32]);
    let ex = ExecutionResult {
        removals: vec![a, b],
        ..Default::default()
    };
    assert_eq!(ex.removals.len(), 2);
    assert_eq!(ex.removals[0], a);
    assert_eq!(ex.removals[1], b);
}

/// **EXE-008 test plan: `pending_assertions_deferred`** — `PendingAssertion` values collected in
/// Tier 2 (EXE-004 / EXE-009) flow through unchanged for STV-005 evaluation.
#[test]
fn pending_assertions_carry_kind_and_coin_id() {
    let coin_id = Bytes32::new([0xab; 32]);
    let abs_height = sample_assertion(coin_id, 100);
    let rel_seconds = PendingAssertion {
        kind: AssertionKind::SecondsRelative(60),
        coin_id,
    };

    let ex = ExecutionResult {
        pending_assertions: vec![abs_height.clone(), rel_seconds.clone()],
        ..Default::default()
    };

    assert_eq!(ex.pending_assertions.len(), 2);
    assert!(matches!(
        ex.pending_assertions[0].kind,
        AssertionKind::HeightAbsolute(100)
    ));
    assert!(matches!(
        ex.pending_assertions[1].kind,
        AssertionKind::SecondsRelative(60)
    ));
    assert_eq!(ex.pending_assertions[0].coin_id, coin_id);
}

/// **EXE-008 test plan: `receipts_per_bundle`** — The `receipts` vector holds one [`Receipt`] per
/// included [`chia_protocol::SpendBundle`]; EXE-001 pushes exactly once per bundle so length
/// matches the block's `spend_bundle_count`.
#[test]
fn receipts_length_matches_bundle_count() {
    // Simulate 3 bundles -> 3 receipts
    let mkrc = |i: u32| {
        Receipt::new(
            Bytes32::new([i as u8; 32]),
            1,
            i,
            ReceiptStatus::Success,
            i as u64,
            Bytes32::new([0; 32]),
            i as u64,
        )
    };
    let ex = ExecutionResult {
        receipts: vec![mkrc(0), mkrc(1), mkrc(2)],
        ..Default::default()
    };
    assert_eq!(ex.receipts.len(), 3);
    for (i, r) in ex.receipts.iter().enumerate() {
        assert_eq!(r.tx_index, i as u32);
    }
}

/// **EXE-008 acceptance:** Total cost and fee are independent `Cost`/`u64` scalars, summed by
/// EXE-001 across bundles.
#[test]
fn totals_are_scalars() {
    let ex = ExecutionResult {
        total_cost: 123_456_789 as Cost,
        total_fees: 42,
        ..Default::default()
    };
    assert_eq!(ex.total_cost, 123_456_789);
    assert_eq!(ex.total_fees, 42);
}

/// **EXE-008 acceptance:** `ExecutionResult` round-trips through bincode (SER-001 / SER-005).
/// Proves Tier-2 outputs can be cached on disk or sent over the wire without data loss.
#[test]
fn bincode_roundtrip() {
    let coin = Coin::new(Bytes32::new([9; 32]), Bytes32::new([8; 32]), 7);
    let ex = ExecutionResult {
        additions: vec![coin],
        removals: vec![Bytes32::new([6; 32])],
        pending_assertions: vec![sample_assertion(Bytes32::new([5; 32]), 99)],
        total_cost: 555,
        total_fees: 33,
        receipts: vec![Receipt::new(
            Bytes32::new([4; 32]),
            1,
            0,
            ReceiptStatus::Success,
            33,
            Bytes32::new([3; 32]),
            33,
        )],
    };
    let bytes = bincode::serialize(&ex).expect("encode");
    let decoded: ExecutionResult = bincode::deserialize(&bytes).expect("decode");
    assert_eq!(ex, decoded);
}

/// **EXE-008 bridge:** `&ExecutionResult` can be passed to a consumer (simulates
/// `validate_state(&ExecutionResult, ...)` in STV-001). Compile-time check.
#[test]
fn can_be_borrowed_for_tier3() {
    fn consume(e: &ExecutionResult) -> usize {
        e.additions.len() + e.removals.len() + e.pending_assertions.len() + e.receipts.len()
    }
    let ex = ExecutionResult::default();
    assert_eq!(consume(&ex), 0);
}
