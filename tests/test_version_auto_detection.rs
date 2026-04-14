//! BLK-007: Protocol version derived from block height and DFSP activation height.
//!
//! **Authoritative spec:** `docs/requirements/domains/block_types/specs/BLK-007.md`
//! **Normative:** `docs/requirements/domains/block_types/NORMATIVE.md` (BLK-007)
//! **Wire semantics:** `docs/resources/SPEC.md` §2.2 (version rules)
//!
//! Implements the BLK-007 **Test Plan**. [`L2BlockHeader::protocol_version_for_height_with_activation`]
//! exercises fork boundaries with arbitrary activation heights; [`L2BlockHeader::protocol_version_for_height`]
//! and constructors use the live [`dig_block::DFSP_ACTIVATION_HEIGHT`] constant.

use dig_block::{
    Bytes32, Cost, L2BlockHeader, DFSP_ACTIVATION_HEIGHT, EMPTY_ROOT, VERSION_V1, VERSION_V2,
};

fn tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// Minimal [`L2BlockHeader::new`] at `height` for version-focused assertions (other fields inert).
fn header_new_at(height: u64) -> L2BlockHeader {
    L2BlockHeader::new(
        height,
        0,
        tag(0x01),
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        0,
        tag(0x02),
        0,
        0,
        0 as Cost,
        0,
        0,
        0,
        0,
        EMPTY_ROOT,
    )
}

/// **Test plan:** `test_version_at_height_zero` — height 0 maps to [`VERSION_V1`] on the live chain config.
#[test]
fn test_version_at_height_zero() {
    let h = header_new_at(0);
    assert_eq!(h.height, 0);
    assert_eq!(h.version, VERSION_V1);
    assert_eq!(h.version, L2BlockHeader::protocol_version_for_height(0));
}

/// **Test plan:** `test_version_below_activation` — `height < activation` ⇒ [`VERSION_V1`] (simulated fork height).
#[test]
fn test_version_below_activation() {
    const ACT: u64 = 1_000_000;
    assert_eq!(
        L2BlockHeader::protocol_version_for_height_with_activation(ACT - 1, ACT),
        VERSION_V1
    );
}

/// **Test plan:** `test_version_at_activation` — `height == activation` ⇒ [`VERSION_V2`] when activation is finite.
#[test]
fn test_version_at_activation() {
    const ACT: u64 = 1_000_000;
    assert_eq!(
        L2BlockHeader::protocol_version_for_height_with_activation(ACT, ACT),
        VERSION_V2
    );
}

/// **Test plan:** `test_version_above_activation` — `height > activation` ⇒ [`VERSION_V2`].
#[test]
fn test_version_above_activation() {
    const ACT: u64 = 1_000_000;
    assert_eq!(
        L2BlockHeader::protocol_version_for_height_with_activation(ACT + 1, ACT),
        VERSION_V2
    );
}

/// **Test plan:** `test_version_default_always_v1` — with `DFSP_ACTIVATION_HEIGHT == u64::MAX`, every practical height is V1.
///
/// Proves the BLK-007 note: `height >= u64::MAX` is only true at `height == u64::MAX`; the implementation
/// treats the disabled sentinel first, so **all** heights use V1 until governance sets a finite activation.
#[test]
fn test_version_default_always_v1() {
    assert_eq!(DFSP_ACTIVATION_HEIGHT, u64::MAX);
    for height in [0_u64, 1, 9_999_999, u64::MAX - 1] {
        assert_eq!(
            L2BlockHeader::protocol_version_for_height(height),
            VERSION_V1,
            "height {height}"
        );
    }
}

/// **Test plan:** `test_version_at_height_u64_max_minus_one` — explicit acceptance row (default config).
#[test]
fn test_version_at_height_u64_max_minus_one() {
    assert_eq!(
        L2BlockHeader::protocol_version_for_height(u64::MAX - 1),
        VERSION_V1
    );
}

/// **Test plan:** `test_all_constructors_same_logic` — `new` variants at the same height share `version`; genesis matches height 0.
#[test]
fn test_all_constructors_same_logic() {
    let height = 42_u64;
    let a = header_new_at(height);
    let b = L2BlockHeader::new_with_collateral(
        height,
        0,
        tag(0x01),
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        0,
        tag(0x02),
        0,
        0,
        0 as Cost,
        0,
        0,
        0,
        0,
        EMPTY_ROOT,
        tag(0xcc),
    );
    let c = L2BlockHeader::new_with_l1_proofs(
        height,
        0,
        tag(0x01),
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        0,
        tag(0x02),
        0,
        0,
        0 as Cost,
        0,
        0,
        0,
        0,
        EMPTY_ROOT,
        tag(0x10),
        tag(0x11),
        tag(0x12),
        tag(0x13),
        tag(0x14),
    );

    assert_eq!(a.version, b.version);
    assert_eq!(a.version, c.version);
    assert_eq!(
        a.version,
        L2BlockHeader::protocol_version_for_height(height),
        "constructor version must match protocol_version_for_height"
    );

    let g = L2BlockHeader::genesis(tag(0x99), 1, tag(0xaa));
    assert_eq!(g.height, 0);
    assert_eq!(g.version, L2BlockHeader::protocol_version_for_height(0));
}

/// **Test plan:** `test_version_not_in_api` — `version` is not a parameter on constructors (compile-time contract).
///
/// Rust integration tests cannot run negative `trybuild` cases here; the **source API** of
/// [`L2BlockHeader::new`], [`L2BlockHeader::new_with_collateral`], [`L2BlockHeader::new_with_l1_proofs`], and
/// [`L2BlockHeader::genesis`] omits `version`, and this test locks the runtime equality
/// `header.version == f(header.height)` for [`header_new_at`].
#[test]
fn test_version_not_in_api() {
    let h = header_new_at(17);
    assert_eq!(
        h.version,
        L2BlockHeader::protocol_version_for_height(h.height)
    );
}
