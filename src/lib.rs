pub mod builder;
pub mod constants;
pub mod error;
pub mod hash;
mod merkle_util;
pub mod primitives;
pub mod traits;
pub mod types;
pub mod validation;

// -- Public re-exports (STR-003) --

// Block types
pub use types::attested::AttestedBlock;
pub use types::block::L2Block;
#[doc(hidden)]
pub use types::block::__blk004_first_duplicate_addition_coin_id;
pub use types::checkpoint::{Checkpoint, CheckpointSubmission};
pub use types::header::L2BlockHeader;

// Status and supporting types
pub use types::receipt::{Receipt, ReceiptList, ReceiptStatus};
pub use types::signer_bitmap::{SignerBitmap, MAX_VALIDATORS};
pub use types::status::{BlockStatus, CheckpointStatus};

// Error types
pub use error::{BlockError, BuilderError, CheckpointError, ReceiptError, SignerBitmapError};

// Primitive types & Chia re-exports (BLK-006)
pub use primitives::{Bytes32, Cost, PublicKey, Signature, VERSION_V1, VERSION_V2};

// Constants (BLK-005); uses [`Cost`] / [`Bytes32`] from [`primitives`]
pub use constants::*;

// Tagged Merkle (HSH-007); [`hash`] module also documents scope vs `merkle_set`.
pub use hash::{hash_leaf, hash_node};

// Spends root over spend bundles (HSH-003); implementation in [`crate::merkle_util`].
pub use merkle_util::compute_spends_root;

// Additions Merkle set root over `Coin` additions (HSH-004); implementation in [`crate::merkle_util`].
pub use merkle_util::compute_additions_root;

// Removals Merkle set root over coin IDs (HSH-005); implementation in [`crate::merkle_util`].
pub use merkle_util::compute_removals_root;

// BIP-158 compact block filter + `filter_hash` (HSH-006); implementation in [`crate::merkle_util`].
pub use merkle_util::{compact_block_filter_encoded, compute_filter_hash};

// Receipts Merkle root (HSH-008); implementation in [`crate::types::receipt`] (see module docs on import layering).
pub use types::receipt::compute_receipts_root;

// Traits
pub use traits::{BlockSigner, CoinLookup};

// Builder types
pub use builder::block_builder::BlockBuilder;
pub use builder::checkpoint_builder::CheckpointBuilder;

// Validation result (Tier 2 — [`ExecutionResult`] placeholder + EXE-009 assertion types; SER-001 serde surface)
pub use validation::execution::{
    collect_pending_assertions_from_conditions, map_clvm_validation_error,
    verify_coin_spend_puzzle_hash, AssertionKind, ExecutionResult, PendingAssertion,
};
