//! Checkpoint domain types: [`Checkpoint`] (CKP-001), [`CheckpointSubmission`] (CKP-002).
//!
//! ## Requirements trace
//!
//! - **[CKP-001](docs/requirements/domains/checkpoint/specs/CKP-001.md)** — [`Checkpoint`]: nine public fields + [`Checkpoint::new`] default instance.
//! - **[CKP-002](docs/requirements/domains/checkpoint/specs/CKP-002.md)** — [`CheckpointSubmission`]: checkpoint + [`crate::SignerBitmap`] + aggregate BLS + score + submitter + L1 tracking options.
//! - **[NORMATIVE § CKP-001 / CKP-002](docs/requirements/domains/checkpoint/NORMATIVE.md)** — checkpoint + submission field layouts and constructors.
//! - **[SPEC §2.6](docs/resources/SPEC.md)** — checkpoint as epoch summary anchored toward L1.
//! - **[CKP-004](docs/requirements/domains/checkpoint/specs/CKP-004.md)** (future) — [`Checkpoint::compute_score`] will use `block_count` × stake.
//! - **[CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md)** (future) — [`crate::builder::checkpoint_builder::CheckpointBuilder`] will populate `block_root` / `withdrawals_root` as Merkle roots over the epoch.
//! - **[HSH-002](docs/requirements/domains/hashing/specs/HSH-002.md)** (future) — fixed-order SHA-256 over the nine fields (160 bytes LE + hashes).
//! - **[SER-001](docs/requirements/domains/serialization/specs/SER-001.md)** — bincode via [`Serialize`] / [`Deserialize`] on wire-bearing structs.
//!
//! ## Rationale
//!
//! - **Public fields:** Same ergonomics as [`crate::types::receipt::Receipt`] — consensus / builder layers assign values; this crate stays a typed bag of record ([CKP-001](docs/requirements/domains/checkpoint/specs/CKP-001.md) acceptance: read/write access).
//! - **Default roots:** [`Bytes32::default`] is the all-zero hash, matching “empty Merkle” conventions used elsewhere ([`crate::constants::EMPTY_ROOT`] is the documented empty-tree sentinel; callers may normalize roots when building real checkpoints — CKP-006).
//! - **`CheckpointSubmission` + L1 options:** `submission_height` / `submission_coin` start [`None`] at construction;
//!   [CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md) records L1 inclusion after submission ([CKP-002](docs/requirements/domains/checkpoint/specs/CKP-002.md) implementation notes).

use serde::{Deserialize, Serialize};

use super::signer_bitmap::SignerBitmap;
use crate::primitives::{Bytes32, PublicKey, Signature};

/// Epoch summary checkpoint: aggregate stats and Merkle roots for one L1-anchored epoch ([SPEC §2.6](docs/resources/SPEC.md), [CKP-001](docs/requirements/domains/checkpoint/specs/CKP-001.md)).
///
/// ## Field semantics (informal)
///
/// - **`epoch`:** Monotonic epoch id this summary closes.
/// - **`state_root`:** Post-epoch L2 state commitment.
/// - **`block_root`:** Merkle root over block hashes in the epoch ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md) will define construction).
/// - **`block_count` / `tx_count` / `total_fees`:** Scalar aggregates for light verification and scoring ([CKP-004](docs/requirements/domains/checkpoint/specs/CKP-004.md)).
/// - **`prev_checkpoint`:** Hash / identity of the prior checkpoint header for chained verification ([CKP-001](docs/requirements/domains/checkpoint/specs/CKP-001.md) implementation notes).
/// - **`withdrawals_root` / `withdrawal_count`:** Merkle root and count over withdrawal records in the epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Epoch number this checkpoint summarizes.
    pub epoch: u64,
    /// L2 state root after applying all blocks in the epoch.
    pub state_root: Bytes32,
    /// Merkle root over block hashes included in this epoch.
    pub block_root: Bytes32,
    /// Number of L2 blocks in the epoch.
    pub block_count: u32,
    /// Total transactions across blocks in the epoch.
    pub tx_count: u64,
    /// Sum of fees collected in the epoch (same currency unit as receipt fees — see RCP-*).
    pub total_fees: u64,
    /// Link to the previous checkpoint (continuity for fraud/validity proofs).
    pub prev_checkpoint: Bytes32,
    /// Merkle root over withdrawal hashes in the epoch.
    pub withdrawals_root: Bytes32,
    /// Number of withdrawals in the epoch.
    pub withdrawal_count: u32,
}

impl Checkpoint {
    /// Default empty checkpoint: epoch `0`, zero counts, all roots [`Bytes32::default`] ([CKP-001](docs/requirements/domains/checkpoint/specs/CKP-001.md) constructor spec).
    ///
    /// **Usage:** [`crate::builder::checkpoint_builder::CheckpointBuilder`] and tests start from this value then overwrite fields; production checkpoints should set non-zero roots before signing (CKP-005 / HSH-002).
    #[must_use]
    pub fn new() -> Self {
        Self {
            epoch: 0,
            state_root: Bytes32::default(),
            block_root: Bytes32::default(),
            block_count: 0,
            tx_count: 0,
            total_fees: 0,
            prev_checkpoint: Bytes32::default(),
            withdrawals_root: Bytes32::default(),
            withdrawal_count: 0,
        }
    }
}

impl Default for Checkpoint {
    fn default() -> Self {
        Self::new()
    }
}

/// Signed checkpoint submission: epoch summary plus validator attestation material ([SPEC §2.7](docs/resources/SPEC.md), [CKP-002](docs/requirements/domains/checkpoint/specs/CKP-002.md)).
///
/// ## Field semantics
///
/// - **`checkpoint`:** The [`Checkpoint`] being proposed for L1 anchoring (CKP-001).
/// - **`signer_bitmap` / `aggregate_signature` / `aggregate_pubkey`:** Who attested and the aggregated BLS proof
///   over the checkpoint preimage (exact signing protocol is outside this crate; types match ATT-001 / ATT-004 patterns).
/// - **`score`:** Competition score, typically from `Checkpoint::compute_score` once [CKP-004](docs/requirements/domains/checkpoint/specs/CKP-004.md) lands.
/// - **`submitter`:** Validator **index** in the epoch set who published this submission ([CKP-002](docs/requirements/domains/checkpoint/specs/CKP-002.md) implementation notes).
/// - **`submission_height` / `submission_coin`:** L1 observation metadata; [`None`] until CKP-005 `record_submission` runs.
///
/// **Serialization:** [`Serialize`] / [`Deserialize`] for bincode ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSubmission {
    /// Epoch summary being submitted.
    pub checkpoint: Checkpoint,
    /// Which validators signed this submission ([`SignerBitmap`], ATT-004).
    pub signer_bitmap: SignerBitmap,
    /// Aggregated BLS signature over the checkpoint commitment.
    pub aggregate_signature: Signature,
    /// Aggregated BLS public key corresponding to `aggregate_signature`.
    pub aggregate_pubkey: PublicKey,
    /// Off-chain / protocol score used to compare competing submissions ([CKP-004](docs/requirements/domains/checkpoint/specs/CKP-004.md)).
    pub score: u64,
    /// Index of the validator who broadcast this submission.
    pub submitter: u32,
    /// L1 block height where the submission transaction was observed, once recorded ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md)).
    pub submission_height: Option<u32>,
    /// L1 coin ID for the submission transaction, once recorded ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md)).
    pub submission_coin: Option<Bytes32>,
}

impl CheckpointSubmission {
    /// Build a submission with attestation material but **no** L1 inclusion data yet ([CKP-002](docs/requirements/domains/checkpoint/specs/CKP-002.md)).
    ///
    /// **`submission_height` / `submission_coin`:** Initialized to [`None`]; CKP-005 `record_submission` will persist
    /// height and coin id once that API exists.
    #[must_use]
    pub fn new(
        checkpoint: Checkpoint,
        signer_bitmap: SignerBitmap,
        aggregate_signature: Signature,
        aggregate_pubkey: PublicKey,
        score: u64,
        submitter: u32,
    ) -> Self {
        Self {
            checkpoint,
            signer_bitmap,
            aggregate_signature,
            aggregate_pubkey,
            score,
            submitter,
            submission_height: None,
            submission_coin: None,
        }
    }
}
