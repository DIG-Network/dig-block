//! # `dig-block` — DIG L2 block format, production, and validation
//!
//! `dig-block` is a self-contained Rust library for the DIG Network L2 blockchain. It owns three
//! concerns in one crate:
//!
//! 1. **Block format** — type definitions for [`L2BlockHeader`], [`L2Block`], [`AttestedBlock`],
//!    [`Checkpoint`], [`CheckpointSubmission`], and supporting types ([SPEC §2](docs/resources/SPEC.md)).
//! 2. **Block production** — [`BlockBuilder`] and [`CheckpointBuilder`] construct structurally
//!    valid blocks and epoch summaries from their inputs ([SPEC §6](docs/resources/SPEC.md)).
//! 3. **Block validation** — a three-tier pipeline (structural / execution / state) rejects any
//!    block that does not match consensus rules ([SPEC §5, §7](docs/resources/SPEC.md)).
//!
//! ## Scope
//!
//! The crate operates on **single, isolated blocks**. External state (coin existence, chain tip,
//! wall-clock time, validator set) is injected through two traits:
//!
//! - [`CoinLookup`] — coin-set queries for Tier-3 state validation.
//! - [`BlockSigner`] — proposer signing hook for block production.
//!
//! `dig-block` never reads from a database, never makes network calls, and never maintains state
//! across blocks ([SPEC §11](docs/resources/SPEC.md)). Downstream crates (`dig-coinstore`,
//! `dig-epoch`, `dig-gossip`) provide the trait implementations, storage, chain management, and
//! networking.
//!
//! ## Quickstart — build a block
//!
//! ```no_run
//! use dig_block::{BlockBuilder, BlockSigner, Bytes32, Signature};
//! use dig_block::traits::SignerError;
//!
//! // 1. Implement BlockSigner for your key material (or use a test mock).
//! struct MySigner;
//! impl BlockSigner for MySigner {
//!     fn sign_block(&self, _header_hash: &Bytes32) -> Result<Signature, SignerError> {
//!         Ok(Signature::default())
//!     }
//! }
//!
//! // 2. Construct a builder anchored to the chain / L1 context.
//! let parent_hash = Bytes32::default();
//! let l1_hash = Bytes32::default();
//! let mut builder = BlockBuilder::new(/*height=*/ 1, /*epoch=*/ 0, parent_hash, 100, l1_hash, 0);
//!
//! // 3. (Optional) add spend bundles via builder.add_spend_bundle(...).
//! // 4. Build a signed L2Block.
//! let state_root = Bytes32::default();
//! let receipts_root = Bytes32::default();
//! let block = builder.build(state_root, receipts_root, &MySigner).expect("build");
//! ```
//!
//! ## Quickstart — validate a block
//!
//! ```no_run
//! use dig_block::{CoinLookup, L2Block, PublicKey};
//! use chia_protocol::{Bytes32, CoinState};
//! use dig_clvm::ValidationConfig;
//!
//! struct MyCoinLookup;
//! impl CoinLookup for MyCoinLookup {
//!     fn get_coin_state(&self, _id: &Bytes32) -> Option<CoinState> { None }
//!     fn get_chain_height(&self) -> u64 { 0 }
//!     fn get_chain_timestamp(&self) -> u64 { 0 }
//! }
//!
//! fn validate(block: &L2Block, pk: &PublicKey) -> Result<Bytes32, dig_block::BlockError> {
//!     let config = ValidationConfig::default();
//!     let genesis = Bytes32::default();
//!     // Runs Tier 1 (structural) → Tier 2 (execution) → Tier 3 (state) and returns the
//!     // computed state root on success.
//!     block.validate_full(&config, &genesis, &MyCoinLookup, pk)
//! }
//! ```
//!
//! ## Module map
//!
//! | Module | SPEC section | Purpose |
//! |--------|-------------|---------|
//! | [`primitives`] | §2.1 | [`Bytes32`], [`Cost`], [`Signature`], [`PublicKey`], version tags |
//! | [`constants`] | §2.11 | [`EMPTY_ROOT`], [`MAX_BLOCK_SIZE`], [`MAX_COST_PER_BLOCK`], etc. |
//! | [`types`] | §2.2–§2.10 | [`L2BlockHeader`], [`L2Block`], [`AttestedBlock`], checkpoint types, [`Receipt`], [`SignerBitmap`], status enums |
//! | [`error`] | §4 | [`BlockError`], [`CheckpointError`], [`BuilderError`], [`SignerBitmapError`], [`ReceiptError`] |
//! | [`hash`] | §3.3 | [`hash_leaf`], [`hash_node`] (0x01/0x02 domain separation) |
//! | [`traits`] | §7.2 | [`CoinLookup`], [`BlockSigner`] |
//! | [`builder`] | §6 | [`BlockBuilder`], [`CheckpointBuilder`] |
//! | [`validation`] | §5, §7 | Structural (Tier 1), Execution (Tier 2), State (Tier 3) |
//!
//! ## Public API
//!
//! All protocol types and helpers are re-exported at the crate root ([SPEC §10](docs/resources/SPEC.md)).
//! For convenience, [`prelude`] re-exports the most common items as a glob.
//!
//! ```
//! use dig_block::prelude::*;
//! ```
//!
//! ## Dependencies
//!
//! `dig-block` reuses the Chia Rust ecosystem and does not reimplement CLVM, BLS, or Merkle
//! primitives ([SPEC §1.2](docs/resources/SPEC.md)):
//!
//! | Concern | Crate |
//! |---------|-------|
//! | Core protocol types ([`Bytes32`], `Coin`, `SpendBundle`, `CoinSpend`, `CoinState`) | `chia-protocol` |
//! | BLS12-381 signatures ([`Signature`], [`PublicKey`], `verify`) | `chia-bls` |
//! | CLVM execution + condition parsing | `dig-clvm` (wraps `chia-consensus`) |
//! | Merkle set roots (additions, removals) | `chia-consensus::compute_merkle_set_root` |
//! | Binary Merkle trees (spends, receipts, slash proposals) | `chia-sdk-types::MerkleTree` |
//! | SHA-256 | `chia-sha2` |
//! | CLVM tree hashing | `clvm-utils::tree_hash` |
//! | Bincode serialization | `bincode` + `serde` |
//! | BIP-158 compact block filter | `bitcoin::bip158` |
//!
//! CLVM execution is **always** routed through `dig_clvm::validate_spend_bundle`; dig-block never
//! calls `chia-consensus::run_spendbundle` directly
//! ([EXE-003](docs/requirements/domains/execution_validation/specs/EXE-003.md) enforced by a
//! grep-based architectural lint in `tests/test_exe_003_clvm_delegation.rs`).

pub mod builder;
pub mod constants;
pub mod error;
pub mod hash;
mod merkle_util;
pub mod primitives;
pub mod traits;
pub mod types;
pub mod validation;

// -- Public re-exports (STR-003 / [SPEC §10](docs/resources/SPEC.md)) --

// Block types (SPEC §2.2–§2.4, §2.6–§2.7)
pub use types::attested::AttestedBlock;
pub use types::block::L2Block;
pub use types::checkpoint::{Checkpoint, CheckpointSubmission};
pub use types::header::L2BlockHeader;

// Test-only helper re-export (BLK-004). Kept in its own re-export block so rustfmt cannot
// mingle the `#[doc(hidden)]` attribute with the public types above — different rustfmt
// versions disagree on whether `__blk004…` sorts before or after `L2Block`, and splitting
// the groups keeps the file stable across toolchains.
#[doc(hidden)]
pub use types::block::__blk004_first_duplicate_addition_coin_id;

// Status and supporting types (SPEC §2.5, §2.8–§2.10)
pub use types::receipt::{Receipt, ReceiptList, ReceiptStatus};
pub use types::signer_bitmap::{SignerBitmap, MAX_VALIDATORS};
pub use types::status::{BlockStatus, CheckpointStatus};

// Error types (SPEC §4)
pub use error::{BlockError, BuilderError, CheckpointError, ReceiptError, SignerBitmapError};

// Primitive types & Chia re-exports (BLK-006 / SPEC §2.1)
pub use primitives::{Bytes32, Cost, PublicKey, Signature, VERSION_V1, VERSION_V2};

// Constants (BLK-005 / SPEC §2.11); uses [`Cost`] / [`Bytes32`] from [`primitives`]
pub use constants::*;

// Tagged Merkle hashing (SPEC §3.3 — 0x01 leaf / 0x02 node domain separation)
pub use hash::{hash_leaf, hash_node};

// Spends root (SPEC §3.3 — MerkleTree over sha256(SpendBundle))
pub use merkle_util::compute_spends_root;

// Additions Merkle set root (SPEC §3.4 — compute_merkle_set_root grouped by puzzle_hash)
pub use merkle_util::compute_additions_root;

// Removals Merkle set root (SPEC §3.5 — compute_merkle_set_root of coin IDs)
pub use merkle_util::compute_removals_root;

// BIP-158 compact block filter + filter_hash (SPEC §3.6)
pub use merkle_util::{compact_block_filter_encoded, compute_filter_hash};

// Receipts Merkle root (SPEC §3.3 — MerkleTree over sha256(bincode(Receipt)))
pub use types::receipt::compute_receipts_root;

// Traits (SPEC §7.2 — CoinLookup for state validation, BlockSigner for block production)
pub use traits::{BlockSigner, CoinLookup};

// Builder types (SPEC §6 — block and checkpoint construction)
pub use builder::block_builder::BlockBuilder;
pub use builder::checkpoint_builder::CheckpointBuilder;

// Validation surface (SPEC §7.4–§7.5 — ExecutionResult + assertion types + Tier-2/3 helpers)
pub use validation::execution::{
    collect_pending_assertions_from_conditions, compute_state_root_from_delta,
    map_clvm_validation_error, verify_coin_spend_puzzle_hash, AssertionKind, ExecutionResult,
    PendingAssertion,
};

// ---------------------------------------------------------------------------
// Prelude — convenience glob for consumers.
// ---------------------------------------------------------------------------

/// Common imports for `dig-block` consumers.
///
/// ```
/// use dig_block::prelude::*;
/// ```
///
/// Re-exports the most frequently used types so downstream code does not have to enumerate
/// individual items. Internal helpers and architectural utilities are deliberately excluded;
/// reach for them through the crate root (e.g. [`crate::compute_state_root_from_delta`]).
pub mod prelude {
    pub use crate::{
        // Block types
        AttestedBlock,
        BlockBuilder,
        BlockError,
        // Traits
        BlockSigner,
        // Status
        BlockStatus,
        BuilderError,
        // Core Chia primitives
        Bytes32,
        // Checkpoint types
        Checkpoint,
        CheckpointBuilder,
        CheckpointError,
        CheckpointStatus,
        CheckpointSubmission,
        CoinLookup,
        Cost,
        // Tier-2 output
        ExecutionResult,
        L2Block,
        L2BlockHeader,
        PublicKey,
        // Receipt types
        Receipt,
        ReceiptList,
        ReceiptStatus,
        Signature,
        // Attestation
        SignerBitmap,
    };
}
