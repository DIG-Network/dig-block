//! SVL-004: Tier-1 header validation must reject **future** timestamps beyond a fixed skew window ([SPEC §5.1 Step 5](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/structural_validation/NORMATIVE.md` (SVL-004)  
//! **Spec + test plan:** `docs/requirements/domains/structural_validation/specs/SVL-004.md`  
//! **Implementation:** [`dig_block::L2BlockHeader::validate_with_dfsp_activation_at_unix`] (deterministic `now`),  
//! [`dig_block::L2BlockHeader::validate_with_dfsp_activation`] / [`dig_block::L2BlockHeader::validate`] (wall clock) — `src/types/header.rs`  
//! **Constants:** [`dig_block::MAX_FUTURE_TIMESTAMP_SECONDS`] ([BLK-005](docs/requirements/domains/block_types/specs/BLK-005.md))  
//! **Error mapping:** [`dig_block::BlockError::TimestampTooFarInFuture`] ([ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md))
//!
//! ## How these tests prove SVL-004
//!
//! Chia’s Check 26a analogue is **`timestamp > now + MAX_FUTURE_TIMESTAMP_SECONDS`** (strict `>`). We never assert on the
//! real wall clock in this file: every case calls [`L2BlockHeader::validate_with_dfsp_activation_at_unix`] with a fixed
//! `now_secs`, so the causal chain is **given synthetic now → set header.timestamp → assert Ok/Err** without flakes.
//!
//! - **`svl004_timestamp_is_now`:** `timestamp == now` ⇒ within window ⇒ `Ok` (acceptance: “now” is valid).
//! - **`svl004_timestamp_at_future_boundary`:** `timestamp == now + 300` ⇒ **not** strictly greater than the cap ⇒ `Ok`
//!   (strict-`>` boundary from the spec).
//! - **`svl004_timestamp_beyond_future_boundary`:** `timestamp == now + 301` ⇒ one second past the window ⇒
//!   [`BlockError::TimestampTooFarInFuture`] with `max_allowed == now + MAX_FUTURE_TIMESTAMP_SECONDS`.
//! - **`svl004_timestamp_far_future`:** large skew (`+3600s`) ⇒ same error variant (proves rule scales, not only +1).
//! - **`svl004_timestamp_in_past`:** `timestamp` well before `now` ⇒ `Ok` (no minimum-timestamp gate at this tier).
//! - **`svl004_max_future_window_is_300_seconds`:** documents the protocol default required by the spec acceptance row.
//!
//! **Flat test path:** `tests/test_svl_004_timestamp_future_bound.rs` per [STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md)
//! (spec’s `tests/structural_validation/…` path is not used here).
//!
//! **Tooling:** Per `docs/prompt/start.md`, prefer `npx gitnexus impact` on [`L2BlockHeader::validate_with_dfsp_activation_at_unix`]
//! before edits; SocratiCode when MCP is available.

use dig_block::{
    BlockError, Bytes32, L2BlockHeader, DFSP_ACTIVATION_HEIGHT, MAX_FUTURE_TIMESTAMP_SECONDS,
    VERSION_V1,
};

fn tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// Synthetic “wall now” shared across boundary tests — any large, non-edge u64 avoids wraparound in `now + 300`.
const T0: u64 = 1_700_000_000;

/// SVL-001/002/003-valid header at height **0** (V1, empty DFSP roots via [`L2BlockHeader::new`], cost/size zero) with caller-controlled timestamp.
fn header_with_timestamp(ts: u64) -> L2BlockHeader {
    let mut h = L2BlockHeader::new(
        0,
        0,
        tag(0x01),
        tag(0x02),
        tag(0x03),
        tag(0x04),
        tag(0x05),
        tag(0x06),
        0,
        tag(0x07),
        0,
        0,
        0,
        0,
        0,
        0,
        tag(0x08),
    );
    assert_eq!(h.version, VERSION_V1);
    h.timestamp = ts;
    h
}

/// **Test plan:** `test_timestamp_is_now` — `timestamp == now` ⇒ `Ok`.
#[test]
fn svl004_timestamp_is_now() {
    let h = header_with_timestamp(T0);
    h.validate_with_dfsp_activation_at_unix(DFSP_ACTIVATION_HEIGHT, T0)
        .expect("timestamp equal to reference now must pass");
}

/// **Test plan:** `test_timestamp_at_future_boundary` — `timestamp == now + 300` ⇒ `Ok`.
#[test]
fn svl004_timestamp_at_future_boundary() {
    let ts = T0 + MAX_FUTURE_TIMESTAMP_SECONDS;
    let h = header_with_timestamp(ts);
    h.validate_with_dfsp_activation_at_unix(DFSP_ACTIVATION_HEIGHT, T0)
        .expect("timestamp exactly at now+MAX_FUTURE must pass (strict >)");
}

/// **Test plan:** `test_timestamp_beyond_future_boundary` — `timestamp == now + 301` ⇒ `TimestampTooFarInFuture`.
#[test]
fn svl004_timestamp_beyond_future_boundary() {
    let ts = T0 + MAX_FUTURE_TIMESTAMP_SECONDS + 1;
    let h = header_with_timestamp(ts);
    let max_allowed = T0 + MAX_FUTURE_TIMESTAMP_SECONDS;
    match h
        .validate_with_dfsp_activation_at_unix(DFSP_ACTIVATION_HEIGHT, T0)
        .expect_err("one second past window must fail")
    {
        BlockError::TimestampTooFarInFuture {
            timestamp,
            max_allowed: got_max,
        } => {
            assert_eq!(timestamp, ts);
            assert_eq!(got_max, max_allowed);
        }
        e => panic!("expected TimestampTooFarInFuture, got {e:?}"),
    }
}

/// **Test plan:** `test_timestamp_far_future` — large positive skew ⇒ `TimestampTooFarInFuture`.
#[test]
fn svl004_timestamp_far_future() {
    let ts = T0 + 3600;
    let h = header_with_timestamp(ts);
    assert!(matches!(
        h.validate_with_dfsp_activation_at_unix(DFSP_ACTIVATION_HEIGHT, T0),
        Err(BlockError::TimestampTooFarInFuture { .. })
    ));
}

/// **Test plan:** `test_timestamp_in_past` — old timestamps remain valid at this tier.
#[test]
fn svl004_timestamp_in_past() {
    let h = header_with_timestamp(T0 - 600);
    h.validate_with_dfsp_activation_at_unix(DFSP_ACTIVATION_HEIGHT, T0)
        .expect("past timestamps are not rejected by SVL-004");
}

/// **Test plan:** acceptance row — default future window is five minutes (300 seconds), matching Chia Check 26a spirit.
#[test]
fn svl004_max_future_window_is_300_seconds() {
    assert_eq!(MAX_FUTURE_TIMESTAMP_SECONDS, 300);
}
