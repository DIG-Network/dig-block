//! Error enums for dig-block.
//!
//! - **Tier 1 [`BlockError`]** (structural): [ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md), [NORMATIVE § ERR-001](docs/requirements/domains/error_types/NORMATIVE.md).
//! - **Tier 2 /3 [`BlockError`]** (execution / state): [ERR-002](docs/requirements/domains/error_types/specs/ERR-002.md), [NORMATIVE § ERR-002](docs/requirements/domains/error_types/NORMATIVE.md).
//! - **[`CheckpointError`]** (checkpoints): [ERR-003](docs/requirements/domains/error_types/specs/ERR-003.md), [SPEC §4.2](docs/resources/SPEC.md).
//! - **[`BuilderError`]** (block construction): [ERR-004](docs/requirements/domains/error_types/specs/ERR-004.md), [SPEC §6.5](docs/resources/SPEC.md).
//! - **[`SignerBitmapError`] / [`ReceiptError`]**: [ERR-005](docs/requirements/domains/error_types/specs/ERR-005.md), [SPEC §4.3–4.4](docs/resources/SPEC.md).
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

    /// Header `version` does not match the height / DFSP activation rule ([SVL-001](docs/requirements/domains/structural_validation/specs/SVL-001.md)).
    #[error("invalid version: expected {expected}, got {actual}")]
    InvalidVersion { expected: u16, actual: u16 },

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

/// Block assembly failures while mutating [`crate::BlockBuilder`] (budgets, slash payloads, signing, v2 DFSP preconditions).
///
/// **Normative:** [ERR-004](docs/requirements/domains/error_types/specs/ERR-004.md), [SPEC §6.5](docs/resources/SPEC.md).
///
/// **Rationale:** These errors are raised *during construction* — before structural [`BlockError`] Tier 1 validation — so producers get
/// immediate feedback on limits defined in [BLK-005 `constants`](crate::constants) (`MAX_COST_PER_BLOCK`, `MAX_BLOCK_SIZE`, slash caps).
///
/// **Usage:** [`BlockBuilder::add_spend_bundle`](docs/resources/SPEC.md) maps overruns to [`BuilderError::CostBudgetExceeded`] /
/// [`BuilderError::SizeBudgetExceeded`]; slash path uses [`BuilderError::TooManySlashProposals`] / [`BuilderError::SlashProposalTooLarge`];
/// [`crate::BlockSigner`] failures map to [`BuilderError::SigningFailed`] ([block_production NORMATIVE](docs/requirements/domains/block_production/NORMATIVE.md)).
///
/// **Derivation:** `Debug` + `Clone` + `thiserror::Error` per ERR-004 acceptance criteria (Display + [`std::error::Error`] for `?`).
#[derive(Debug, Clone, Error)]
pub enum BuilderError {
    /// Cumulative CLVM cost would exceed the per-block budget after adding the candidate spend.
    #[error("cost budget exceeded: current={current}, addition={addition}, max={max}")]
    CostBudgetExceeded {
        current: u64,
        addition: u64,
        max: u64,
    },

    /// Serialized block size would exceed [`crate::MAX_BLOCK_SIZE`](crate::constants) after adding the candidate body bytes.
    #[error("size budget exceeded: current={current}, addition={addition}, max={max}")]
    SizeBudgetExceeded {
        current: u32,
        addition: u32,
        max: u32,
    },

    /// Slash proposal count would exceed [`crate::MAX_SLASH_PROPOSALS_PER_BLOCK`](crate::constants).
    #[error("too many slash proposals: max={max}")]
    TooManySlashProposals { max: u32 },

    /// One slash payload exceeds [`crate::MAX_SLASH_PROPOSAL_PAYLOAD_BYTES`](crate::constants).
    #[error("slash proposal too large: size={size}, max={max}")]
    SlashProposalTooLarge { size: u32, max: u32 },

    /// [`crate::BlockSigner`] rejected the block hash or could not produce a signature ([BLD-006](docs/requirements/domains/block_production/specs/BLD-006.md)).
    #[error("signing failed: {0}")]
    SigningFailed(String),

    /// Finalize called with no spend bundles — blocks must carry at least one user transaction bundle ([ERR-004 notes](docs/requirements/domains/error_types/specs/ERR-004.md#implementation-notes)).
    #[error("empty block: no spend bundles added")]
    EmptyBlock,

    /// Header `version >= VERSION_V2` but DFSP root fields were not supplied on the builder ([ERR-004 notes](docs/requirements/domains/error_types/specs/ERR-004.md#implementation-notes)).
    #[error("missing DFSP roots")]
    MissingDfspRoots,
}

/// Signer bitmap subsystem failures: index bounds, wire length, and validator-set cardinality ([ERR-005](docs/requirements/domains/error_types/specs/ERR-005.md)).
///
/// **Usage:** [`crate::SignerBitmap::set_signed`] returns [`SignerBitmapError::IndexOutOfBounds`] when `index >= validator_count`;
/// [`crate::SignerBitmap::merge`] returns [`SignerBitmapError::ValidatorCountMismatch`] when operands were sized for different sets
/// ([ATT-004](docs/requirements/domains/attestation/specs/ATT-004.md), [ATT-005](docs/requirements/domains/attestation/specs/ATT-005.md)).
///
/// **Rationale:** `TooManyValidators` and `InvalidLength` support deserialization / policy checks where the byte vector or declared
/// count disagrees with protocol limits ([ERR-005 implementation notes](docs/requirements/domains/error_types/specs/ERR-005.md#implementation-notes));
/// production paths may grow into these without changing the public enum shape again.
///
/// **Derivation:** `Debug` + `Clone` + `thiserror::Error`; `PartialEq` + `Eq` retained so ATT-004/ATT-005 integration tests can
/// compare structured errors without string parsing.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SignerBitmapError {
    /// `index` is not a valid signer slot; `max` is the bitmap's [`crate::SignerBitmap::validator_count`] (valid indices: `0..max`).
    #[error("index out of bounds: index={index}, max={max}")]
    IndexOutOfBounds { index: u32, max: u32 },

    /// Validator set size exceeds what the protocol or deployment allows (see [`crate::MAX_VALIDATORS`](crate::MAX_VALIDATORS)).
    #[error("too many validators: {0}")]
    TooManyValidators(usize),

    /// Byte length of a serialized bitmap does not match `ceil(validator_count / 8)` (or caller’s expected width).
    #[error("invalid bitmap length: expected={expected}, got={got}")]
    InvalidLength { expected: usize, got: usize },

    /// Two bitmap operands do not share the same [`crate::SignerBitmap::validator_count`] ([`crate::SignerBitmap::merge`]).
    #[error("validator count mismatch: expected={expected}, got={got}")]
    ValidatorCountMismatch { expected: u32, got: u32 },
}

/// Receipt list / indexer failures for execution receipts ([ERR-005](docs/requirements/domains/error_types/specs/ERR-005.md), [SPEC §4.4](docs/resources/SPEC.md)).
///
/// **Usage:** Serialization or field validation maps to [`ReceiptError::InvalidData`]; lookup by receipt or tx id uses [`ReceiptError::NotFound`]
/// when the key is absent ([RCP domain](docs/requirements/domains/receipt/NORMATIVE.md) — future RCP helpers).
///
/// **Derivation:** `Debug` + `Clone` + `thiserror::Error`; `PartialEq` + `Eq` for testability ([`Bytes32`] is compared by value).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ReceiptError {
    /// Opaque parse or semantic failure (e.g. bincode, bad status byte).
    #[error("invalid receipt data: {0}")]
    InvalidData(String),

    /// No receipt recorded for the given id / hash.
    #[error("receipt not found: {0}")]
    NotFound(Bytes32),
}
