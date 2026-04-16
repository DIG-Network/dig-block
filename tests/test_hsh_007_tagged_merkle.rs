//! HSH-007: Tagged Merkle hashing — `0x01` leaf prefix and `0x02` internal-node prefix.
//!
//! **Normative:** `docs/requirements/domains/hashing/NORMATIVE.md` (HSH-007)  
//! **Spec + test plan:** `docs/requirements/domains/hashing/specs/HSH-007.md`  
//! **Implementation:** `src/hash.rs` (`dig_block::hash_leaf`, `dig_block::hash_node`)  
//! **Crate spec:** [SPEC §3.3](docs/resources/SPEC.md)
//!
//! **Layout note:** HSH-007 names `tests/hashing/test_tagged_merkle_hashing.rs`; this repository keeps a **flat** `tests/` tree
//! ([STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md) `integration_tests_directory_is_flat`), so this file lives at the root.
//!
//! ## How these tests prove HSH-007
//!
//! - **`test_leaf_hash_prefix` / `test_node_hash_prefix`:** Manual [`dig_block::hash_leaf`] / [`dig_block::hash_node`]
//!   match [`chia_sdk_types::MerkleTree`] roots on the same leaves — the SDK is the Chia reference implementation
//!   for this tagging ([`MerkleTree` source](https://github.com/Chia-Network/chia-sdk/blob/main/crates/chia-sdk-types/src/merkle_tree.rs)).
//! - **`test_leaf_node_domain_separation`:** Accepts the **second-preimage** goal: treating the same 32 bytes as a leaf
//!   preimage vs as a child digest in [`hash_node`] yields different digests, so roles are not interchangeable.
//! - **`test_chia_parity_*`:** Rust-level parity with Chia’s tagged tree (Python `merkle_utils.py` mirrors the same
//!   prefixes for puzzle-facing Merkle proofs; we anchor on `chia-sdk-types` in this repo).
//! - **`test_public_prefix_constants_match_blk005`:** [`dig_block::HASH_LEAF_PREFIX`] / [`dig_block::HASH_TREE_PREFIX`]
//!   stay aligned with BLK-005; Merkle **tree** code continues to delegate to Chia crates rather than reimplementing trees.
//!
//! **SocratiCode:** Not used in this environment (no MCP); search was done via repo + `chia-sdk-types` registry source.

use chia_sdk_types::MerkleTree;
use dig_block::{hash_leaf, hash_node, Bytes32, HASH_LEAF_PREFIX, HASH_TREE_PREFIX};

/// **Test plan:** `test_leaf_hash_prefix` — SHA-256(0x01 ‖ leaf_bytes) equals a one-leaf [`MerkleTree`] root.
#[test]
fn hsh007_leaf_hash_prefix_matches_merkle_tree_single_leaf() {
    let leaf = Bytes32::new([0x7a; 32]);
    let expected = MerkleTree::new(&[leaf]).root();
    assert_eq!(hash_leaf(leaf.as_ref()), expected);
}

/// **Test plan:** `test_node_hash_prefix` — parent step matches two-leaf [`MerkleTree`] (balanced split).
#[test]
fn hsh007_node_hash_prefix_matches_merkle_tree_two_leaves() {
    let a = Bytes32::new([0x01; 32]);
    let b = Bytes32::new([0x02; 32]);
    let tree_root = MerkleTree::new(&[a, b]).root();
    let left_d = hash_leaf(a.as_ref());
    let right_d = hash_leaf(b.as_ref());
    assert_eq!(hash_node(&left_d, &right_d), tree_root);
}

/// **Test plan:** `test_leaf_node_domain_separation` — same 32-byte string is not ambiguous between leaf and node roles.
#[test]
fn hsh007_leaf_and_node_hashes_differ_for_same_underlying_bytes() {
    let preimage = Bytes32::new([0x5e; 32]);
    let as_leaf = hash_leaf(preimage.as_ref());
    // Treat the same bytes as two child digests at an internal node (not as leaf preimages of a two-leaf tree).
    let as_node = hash_node(&preimage, &preimage);
    assert_ne!(
        as_leaf, as_node,
        "0x01 vs 0x02 domain separation must keep leaf and node mixes distinct"
    );
}

/// **Test plan:** `test_chia_parity_leaf` — [`MerkleTree`] implements the same tagged leaf step as Chia `merkle_utils.hash_leaf`.
#[test]
fn hsh007_chia_parity_leaf_via_sdk() {
    let v = Bytes32::new([0xcc; 32]);
    assert_eq!(hash_leaf(v.as_ref()), MerkleTree::new(&[v]).root());
}

/// **Test plan:** `test_chia_parity_node` — internal node mix matches [`MerkleTree`] over two leaves.
#[test]
fn hsh007_chia_parity_node_via_sdk() {
    let x = Bytes32::new([0x11; 32]);
    let y = Bytes32::new([0x22; 32]);
    let root = MerkleTree::new(&[x, y]).root();
    assert_eq!(
        root,
        hash_node(&hash_leaf(x.as_ref()), &hash_leaf(y.as_ref()))
    );
}

/// **Test plan:** `test_no_constant_redefinition` — BLK-005 names stay `0x01`/`0x02`; binary Merkle aggregation in this crate
/// goes through `chia-sdk-types` ([`MerkleTree`]) rather than a second copy of the tree logic with different tags.
#[test]
fn hsh007_public_prefix_constants_match_blk005() {
    assert_eq!(HASH_LEAF_PREFIX, 0x01);
    assert_eq!(HASH_TREE_PREFIX, 0x02);
}
