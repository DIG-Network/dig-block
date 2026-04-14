//! Error enums for dig-block.
//!
//! - **Tier 1 [`BlockError`]** (structural): [ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md), [NORMATIVE § ERR-001](docs/requirements/domains/error_types/NORMATIVE.md).
//! - **Tier 2 /3 [`BlockError`]** (execution / state): [ERR-002](docs/requirements/domains/error_types/specs/ERR-002.md), [NORMATIVE § ERR-002](docs/requirements/domains/error_types/NORMATIVE.md).
//! - **[`CheckpointError`]** (checkpoints): [ERR-003](docs/requirements/domains/error_types/specs/ERR-003.md), [SPEC §4.2](docs/resources/SPEC.md).
//! - **Crate spec:** [SPEC §4.1–4.2](docs/resources/SPEC.md) — error taxonomy and validator layering.

use crate::primitives::Bytes32;
use thiserror::Error;

/// Block validation failures across three tiers on one enum ([ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md), [ERR-002](docs/requirements/domains/error_types/specs/ERR-002.md)).
///
/// **Tier 1 — structural:** cheapest checks first on [`crate::L2BlockHeader`] / [`crate::L2Block`] (SVL-*); no CLVM, no [`CoinLookup`].
///
/// **Tier 2 — execution:** CLVM / puzzle / signature / fee invariants for `validate_execution` (EXE-*;
/// [execution_validation NORMATIVE](docs/requirements/domains/execution_validation/NORMATIVE.md)). Prefer mapping `dig-clvm` errors into these variants rather than [`BlockError::InvalidData`].
///
/// **Tier 3 — state:** coin set and proposer checks in `validate_state` (STV-*;
/// [state_validation NORMATIVE](docs/requirements/domains/state_validation/NORMATIVE.md)).
///
/// **Derivation:** `Debug` + `Clone` + `thiserror::Error` — same rationale as ERR-001 ([acceptance criteria](docs/requirements/domains/error_types/specs/ERR-002.md#acceptance-criteria)).
///
/// **Semantic links:** Serialization still uses [`BlockError::InvalidData`] for decode failures ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)).
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

    // --- Tier 2: Execution validation (ERR-002) ---
    /// On-chain puzzle hash does not match hash of serialized puzzle revealed in the spend ([EXE NORMATIVE](docs/requirements/domains/execution_validation/NORMATIVE.md)).
    #[error("puzzle hash mismatch for coin {coin_id}: expected={expected}, computed={computed}")]
    PuzzleHashMismatch {
        coin_id: Bytes32,
        expected: Bytes32,
        computed: Bytes32,
    },

    /// `dig-clvm` / CLVM runtime rejected the spend; `reason` preserves the engine diagnostic ([start.md](docs/prompt/start.md) — use dig-clvm, not raw `chia-consensus` entrypoints).
    #[error("CLVM execution failed for coin {coin_id}: {reason}")]
    ClvmExecutionFailed { coin_id: Bytes32, reason: String },

    /// Single spend exceeded the remaining per-block CLVM budget after prior spends.
    #[error("CLVM cost exceeded for coin {coin_id}: cost={cost}, remaining={remaining}")]
    ClvmCostExceeded {
        coin_id: Bytes32,
        cost: u64,
        remaining: u64,
    },

    /// ASSERT_* or concurrent-spend assertion failed; `condition` names the opcode class, `reason` is validator-local detail ([ERR-002 notes](docs/requirements/domains/error_types/specs/ERR-002.md#implementation-notes)).
    #[error("assertion failed: condition={condition}, reason={reason}")]
    AssertionFailed { condition: String, reason: String },

    /// Spend expected an announcement that was not present in the ephemeral announcement set.
    #[error("announcement not found: {announcement_hash}")]
    AnnouncementNotFound { announcement_hash: Bytes32 },

    /// Aggregate or AGG_SIG verification failed for the spend bundle at `bundle_index`.
    #[error("signature verification failed for bundle index {bundle_index}")]
    SignatureFailed { bundle_index: u32 },

    /// Value conservation failure: outputs (`added`) exceed destroyed value (`removed`) without minting authority ([ERR-002 notes](docs/requirements/domains/error_types/specs/ERR-002.md#implementation-notes)).
    #[error("coin minting: removed={removed}, added={added}")]
    CoinMinting { removed: u64, added: u64 },

    /// Header `total_fees` does not match summed fees from execution / receipts.
    #[error("fees mismatch: header={header}, computed={computed}")]
    FeesMismatch { header: u64, computed: u64 },

    /// Reserve-fee condition not satisfied by available fees in the bundle.
    #[error("reserve fee failed: required={required}, actual={actual}")]
    ReserveFeeFailed { required: u64, actual: u64 },

    /// Header `total_cost` does not match summed CLVM costs from spends.
    #[error("cost mismatch: header={header}, computed={computed}")]
    CostMismatch { header: u64, computed: u64 },

    // --- Tier 3: State validation (ERR-002) ---
    /// Proposer BLS signature over the block hash did not verify ([STV-006](docs/requirements/domains/state_validation/specs/STV-006.md)).
    #[error("invalid proposer signature")]
    InvalidProposerSignature,

    /// Looked up a block by hash (e.g. parent) that is not in the local view ([ERR-002 notes](docs/requirements/domains/error_types/specs/ERR-002.md#implementation-notes)).
    #[error("block not found: {0}")]
    NotFound(Bytes32),

    /// State transition Merkle root after applying removals/additions does not match header `state_root` ([STV-007](docs/requirements/domains/state_validation/specs/STV-007.md)).
    #[error("invalid state root: expected={expected}, computed={computed}")]
    InvalidStateRoot {
        expected: Bytes32,
        computed: Bytes32,
    },

    /// Removal references a coin id absent from [`CoinLookup`] and not ephemeral in-block ([STV-002](docs/requirements/domains/state_validation/specs/STV-002.md)).
    #[error("coin not found: {coin_id}")]
    CoinNotFound { coin_id: Bytes32 },

    /// Removal targets a coin already marked spent at `spent_height`.
    #[error("coin already spent: {coin_id} at height {spent_height}")]
    CoinAlreadySpent { coin_id: Bytes32, spent_height: u64 },

    /// Addition would create a coin id that already exists in the live coin set ([STV NORMATIVE](docs/requirements/domains/state_validation/NORMATIVE.md)).
    #[error("coin already exists: {coin_id}")]
    CoinAlreadyExists { coin_id: Bytes32 },
}

/// Checkpoint lifecycle failures: submission, scoring, epoch alignment, and finalization ([ERR-003](docs/requirements/domains/error_types/specs/ERR-003.md)).
///
/// **Design:** Checkpoints aggregate L2 state for an epoch and bridge to L1; errors here are orthogonal to per-block [`BlockError`]
/// ([ERR-003 implementation notes](docs/requirements/domains/error_types/specs/ERR-003.md#implementation-notes)).
///
/// **Usage:** Serialization failures should map to [`CheckpointError::InvalidData`] ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md));
/// validation layers return structured variants (`ScoreNotHigher`, `EpochMismatch`, etc.) per [checkpoint NORMATIVE](docs/requirements/domains/checkpoint/NORMATIVE.md).
///
/// **Derivation:** `Debug` + `Clone` + `thiserror::Error` — same ergonomics as [`BlockError`] ([SPEC §4.2](docs/resources/SPEC.md)).
#[derive(Debug, Clone, Error)]
pub enum CheckpointError {
    /// Bincode or field-level parse failure; also used for generic “bad checkpoint bytes” ([SER NORMATIVE](docs/requirements/domains/serialization/NORMATIVE.md)).
    #[error("invalid checkpoint data: {0}")]
    InvalidData(String),

    /// No checkpoint record for the given epoch (sync / indexer gap).
    #[error("checkpoint not found for epoch {0}")]
    NotFound(u64),

    /// Checkpoint failed semantic validation (Merkle root, signature set, etc.).
    #[error("invalid checkpoint: {0}")]
    Invalid(String),

    /// New submission does not beat the incumbent score ([ERR-003 notes](docs/requirements/domains/error_types/specs/ERR-003.md#implementation-notes)).
    #[error("score not higher: current={current}, submitted={submitted}")]
    ScoreNotHigher { current: u64, submitted: u64 },

    /// Submission epoch does not match the expected collecting epoch.
    #[error("epoch mismatch: expected={expected}, got={got}")]
    EpochMismatch { expected: u64, got: u64 },

    /// Checkpoint already committed; immutable ([ERR-003 notes](docs/requirements/domains/error_types/specs/ERR-003.md#implementation-notes)).
    #[error("checkpoint already finalized")]
    AlreadyFinalized,

    /// Finalize or query called before the epoch checkpoint process began.
    #[error("checkpoint process not started")]
    NotStarted,
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
