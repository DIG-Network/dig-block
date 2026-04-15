//! BLD-003: [`dig_block::BlockBuilder::add_slash_proposal`] — per-block slash **count** cap and per-payload **byte** cap
//! ([SPEC §6.3](docs/resources/SPEC.md),
//! [NORMATIVE — BLD-003](docs/requirements/domains/block_production/NORMATIVE.md#bld-003-add_slash_proposal-with-limits)).
//!
//! **Authoritative spec:** `docs/requirements/domains/block_production/specs/BLD-003.md` (acceptance criteria + test plan).
//! **Flat test path:** `tests/test_bld_003_add_slash_proposal_limits.rs` (STR-002; not `tests/block_production/…` from spec prose).
//!
//! ## How these tests prove BLD-003
//!
//! - **`bld003_add_slash_proposal_ok`:** Matches test-plan `test_add_slash_proposal_ok` — a payload strictly inside both
//!   [`dig_block::MAX_SLASH_PROPOSALS_PER_BLOCK`] and [`dig_block::MAX_SLASH_PROPOSAL_PAYLOAD_BYTES`] must append to
//!   `slash_proposal_payloads` and return `Ok`.
//! - **`bld003_add_slash_proposal_at_count_limit`:** Filling the **last** legal slot (`len == max - 1` then one `Ok`)
//!   proves the count rule is `len < MAX` before push (spec uses `>=` guard so exactly `MAX` rows are allowed).
//! - **`bld003_add_slash_proposal_exceeds_count`:** `Err(BuilderError::TooManySlashProposals)` when `len == MAX` and another
//!   add is attempted proves the cap cannot be exceeded.
//! - **`bld003_slash_proposal_at_size_limit`:** Payload **length ==** `MAX_SLASH_PROPOSAL_PAYLOAD_BYTES` must `Ok`
//!   (strict `>` in implementation per acceptance “within limit” boundary).
//! - **`bld003_slash_proposal_exceeds_size`:** `len == max + 1` ⇒ `SlashProposalTooLarge` with structured `size` / `max`.
//! - **`bld003_rejected_proposal_no_state_change`:** After `Err`, `slash_proposal_payloads` and unrelated builder fields
//!   match a pre-call snapshot (no partial push).
//! - **`bld003_count_error_precedes_size_error`:** Implementation note — when **both** count and payload size would
//!   violate limits, the **count** check runs first, so a full builder still yields `TooManySlashProposals`, not
//!   `SlashProposalTooLarge` (ordering obligation in BLD-003 spec).
//!
//! **Tooling:** Repomix packs (`.repomix/pack-src.xml`, `.repomix/pack-block-production-reqs.xml`) regenerated before
//! coding. `npx gitnexus impact BlockBuilder` → **LOW** / zero upstream callers. SocratiCode MCP was not configured.

use dig_block::{
    BlockBuilder, BuilderError, Bytes32, MAX_SLASH_PROPOSALS_PER_BLOCK,
    MAX_SLASH_PROPOSAL_PAYLOAD_BYTES,
};

fn mk_builder() -> BlockBuilder {
    BlockBuilder::new(
        1,
        0,
        Bytes32::new([0x01; 32]),
        1,
        Bytes32::new([0x02; 32]),
        0,
    )
}

/// **Test plan:** `test_add_slash_proposal_ok`
#[test]
fn bld003_add_slash_proposal_ok() {
    let mut b = mk_builder();
    let payload = vec![0xde, 0xad];
    b.add_slash_proposal(payload.clone())
        .expect("within limits");
    assert_eq!(b.slash_proposal_payloads.len(), 1);
    assert_eq!(b.slash_proposal_payloads[0], payload);
}

/// **Test plan:** `test_add_slash_proposal_at_count_limit`
#[test]
fn bld003_add_slash_proposal_at_count_limit() {
    let mut b = mk_builder();
    let max = MAX_SLASH_PROPOSALS_PER_BLOCK as usize;
    assert!(max >= 1, "fixture assumes at least one slash slot");
    for i in 0..max - 1 {
        b.add_slash_proposal(vec![i as u8]).expect("below cap");
    }
    b.add_slash_proposal(vec![0xff]).expect("fills final slot");
    assert_eq!(b.slash_proposal_payloads.len(), max);
}

/// **Test plan:** `test_add_slash_proposal_exceeds_count`
#[test]
fn bld003_add_slash_proposal_exceeds_count() {
    let mut b = mk_builder();
    let max = MAX_SLASH_PROPOSALS_PER_BLOCK as usize;
    for i in 0..max {
        b.add_slash_proposal(vec![i as u8]).unwrap();
    }
    let err = b
        .add_slash_proposal(vec![0xcc])
        .expect_err("must reject when already at MAX slash proposals");
    match err {
        BuilderError::TooManySlashProposals { max: m } => {
            assert_eq!(m, MAX_SLASH_PROPOSALS_PER_BLOCK);
        }
        e => panic!("unexpected error: {e:?}"),
    }
    assert_eq!(b.slash_proposal_payloads.len(), max);
}

/// **Test plan:** `test_slash_proposal_at_size_limit`
#[test]
fn bld003_slash_proposal_at_size_limit() {
    let mut b = mk_builder();
    let payload = vec![0x5au8; MAX_SLASH_PROPOSAL_PAYLOAD_BYTES as usize];
    assert_eq!(payload.len(), MAX_SLASH_PROPOSAL_PAYLOAD_BYTES as usize);
    b.add_slash_proposal(payload.clone())
        .expect("exactly at byte cap is Ok");
    assert_eq!(b.slash_proposal_payloads.len(), 1);
    assert_eq!(b.slash_proposal_payloads[0].len(), payload.len());
}

/// **Test plan:** `test_slash_proposal_exceeds_size`
#[test]
fn bld003_slash_proposal_exceeds_size() {
    let mut b = mk_builder();
    let n = MAX_SLASH_PROPOSAL_PAYLOAD_BYTES as usize + 1;
    let payload = vec![0x3cu8; n];
    let err = b
        .add_slash_proposal(payload)
        .expect_err("one byte over cap must fail");
    match err {
        BuilderError::SlashProposalTooLarge { size, max } => {
            assert_eq!(max, MAX_SLASH_PROPOSAL_PAYLOAD_BYTES);
            assert_eq!(size as usize, n);
        }
        e => panic!("unexpected error: {e:?}"),
    }
    assert!(b.slash_proposal_payloads.is_empty());
}

/// **Test plan:** `test_rejected_proposal_no_state_change`
#[test]
fn bld003_rejected_proposal_no_state_change() {
    let mut b = mk_builder();
    b.add_slash_proposal(vec![1, 2, 3]).unwrap();
    let snap_slash = b.slash_proposal_payloads.clone();
    let snap_spend_len = b.spend_bundles.len();
    let snap_cost = b.total_cost;

    let _ = b.add_slash_proposal(vec![0u8; MAX_SLASH_PROPOSAL_PAYLOAD_BYTES as usize + 1]);

    assert_eq!(b.slash_proposal_payloads, snap_slash);
    assert_eq!(b.spend_bundles.len(), snap_spend_len);
    assert_eq!(b.total_cost, snap_cost);
}

/// **Implementation note:** count check before size check when both would fail.
#[test]
fn bld003_count_error_precedes_size_error() {
    let mut b = mk_builder();
    let max = MAX_SLASH_PROPOSALS_PER_BLOCK as usize;
    for i in 0..max {
        b.add_slash_proposal(vec![i as u8]).unwrap();
    }
    let err = b
        .add_slash_proposal(vec![0u8; MAX_SLASH_PROPOSAL_PAYLOAD_BYTES as usize + 1])
        .expect_err("full count + oversized payload still hits count gate first");
    assert!(
        matches!(err, BuilderError::TooManySlashProposals { .. }),
        "expected TooManySlashProposals, got {err:?}"
    );
}
