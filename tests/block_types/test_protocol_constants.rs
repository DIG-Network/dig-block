//! BLK-005: Protocol constants — acceptance tests.
//!
//! **Authoritative spec:** `docs/requirements/domains/block_types/specs/BLK-005.md`
//! **Normative summary:** `docs/requirements/domains/block_types/NORMATIVE.md` (BLK-005)
//!
//! Each test maps to a row in the BLK-005 **Test Plan** table. Together they demonstrate that the crate
//! exposes the exact sentinel values, limits, and Merkle prefixes the DIG L2 protocol requires, so
//! downstream validation and builders can rely on these invariants.

use chia_sha2::Sha256;
use dig_block::{
    DFSP_ACTIVATION_HEIGHT, EMPTY_ROOT, HASH_LEAF_PREFIX, HASH_TREE_PREFIX, MAX_BLOCK_SIZE,
    MAX_COST_PER_BLOCK, MAX_FUTURE_TIMESTAMP_SECONDS, MAX_SLASH_PROPOSALS_PER_BLOCK,
    MAX_SLASH_PROPOSAL_PAYLOAD_BYTES, ZERO_HASH,
};

/// **Test plan:** `test_empty_root_value` — proves `EMPTY_ROOT` is SHA-256 of the empty string.
///
/// Satisfies BLK-005 acceptance: “EMPTY_ROOT equals SHA256 of the empty string”. We hash `&[]` with the
/// same [`chia_sha2::Sha256`] primitive used across the Chia/DIG stack (see project `start.md` hard
/// requirement: no ad-hoc SHA-256).
#[test]
fn test_empty_root_value() {
    let expected = Sha256::new().finalize();
    assert_eq!(EMPTY_ROOT.as_ref(), expected.as_slice());
}

/// **Test plan:** `test_zero_hash_value` — `ZERO_HASH` is 32 zero bytes.
///
/// Satisfies BLK-005: distinct from [`EMPTY_ROOT`] so optional header fields and “no value” sentinels do
/// not collide with the canonical empty Merkle root.
#[test]
fn test_zero_hash_value() {
    assert_eq!(ZERO_HASH.as_ref(), &[0u8; 32]);
}

/// **Test plan:** `test_max_block_size` — serialized block size cap is 10_000_000 bytes.
#[test]
fn test_max_block_size() {
    assert_eq!(MAX_BLOCK_SIZE, 10_000_000);
}

/// **Test plan:** `test_max_cost_per_block` — per-block CLVM cost ceiling.
///
/// Typed as [`dig_block::Cost`] (u64) per BLK-005 / BLK-006 so execution validation shares units with
/// bundle-reported cost.
#[test]
fn test_max_cost_per_block() {
    assert_eq!(MAX_COST_PER_BLOCK, 550_000_000_000_u64);
}

/// **Test plan:** `test_max_slash_proposals` — slash proposal count bound.
#[test]
fn test_max_slash_proposals() {
    assert_eq!(MAX_SLASH_PROPOSALS_PER_BLOCK, 64);
}

/// **Test plan:** `test_max_slash_payload` — per-payload byte limit for slash proposals.
#[test]
fn test_max_slash_payload() {
    assert_eq!(MAX_SLASH_PROPOSAL_PAYLOAD_BYTES, 65_536);
}

/// **Test plan:** `test_dfsp_activation_height` — DFSP disabled by default (`u64::MAX`).
///
/// Proves the “effectively off until configured” semantics from BLK-005 implementation notes; BLK-007
/// relies on this for default VERSION_V1-only behavior.
#[test]
fn test_dfsp_activation_height() {
    assert_eq!(DFSP_ACTIVATION_HEIGHT, u64::MAX);
}

/// **Test plan:** `test_max_future_timestamp` — clock skew allowance (300 seconds).
#[test]
fn test_max_future_timestamp() {
    assert_eq!(MAX_FUTURE_TIMESTAMP_SECONDS, 300);
}

/// **Test plan:** `test_hash_leaf_prefix` — Merkle leaf domain tag.
#[test]
fn test_hash_leaf_prefix() {
    assert_eq!(HASH_LEAF_PREFIX, 0x01);
}

/// **Test plan:** `test_hash_tree_prefix` — Merkle internal-node domain tag.
#[test]
fn test_hash_tree_prefix() {
    assert_eq!(HASH_TREE_PREFIX, 0x02);
}

/// **Acceptance:** all BLK-005 constants remain reachable from the crate root (`pub use` in `lib.rs`).
///
/// This guards STR-003’s public API surface while encoding BLK-005’s “publicly accessible” criterion.
#[test]
fn test_constants_public_from_crate_root() {
    use dig_block::*;

    let _ = (
        EMPTY_ROOT,
        ZERO_HASH,
        MAX_BLOCK_SIZE,
        MAX_COST_PER_BLOCK,
        MAX_SLASH_PROPOSALS_PER_BLOCK,
        MAX_SLASH_PROPOSAL_PAYLOAD_BYTES,
        DFSP_ACTIVATION_HEIGHT,
        MAX_FUTURE_TIMESTAMP_SECONDS,
        HASH_LEAF_PREFIX,
        HASH_TREE_PREFIX,
    );
}
