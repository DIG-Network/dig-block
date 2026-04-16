//! SVL-003: Tier-1 header validation must bound **declared** aggregate CLVM cost and serialized block size
//! ([SPEC §5.1 Steps 3–4](docs/resources/SPEC.md), [NORMATIVE — SVL-003](docs/requirements/domains/structural_validation/NORMATIVE.md#svl-003-header-cost-and-size-checks)).
//!
//! **Spec + test plan:** `docs/requirements/domains/structural_validation/specs/SVL-003.md`  
//! **Implementation:** [`dig_block::L2BlockHeader::validate_with_dfsp_activation`], [`dig_block::L2BlockHeader::validate`] — `src/types/header.rs`  
//! **Limits:** [`dig_block::MAX_COST_PER_BLOCK`], [`dig_block::MAX_BLOCK_SIZE`] ([BLK-005](docs/requirements/domains/block_types/specs/BLK-005.md)).
//!
//! ## How these tests prove SVL-003
//!
//! Each case mutates only `total_cost` and/or `block_size` on an otherwise SVL-001/SVL-002-valid header (height `0`,
//! [`dig_block::VERSION_V1`], DFSP roots empty via [`L2BlockHeader::new`], sentinel pre-activation semantics). Assertions
//! match the spec’s **strict greater-than** rule: values **equal** to the protocol maximum are accepted; **one** unit
//! over the maximum is rejected with the correct [`dig_block::BlockError`] variant. The **cost-before-size** ordering is
//! proven by `svl003_cost_exceeds_size_ok`: when both fields are out of range, `CostExceeded` wins so the implementation
//! cannot have reordered checks silently.
//!
//! **Flat test path:** `tests/test_svl_003_cost_and_size_checks.rs` per [STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md)
//! (the SVL-003 spec historically referenced `tests/structural_validation/…`; this repo keeps one file per requirement at `tests/` root).
//!
//! **Tooling:** Per `docs/prompt/start.md`, run `npx gitnexus status` / `analyze` before changing [`L2BlockHeader::validate_with_dfsp_activation`]
//! and use SocratiCode `codebase_search` when the MCP is wired. This suite only **consumes** the public validation API.

use dig_block::{
    BlockError, Bytes32, L2BlockHeader, MAX_BLOCK_SIZE, MAX_COST_PER_BLOCK, VERSION_V1,
};

fn tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// Header that passes SVL-001 (version for height 0) and SVL-002 (empty DFSP roots); only cost/size are varied per test.
fn base_header(total_cost: u64, block_size: u32) -> L2BlockHeader {
    L2BlockHeader::new(
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
        total_cost,
        0,
        0,
        0,
        block_size,
        tag(0x08),
    )
}

/// **Test plan:** `test_cost_at_limit` — boundary: `total_cost == MAX_COST_PER_BLOCK`.
#[test]
fn svl003_cost_at_limit() {
    let h = base_header(MAX_COST_PER_BLOCK, 1);
    assert_eq!(h.version, VERSION_V1);
    h.validate().expect("cost exactly at limit must validate");
}

/// **Test plan:** `test_cost_exceeds_limit`.
#[test]
fn svl003_cost_exceeds_limit() {
    let h = base_header(MAX_COST_PER_BLOCK + 1, 1);
    match h.validate().expect_err("cost one over limit must fail") {
        BlockError::CostExceeded { cost, max } => {
            assert_eq!(cost, MAX_COST_PER_BLOCK + 1);
            assert_eq!(max, MAX_COST_PER_BLOCK);
        }
        e => panic!("expected CostExceeded, got {e:?}"),
    }
}

/// **Test plan:** `test_size_at_limit` — boundary: `block_size == MAX_BLOCK_SIZE`.
#[test]
fn svl003_size_at_limit() {
    let h = base_header(0, MAX_BLOCK_SIZE);
    h.validate().expect("size exactly at limit must validate");
}

/// **Test plan:** `test_size_exceeds_limit`.
#[test]
fn svl003_size_exceeds_limit() {
    let h = base_header(0, MAX_BLOCK_SIZE + 1);
    match h.validate().expect_err("size one over limit must fail") {
        BlockError::TooLarge { size, max } => {
            assert_eq!(size, MAX_BLOCK_SIZE + 1);
            assert_eq!(max, MAX_BLOCK_SIZE);
        }
        e => panic!("expected TooLarge, got {e:?}"),
    }
}

/// **Test plan:** `test_both_within_limits`.
#[test]
fn svl003_both_within_limits() {
    let h = base_header(1_000, 10_000);
    h.validate().expect("well under both caps");
}

/// **Test plan:** `test_cost_exceeds_size_ok` — proves cost check runs before size when both violate limits.
#[test]
fn svl003_cost_exceeds_size_ok() {
    let h = base_header(MAX_COST_PER_BLOCK + 1, MAX_BLOCK_SIZE + 1);
    match h.validate().expect_err("must fail on cost first") {
        BlockError::CostExceeded { .. } => {}
        e => panic!("expected CostExceeded when both exceed, got {e:?}"),
    }
}

/// **Test plan:** `test_cost_ok_size_exceeds` — cost at limit, size over limit ⇒ `TooLarge`.
#[test]
fn svl003_cost_ok_size_exceeds() {
    let h = base_header(MAX_COST_PER_BLOCK, MAX_BLOCK_SIZE + 1);
    match h.validate().expect_err("oversized block must fail") {
        BlockError::TooLarge { .. } => {}
        e => panic!("expected TooLarge, got {e:?}"),
    }
}
