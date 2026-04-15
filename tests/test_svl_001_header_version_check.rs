//! SVL-001: Header **version** must match height relative to DFSP activation ([SPEC §5.1 Step 1](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/structural_validation/NORMATIVE.md` (SVL-001)  
//! **Spec + test plan:** `docs/requirements/domains/structural_validation/specs/SVL-001.md`  
//! **Implementation:** [`dig_block::L2BlockHeader::validate`], [`dig_block::L2BlockHeader::validate_with_dfsp_activation`] — `src/types/header.rs`  
//! **Error mapping:** [`dig_block::BlockError::InvalidVersion`] ([ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md))  
//! **BLK-007:** [`L2BlockHeader::protocol_version_for_height_with_activation`] defines the same fork rule exercised here.
//!
//! ## How these tests prove SVL-001
//!
//! - **`svl001_v1_below_activation_ok`:** Height strictly **before** a finite activation ⇒ expected [`VERSION_V1`]; a V1 header passes
//!   [`validate_with_dfsp_activation`](L2BlockHeader::validate_with_dfsp_activation).
//! - **`svl001_v2_at_activation_ok` / `svl001_v2_above_activation_ok`:** At or after activation ⇒ expected [`VERSION_V2`]; headers with
//!   `version == VERSION_V2` pass (we set `version` explicitly because default [`DFSP_ACTIVATION_HEIGHT`] is `u64::MAX`, so [`L2BlockHeader::new`]
//!   always emits V1 in this test binary).
//! - **`svl001_v2_below_activation_rejected`:** V2 when height `<` activation ⇒ [`BlockError::InvalidVersion`].
//! - **`svl001_v1_at_activation_rejected` / `svl001_v1_above_activation_rejected`:** V1 when height `>=` activation ⇒ invalid (expected V2).
//! - **`svl001_sentinel_max_always_v1`:** With the crate’s [`DFSP_ACTIVATION_HEIGHT`] (`u64::MAX`), [`validate`] always expects V1;
//!   V2 must fail regardless of height.
//! - **`svl001_validate_delegates_to_constant`:** [`validate`] agrees with `validate_with_dfsp_activation(DFSP_ACTIVATION_HEIGHT)` for a valid header.
//!
//! **Test file location:** Flat `tests/` ([STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md)); the SVL-001 spec mentions
//! `tests/structural_validation/…` — this repository keeps one integration-test crate root without subdirectories.  
//! **SocratiCode:** Not used here (no MCP).  
//! **Tools:** Repomix/gitnexus per `docs/prompt/start.md` before implementation.

use dig_block::{
    BlockError, Bytes32, L2BlockHeader, DFSP_ACTIVATION_HEIGHT, VERSION_V1, VERSION_V2,
};

/// Simulated DFSP fork height for scenarios where activation is **finite** (unlike default `u64::MAX`).
const TEST_DFSP_ACTIVATION: u64 = 10_000;

fn tag_byte(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// Minimal header for SVL-001: only `height` / `version` matter for this requirement; other fields are inert placeholders.
fn minimal_header_at_height(height: u64) -> L2BlockHeader {
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

/// **Test plan:** `test_v1_below_activation` — V1 header with height `<` activation ⇒ Ok.
#[test]
fn svl001_v1_below_activation_ok() {
    let h = minimal_header_at_height(TEST_DFSP_ACTIVATION - 1);
    assert_eq!(h.version, VERSION_V1);
    h.validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect("V1 below fork must validate");
}

/// **Test plan:** `test_v2_at_activation` — V2 at height `==` activation ⇒ Ok.
#[test]
fn svl001_v2_at_activation_ok() {
    let mut h = minimal_header_at_height(TEST_DFSP_ACTIVATION);
    h.version = VERSION_V2;
    h.validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect("V2 at fork height must validate");
}

/// **Test plan:** `test_v2_above_activation` — V2 with height `>` activation ⇒ Ok.
#[test]
fn svl001_v2_above_activation_ok() {
    let mut h = minimal_header_at_height(TEST_DFSP_ACTIVATION + 1);
    h.version = VERSION_V2;
    h.validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect("V2 above fork must validate");
}

/// **Test plan:** `test_v2_below_activation_rejected`.
#[test]
fn svl001_v2_below_activation_rejected() {
    let mut h = minimal_header_at_height(TEST_DFSP_ACTIVATION - 1);
    h.version = VERSION_V2;
    let e = h
        .validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect_err("V2 before fork must be rejected");
    assert!(matches!(
        e,
        BlockError::InvalidVersion {
            expected: VERSION_V1,
            actual: VERSION_V2
        }
    ));
}

/// **Test plan:** `test_v1_at_activation_rejected`.
#[test]
fn svl001_v1_at_activation_rejected() {
    let h = minimal_header_at_height(TEST_DFSP_ACTIVATION);
    assert_eq!(h.version, VERSION_V1);
    let e = h
        .validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect_err("V1 at fork height must be rejected");
    assert!(matches!(
        e,
        BlockError::InvalidVersion {
            expected: VERSION_V2,
            actual: VERSION_V1
        }
    ));
}

/// **Test plan:** `test_v1_above_activation_rejected`.
#[test]
fn svl001_v1_above_activation_rejected() {
    let h = minimal_header_at_height(TEST_DFSP_ACTIVATION + 100);
    assert_eq!(h.version, VERSION_V1);
    let e = h
        .validate_with_dfsp_activation(TEST_DFSP_ACTIVATION)
        .expect_err("V1 after fork must be rejected");
    assert!(matches!(
        e,
        BlockError::InvalidVersion {
            expected: VERSION_V2,
            actual: VERSION_V1
        }
    ));
}

/// **Test plan:** `test_sentinel_max_always_v1` — default constant disables DFSP ⇒ always expect V1.
#[test]
fn svl001_sentinel_max_always_v1() {
    let h = minimal_header_at_height(0);
    h.validate()
        .expect("V1 must pass with DFSP_ACTIVATION_HEIGHT = u64::MAX");

    let mut h2 = minimal_header_at_height(u64::MAX - 1);
    h2.version = VERSION_V2;
    let e = h2
        .validate()
        .expect_err("V2 must never validate while sentinel disables DFSP");
    assert!(matches!(
        e,
        BlockError::InvalidVersion {
            expected: VERSION_V1,
            actual: VERSION_V2
        }
    ));
}

/// **Test plan:** `validate()` uses the same rule as `validate_with_dfsp_activation(DFSP_ACTIVATION_HEIGHT)`.
#[test]
fn svl001_validate_delegates_to_constant() {
    let h = minimal_header_at_height(42);
    h.validate().expect("ok");
    h.validate_with_dfsp_activation(DFSP_ACTIVATION_HEIGHT)
        .expect("same as validate()");
}
