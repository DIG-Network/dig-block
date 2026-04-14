//! BLK-006: Primitive types and crate-root type surface.
//!
//! **Authoritative spec:** `docs/requirements/domains/block_types/specs/BLK-006.md`
//! **Normative:** `docs/requirements/domains/block_types/NORMATIVE.md` (BLK-006)
//!
//! These tests implement the BLK-006 **Test Plan**: they show that [`dig_block`] exposes the correct
//! `Cost` alias, protocol version constants, and Chia ecosystem types at the crate root so downstream
//! crates can depend on **`dig-block` alone** for the shared DIG L2 vocabulary (see implementation notes in
//! the spec — avoids forcing every consumer to pin `chia-protocol` / `chia-bls` for `Bytes32` / BLS types).

use chia_bls::{PublicKey as ChiaPublicKey, Signature as ChiaSignature};
use chia_protocol::Bytes32 as ChiaBytes32;
use dig_block::{Bytes32, Cost, PublicKey, Signature, VERSION_V1, VERSION_V2};

/// **Test plan:** `test_cost_is_u64` — `Cost` accepts `u64` values (type alias to `u64`).
///
/// Proves the acceptance criterion “`Cost` is a type alias for u64”: if this assignment compiled in CI,
/// Rust has unified `Cost` with `u64` for this crate’s public API.
#[test]
fn test_cost_is_u64() {
    let bundle_reported: Cost = 1_234_567_u64;
    let as_plain_u64: u64 = bundle_reported;
    assert_eq!(as_plain_u64, 1_234_567);
}

/// **Test plan:** `test_version_v1_value` — pre-DFSP header version tag.
///
/// Satisfies BLK-006 and feeds BLK-007: below [`dig_block::DFSP_ACTIVATION_HEIGHT`], headers use V1.
#[test]
fn test_version_v1_value() {
    assert_eq!(VERSION_V1, 1);
}

/// **Test plan:** `test_version_v2_value` — DFSP-era header version tag.
///
/// Used once height is at or past activation (see BLK-007); value must remain stable for fork safety.
#[test]
fn test_version_v2_value() {
    assert_eq!(VERSION_V2, 2);
}

/// **Test plan:** `test_bytes32_reexport` — `crate::Bytes32` is `chia_protocol::Bytes32`.
///
/// We pass a value constructed via the **`dig_block`** path into a function typed with
/// **`chia_protocol::Bytes32`**. If the types differed, this would not compile — so the test is both
/// documentation and a compile-time contract check.
#[test]
fn test_bytes32_reexport() {
    fn accepts_chia_protocol_hash(_: ChiaBytes32) {}

    let from_dig_block = Bytes32::new([7u8; 32]);
    accepts_chia_protocol_hash(from_dig_block);
}

/// **Test plan:** `test_signature_reexport` — `crate::Signature` is `chia_bls::Signature`.
#[test]
fn test_signature_reexport() {
    fn accepts_chia_signature(_: ChiaSignature) {}

    let from_dig_block = Signature::default();
    accepts_chia_signature(from_dig_block);
}

/// **Test plan:** `test_public_key_reexport` — `crate::PublicKey` is `chia_bls::PublicKey`.
#[test]
fn test_public_key_reexport() {
    fn accepts_chia_pk(_: ChiaPublicKey) {}

    let from_dig_block = PublicKey::default();
    accepts_chia_pk(from_dig_block);
}

/// **Acceptance:** primitives remain glob-importable from the crate root alongside constants.
///
/// Guards the “publicly accessible from the crate root” criterion without duplicating STR-003’s full
/// import matrix.
#[test]
fn test_primitives_visible_with_glob() {
    use dig_block::*;

    let _: Cost = 0;
    let _ = (VERSION_V1, VERSION_V2);
    let _ = Bytes32::default();
    let _ = Signature::default();
    let _ = PublicKey::default();
}
