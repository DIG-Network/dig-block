//! Error enums for dig-block.
//!
//! - **Tier 1 [`BlockError`]** (structural): [ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md), [NORMATIVE § ERR-001](docs/requirements/domains/error_types/NORMATIVE.md).
//! - **Tier 2 /3 [`BlockError`]** (execution / state): deferred to [ERR-002](docs/requirements/domains/error_types/specs/ERR-002.md) — keep this file in sync when those variants land.
//! - **Crate spec:** [SPEC §4.1](docs/resources/SPEC.md) — error taxonomy and validator layering.

use crate::primitives::Bytes32;
use thiserror::Error;

/// Block validation failures: Tier 1 (structural) variants per [ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md).
///
/// **Design:** Structural checks run before CLVM or coin-state lookups ([structural_validation NORMATIVE](docs/requirements/domains/structural_validation/NORMATIVE.md)).
/// Each variant carries enough context for SVL-* validators to return actionable diagnostics without stringly-typed ad hoc errors.
///
/// **Derivation:** `Debug` + `Clone` allow logging, test fixtures, and cheap duplication; `thiserror::Error` supplies [`std::fmt::Display`] and [`std::error::Error`]
/// ([ERR-001 acceptance](docs/requirements/domains/error_types/specs/ERR-001.md#acceptance-criteria)).
///
/// **Semantic links:** Serialization maps decode failures to [`BlockError::InvalidData`] ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)).
#[derive(Debug, Clone, Error)]
pub enum BlockError {
    // --- Tier 1: Structural validation (ERR-001) ---
    /// Opaque structural failure (e.g. bincode parse, policy text); prefer specific variants when available.
    #[error("invalid data: {0}")]
    InvalidData(String),

    /// Header `version` does not match the epoch / activation rules.
    #[error("invalid version: {0}")]
    InvalidVersion(u16),

    /// Serialized block size exceeds [`crate::MAX_BLOCK_SIZE`](crate::constants) or configured limit.
    #[error("block too large: {size} bytes exceeds max {max}")]
    TooLarge { size: u32, max: u32 },

    /// Sum of spend costs exceeds [`crate::MAX_COST_PER_BLOCK`](crate::constants) or header-declared budget.
    #[error("cost exceeded: {cost} exceeds max {max}")]
    CostExceeded { cost: u64, max: u64 },

    /// `header.spend_bundle_count` does not match `spend_bundles.len()`.
    #[error("spend bundle count mismatch: header={header}, actual={actual}")]
    SpendBundleCountMismatch { header: u32, actual: u32 },

    /// Merkle root of spends does not match `header.spends_root`.
    #[error("invalid spends root: expected={expected}, computed={computed}")]
    InvalidSpendsRoot {
        expected: Bytes32,
        computed: Bytes32,
    },

    /// Merkle root of receipts does not match `header.receipts_root`.
    #[error("invalid receipts root: expected={expected}, computed={computed}")]
    InvalidReceiptsRoot {
        expected: Bytes32,
        computed: Bytes32,
    },

    /// `header.prev_hash` chain link failure (wrong parent).
    #[error("invalid parent: expected={expected}, got={got}")]
    InvalidParent { expected: Bytes32, got: Bytes32 },

    /// Slash proposals Merkle root does not match commitment in header.
    #[error("invalid slash proposals root")]
    InvalidSlashProposalsRoot,

    /// Single slash proposal payload exceeds max serialized size.
    #[error("slash proposal payload too large")]
    SlashProposalPayloadTooLarge,

    /// More slash proposals than protocol allows.
    #[error("too many slash proposals")]
    TooManySlashProposals,

    /// Additions set root mismatch (see SVL-006).
    #[error("invalid additions root")]
    InvalidAdditionsRoot,

    /// Removals set root mismatch.
    #[error("invalid removals root")]
    InvalidRemovalsRoot,

    /// Gossip / filter hash in header does not match computed filter.
    #[error("invalid filter hash")]
    InvalidFilterHash,

    /// Two outputs mint the same coin id within the block.
    #[error("duplicate output: coin_id={coin_id}")]
    DuplicateOutput { coin_id: Bytes32 },

    /// Same coin id spent more than once in removals.
    #[error("double spend in block: coin_id={coin_id}")]
    DoubleSpendInBlock { coin_id: Bytes32 },

    /// Header `additions_count` vs computed additions length.
    #[error("additions count mismatch")]
    AdditionsCountMismatch,

    /// Header `removals_count` vs computed removals length.
    #[error("removals count mismatch")]
    RemovalsCountMismatch,

    /// Header slash proposal count vs payloads length.
    #[error("slash proposal count mismatch")]
    SlashProposalCountMismatch,

    /// Block timestamp too far ahead of local wall clock ([SVL-004](docs/requirements/domains/structural_validation/specs/SVL-004.md)).
    #[error("timestamp too far in future: {timestamp} exceeds max_allowed {max_allowed}")]
    TimestampTooFarInFuture { timestamp: u64, max_allowed: u64 },
}

/// Errors from checkpoint operations.
#[derive(Debug, Error)]
pub enum CheckpointError {
    /// Placeholder — variants will be added in ERR-003.
    #[error("checkpoint error: {0}")]
    Other(String),
}

/// Errors from block/checkpoint builder operations.
#[derive(Debug, Error)]
pub enum BuilderError {
    /// Placeholder — variants will be added in ERR-004.
    #[error("builder error: {0}")]
    Other(String),
}

/// Errors from SignerBitmap operations.
///
/// - **ATT-004:** [`SignerBitmapError::IndexOutOfBounds`] — [`crate::SignerBitmap::set_signed`] when `index >= validator_count`.
/// - **ATT-005:** [`SignerBitmapError::ValidatorCountMismatch`] — [`crate::SignerBitmap::merge`] when counts differ.
///
/// Further variants: [ERR-005](docs/requirements/domains/error_types/specs/ERR-005.md).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SignerBitmapError {
    /// Validator index is not in `[0, validator_count)`.
    #[error("validator index out of bounds for this bitmap")]
    IndexOutOfBounds,
    /// [`crate::SignerBitmap::merge`] requires both operands to use the same `validator_count`.
    #[error("signer bitmap validator_count mismatch")]
    ValidatorCountMismatch,
    /// Placeholder — variants will be expanded in ERR-005.
    #[error("signer bitmap error: {0}")]
    Other(String),
}

/// Errors from Receipt operations.
#[derive(Debug, Error)]
pub enum ReceiptError {
    /// Placeholder — variants will be added in ERR-005.
    #[error("receipt error: {0}")]
    Other(String),
}
