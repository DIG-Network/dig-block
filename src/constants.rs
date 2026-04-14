//! Protocol-wide limits and sentinel values for the DIG L2 block format.
//!
//! **Requirement:** [BLK-005](docs/requirements/domains/block_types/specs/BLK-005.md) /
//! [NORMATIVE § BLK-005](docs/requirements/domains/block_types/NORMATIVE.md#blk-005-protocol-constants) /
//! [SPEC §2.11](docs/resources/SPEC.md) (protocol constants).
//!
//! **Rationale:** Centralizing these values avoids magic numbers in validation ([`crate::validation`]),
//! builders ([`crate::builder`]), and hashing ([`crate::hash`]). Limits are chosen to bound worst-case
//! block work (cost, size) and slash-proposal abuse while staying aligned with mainnet-style CLVM budgets.
//!
//! **Types:** Numeric limits use the widths required by BLK-005 (`u32` for byte/count caps, `u64` for
//! height/timestamp/cost). [`Cost`] is a `u64` alias (full primitive surface is completed in BLK-006).

use chia_protocol::Bytes32;

/// Digest of the empty byte string, i.e. SHA-256 of `""`.
///
/// Used wherever an “empty” Merkle accumulator or trie root must be represented canonically (for example
/// pre-DFSP roots and empty subtrees). The value is fixed by the protocol; clients must not substitute
/// [`ZERO_HASH`] for this purpose.
///
/// **Proof obligation:** [`tests/block_types/test_protocol_constants.rs`](../../tests/block_types/test_protocol_constants.rs)
/// compares this constant to [`chia_sha2::Sha256`] of the empty input so we satisfy BLK-005 without
/// recomputing at runtime in production code.
pub const EMPTY_ROOT: Bytes32 = Bytes32::new([
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
]);

/// 32 zero bytes — a distinct sentinel from [`EMPTY_ROOT`].
///
/// **Rationale:** Header hashing and optional fields use an all-zero hash to mean “absent” or “zeroed
/// placeholder” per SPEC; Merkle empty roots must still use [`EMPTY_ROOT`]. Keeping both constants avoids
/// ambiguous “empty” semantics that would break second-layer verifiers.
pub const ZERO_HASH: Bytes32 = Bytes32::new([0u8; 32]);

/// Maximum serialized block size in bytes (10 MiB).
///
/// Structural validation and [`crate::builder::BlockBuilder`] must reject blocks that would exceed this
/// size ([`crate::validation::structural`], BLD-002 / SVL-003 in requirements).
pub const MAX_BLOCK_SIZE: u32 = 10_000_000;

/// CLVM execution budget allowed in a single block.
///
/// **Decision:** Typed as [`Cost`] (alias of `u64`) so execution checks share the same unit as bundle
/// cost fields throughout the stack; see BLK-006 for the full primitive-type surface.
pub const MAX_COST_PER_BLOCK: Cost = 550_000_000_000;

/// Upper bound on slash-proposal payloads included in one block.
///
/// Paired with [`MAX_SLASH_PROPOSAL_PAYLOAD_BYTES`] to cap slash metadata volume (structural validation
/// and builder paths; see SVL-006 / BLD-003).
pub const MAX_SLASH_PROPOSALS_PER_BLOCK: u32 = 64;

/// Maximum size of a single slash-proposal payload in bytes (64 KiB).
pub const MAX_SLASH_PROPOSAL_PAYLOAD_BYTES: u32 = 65_536;

/// Block height at which DFSP (decentralized fraud/slashing protocol) features activate.
///
/// **Decision:** Defaults to `u64::MAX` so DFSP is effectively off until governance updates this constant;
/// [`crate::types::header::L2BlockHeader`] auto-versioning (BLK-007) treats this as “always pre-DFSP” in
/// the default configuration.
pub const DFSP_ACTIVATION_HEIGHT: u64 = u64::MAX;

/// Maximum allowed block timestamp skew into the future (seconds).
///
/// Used by header structural checks (SVL-004) to bound clock abuse while tolerating reasonable skew.
pub const MAX_FUTURE_TIMESTAMP_SECONDS: u64 = 300;

/// Domain-separation prefix for Merkle leaf nodes (0x01).
///
/// **Rationale:** Prefixing leaf and internal node hashes prevents second-preimage ambiguity between
/// leaf-level data and hashed pairs; see HSH-007 and BLK-005 implementation notes.
pub const HASH_LEAF_PREFIX: u8 = 0x01;

/// Domain-separation prefix for Merkle internal nodes (0x02).
pub const HASH_TREE_PREFIX: u8 = 0x02;

/// CLVM / block cost unit (matches BLK-006).
pub type Cost = u64;
