//! SVL-002: Before DFSP activation height, **all five DFSP layer roots** must be [`EMPTY_ROOT`]
//! ([SPEC §5.1 Step 2](docs/resources/SPEC.md), [NORMATIVE — SVL-002](docs/requirements/domains/structural_validation/NORMATIVE.md#svl-002-header-dfsp-root-pre-activation-check)).
//!
//! **Spec + test plan:** `docs/requirements/domains/structural_validation/specs/SVL-002.md`  
//! **Implementation:** [`dig_block::L2BlockHeader::validate_with_dfsp_activation`], [`dig_block::L2BlockHeader::validate`] — `src/types/header.rs`  
//! **Error mapping:** [`dig_block::BlockError::InvalidData`] ([ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md)) with the fixed diagnostic string from the spec’s reference algorithm.
//!
//! ## How these tests prove SVL-002
//!
//! - **`svl002_all_empty_roots_pre_activation`:** Height **strictly below** a finite activation height, all five DFSP roots left at
//!   [`EMPTY_ROOT`] (via [`L2BlockHeader::new`]) ⇒ [`validate_with_dfsp_activation`] returns `Ok` (matches test plan `test_all_empty_roots_pre_activation`).
//! - **`svl002_*_non_empty_pre_activation`:** Five tests, each poisoning **one** DFSP root with a distinct non-[`EMPTY_ROOT`] [`Bytes32`] while
//!   keeping [`VERSION_V1`] and height `<` activation ⇒ each returns [`BlockError::InvalidData`] (proves **every** root is enforced individually).
//! - **`svl002_non_empty_roots_post_activation`:** At activation height with [`VERSION_V2`] and all five roots non-empty ⇒ `Ok` (post-activation
//!   roots are not constrained by this step).
//! - **`svl002_validate_with_sentinel_requires_empty_roots`:** Production [`validate`] uses [`DFSP_ACTIVATION_HEIGHT`]; with the crate default
//!   `u64::MAX` sentinel, every finite height is still “pre-activation” for this check (`height < u64::MAX`), so a poisoned root must fail
//!   [`validate`] as well—proving [`validate`] chains the same invariant as `validate_with_dfsp_activation(DFSP_ACTIVATION_HEIGHT)`.
//!
//! **Test file location:** Flat `tests/` per [STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md). The SVL-002 spec’s
//! `tests/structural_validation/…` path is **not** used here.  
//! **SocratiCode:** MCP not available in this environment — search/graph steps from `docs/prompt/start.md` were skipped.  
//! **GitNexus:** `npx gitnexus status` failed during tool install in this session; blast radius was taken manually: callers of
//! [`validate_with_dfsp_activation`] / [`validate`] are this test crate and future SVL wiring only.

use dig_block::{BlockError, Bytes32, L2BlockHeader, EMPTY_ROOT, VERSION_V1, VERSION_V2};

/// Simulated DFSP fork height (finite) so pre/post activation scenarios are reachable in tests.
const TEST_DFSP_ACTIVATION: u64 = 10_000;

/// Canonical [`BlockError::InvalidData`] message for SVL-002 (must match `specs/SVL-002.md` reference snippet).
const SVL002_MSG: &str = "DFSP root must be EMPTY_ROOT before activation";

fn tag_byte(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// Minimal header: SVL-002 only cares about `height`, `version`, and the five DFSP roots; other fields are inert placeholders from [`L2BlockHeader::new`].
fn minimal_header_pre_activation() -> L2BlockHeader {
    let height = TEST_DFSP_ACTIVATION - 1;
    L2BlockHeader::new(
        height,
        0,
        tag_byte(0x01),
        tag_byte(0x02),
        tag_byte(0x03),
        tag_byte(0x04),
        tag_byte(0x05),
        tag_byte(0x06),
        100,
        tag_byte(0x07),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        tag_byte(0x08),
    )
}

fn assert_invalid_dfsp(e: BlockError) {
    match e {
        BlockError::InvalidData(msg) => assert_eq!(msg, SVL002_MSG),
        other => panic!("expected InvalidData({SVL002_MSG:?}), got {other:?}"),
    }
}

/// **Test plan:** `test_all_empty_roots_pre_activation`
#[test]
fn svl002_all_empty_roots_pre_activation() {
    let h = minimal_header_pre_activation();
    assert_eq!(h.version, VERSION_V1);
    assert_eq!(h.collateral_registry_root, EMPTY_ROOT);
    h.validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect("all EMPTY_ROOT below activation");
}

/// **Test plan:** `test_collateral_root_non_empty_pre_activation`
#[test]
fn svl002_collateral_root_non_empty_pre_activation() {
    let mut h = minimal_header_pre_activation();
    h.collateral_registry_root = tag_byte(0xC1);
    let e = h
        .validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect_err("non-empty collateral_registry_root must fail");
    assert_invalid_dfsp(e);
}

/// **Test plan:** `test_cid_state_root_non_empty_pre_activation`
#[test]
fn svl002_cid_state_root_non_empty_pre_activation() {
    let mut h = minimal_header_pre_activation();
    h.cid_state_root = tag_byte(0xC2);
    let e = h
        .validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect_err("non-empty cid_state_root must fail");
    assert_invalid_dfsp(e);
}

/// **Test plan:** `test_node_registry_non_empty_pre_activation`
#[test]
fn svl002_node_registry_non_empty_pre_activation() {
    let mut h = minimal_header_pre_activation();
    h.node_registry_root = tag_byte(0xC3);
    let e = h
        .validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect_err("non-empty node_registry_root must fail");
    assert_invalid_dfsp(e);
}

/// **Test plan:** `test_namespace_update_non_empty_pre_activation`
#[test]
fn svl002_namespace_update_non_empty_pre_activation() {
    let mut h = minimal_header_pre_activation();
    h.namespace_update_root = tag_byte(0xC4);
    let e = h
        .validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect_err("non-empty namespace_update_root must fail");
    assert_invalid_dfsp(e);
}

/// **Test plan:** `test_dfsp_finalize_non_empty_pre_activation`
#[test]
fn svl002_dfsp_finalize_non_empty_pre_activation() {
    let mut h = minimal_header_pre_activation();
    h.dfsp_finalize_commitment_root = tag_byte(0xC5);
    let e = h
        .validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect_err("non-empty dfsp_finalize_commitment_root must fail");
    assert_invalid_dfsp(e);
}

/// **Test plan:** `test_non_empty_roots_post_activation`
#[test]
fn svl002_non_empty_roots_post_activation() {
    let mut h = L2BlockHeader::new(
        TEST_DFSP_ACTIVATION,
        0,
        tag_byte(0x01),
        tag_byte(0x02),
        tag_byte(0x03),
        tag_byte(0x04),
        tag_byte(0x05),
        tag_byte(0x06),
        100,
        tag_byte(0x07),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        tag_byte(0x08),
    );
    h.version = VERSION_V2;
    h.collateral_registry_root = tag_byte(0xD1);
    h.cid_state_root = tag_byte(0xD2);
    h.node_registry_root = tag_byte(0xD3);
    h.namespace_update_root = tag_byte(0xD4);
    h.dfsp_finalize_commitment_root = tag_byte(0xD5);
    h.validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect("non-empty DFSP roots allowed at or after activation");
}

/// Default [`DFSP_ACTIVATION_HEIGHT`] keeps DFSP disabled (`u64::MAX`); SVL-002 still requires empty roots for all finite heights.
#[test]
fn svl002_validate_with_sentinel_requires_empty_roots() {
    let mut h = minimal_header_pre_activation();
    h.cid_state_root = tag_byte(0xE0);
    let e = h
        .validate()
        .expect_err("sentinel activation: finite height is still pre-DFSP");
    assert_invalid_dfsp(e);
}
