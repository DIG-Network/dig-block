//! Tagged Merkle hashing for **binary** Merkle trees (HSH-007).
//!
//! **Normative:** [HSH-007](docs/requirements/domains/hashing/specs/HSH-007.md) / [hashing NORMATIVE](docs/requirements/domains/hashing/NORMATIVE.md)  
//! **Crate spec:** [SPEC §3.3](docs/resources/SPEC.md)  
//! **Prefixes:** [`crate::HASH_LEAF_PREFIX`] and [`crate::HASH_TREE_PREFIX`] ([BLK-005](docs/requirements/domains/block_types/specs/BLK-005.md)).
//!
//! ## What this module is for
//!
//! - [`hash_leaf`] and [`hash_node`] are the explicit SHA-256 formulas Chia uses for **leaf** and **internal**
//!   nodes in [`chia_sdk_types::MerkleTree`] (see `merkle_tree.rs` in that crate: `HASH_LEAF_PREFIX = &[1]`,
//!   `HASH_TREE_PREFIX = &[2]` — [upstream source](https://github.com/Chia-Network/chia-sdk/blob/main/crates/chia-sdk-types/src/merkle_tree.rs)).
//! - DIG block code builds those trees via [`crate::merkle_util::merkle_tree_root`] and direct [`MerkleTree::new`]
//!   calls; **do not** hand-roll a parallel Merkle implementation with different tags.
//!
//! ## What this is *not*
//!
//! - [`chia_consensus::merkle_set::compute_merkle_set_root`] (additions/removals roots) uses Chia’s **radix / sorted-set**
//!   tree and a **different** internal `hash(ltype, rtype, left, right)` — not `0x01||data` / `0x02||a||b`.
//!   That path is correct for HSH-004/HSH-005 but is **out of scope** for the formulas in this file ([`merkle_util`](crate::merkle_util)).
//!
//! ## Rationale
//!
//! Exposing [`hash_leaf`] / [`hash_node`] makes the domain separation **testable** and gives downstream crates a
//! single place to name the tagging scheme, while the actual tree shape remains delegated to `chia-sdk-types`.

use chia_protocol::Bytes32;
use chia_sha2::Sha256;

use crate::constants::{HASH_LEAF_PREFIX, HASH_TREE_PREFIX};

/// Leaf digest: **SHA-256( [`HASH_LEAF_PREFIX`](crate::HASH_LEAF_PREFIX) ‖ `data` )**.
///
/// For a [`Bytes32`] leaf value `v` in [`chia_sdk_types::MerkleTree`], this is exactly the digest mixed at the leaf.
#[must_use]
pub fn hash_leaf(data: &[u8]) -> Bytes32 {
    let mut hasher = Sha256::new();
    hasher.update([HASH_LEAF_PREFIX]);
    hasher.update(data);
    Bytes32::new(hasher.finalize())
}

/// Internal node digest: **SHA-256( [`HASH_TREE_PREFIX`](crate::HASH_TREE_PREFIX) ‖ `left` ‖ `right` )**.
///
/// This combines two **already-hashed** child digests (typically outputs of [`hash_leaf`] or nested [`hash_node`] results).
#[must_use]
pub fn hash_node(left: &Bytes32, right: &Bytes32) -> Bytes32 {
    let mut hasher = Sha256::new();
    hasher.update([HASH_TREE_PREFIX]);
    hasher.update(left.as_ref());
    hasher.update(right.as_ref());
    Bytes32::new(hasher.finalize())
}
