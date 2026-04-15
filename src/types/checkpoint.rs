//! Checkpoint domain types: [`Checkpoint`] (CKP-001), [`CheckpointSubmission`] (CKP-002).
//!
//! ## Requirements trace
//!
//! - **[CKP-001](docs/requirements/domains/checkpoint/specs/CKP-001.md)** — [`Checkpoint`]: nine public fields + [`Checkpoint::new`] default instance.
//! - **[CKP-002](docs/requirements/domains/checkpoint/specs/CKP-002.md)** — [`CheckpointSubmission`]: checkpoint + [`crate::SignerBitmap`] + aggregate BLS + score + submitter + L1 tracking options.
//! - **[NORMATIVE § CKP-001 / CKP-002](docs/requirements/domains/checkpoint/NORMATIVE.md)** — checkpoint + submission field layouts and constructors.
//! - **[SPEC §2.6](docs/resources/SPEC.md)** — checkpoint as epoch summary anchored toward L1.
//! - **[CKP-004](docs/requirements/domains/checkpoint/specs/CKP-004.md)** — [`Checkpoint::compute_score`]: `stake_percentage * block_count` (epoch competition score).
//! - **[CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md)** — [`CheckpointSubmission`]: [`CheckpointSubmission::hash`], [`CheckpointSubmission::epoch`], threshold helpers, L1 [`CheckpointSubmission::record_submission`].
//! - **[CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md)** — [`crate::CheckpointBuilder`] accumulates block / withdrawal hashes and produces `block_root` / `withdrawals_root` using the same internal Merkle helper as BLK-004 (`merkle_tree_root` in `merkle_util.rs`).
//! - **[HSH-002](docs/requirements/domains/hashing/specs/HSH-002.md)** / **[SPEC §3.2](docs/resources/SPEC.md)** — [`Checkpoint::hash`]: SHA-256 over 160-byte fixed-order preimage ([`chia_sha2::Sha256`]).
//! - **[SER-001](docs/requirements/domains/serialization/specs/SER-001.md)** — bincode via [`Serialize`] / [`Deserialize`] on wire-bearing structs.
//! - **[SER-002](docs/requirements/domains/serialization/specs/SER-002.md)** — [`Checkpoint::to_bytes`] / [`Checkpoint::from_bytes`] and [`CheckpointSubmission::to_bytes`] / [`CheckpointSubmission::from_bytes`]
//!   with [`CheckpointError::InvalidData`](crate::CheckpointError::InvalidData) on decode failures.
//!
//! ## Rationale
//!
//! - **Public fields:** Same ergonomics as [`crate::types::receipt::Receipt`] — consensus / builder layers assign values; this crate stays a typed bag of record ([CKP-001](docs/requirements/domains/checkpoint/specs/CKP-001.md) acceptance: read/write access).
//! - **Default roots:** [`Bytes32::default`] is the all-zero hash, matching “empty Merkle” conventions used elsewhere ([`crate::constants::EMPTY_ROOT`] is the documented empty-tree sentinel; callers may normalize roots when building real checkpoints — CKP-006).
//! - **`CheckpointSubmission` + L1 options:** `submission_height` / `submission_coin` start [`None`] at construction;
//!   [`CheckpointSubmission::record_submission`](CheckpointSubmission::record_submission) persists L1 proof ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md), [CKP-002](docs/requirements/domains/checkpoint/specs/CKP-002.md) notes).

use chia_sha2::Sha256;
use serde::{Deserialize, Serialize};

use super::signer_bitmap::SignerBitmap;
use crate::error::CheckpointError;
use crate::primitives::{Bytes32, PublicKey, Signature};

/// Epoch summary checkpoint: aggregate stats and Merkle roots for one L1-anchored epoch ([SPEC §2.6](docs/resources/SPEC.md), [CKP-001](docs/requirements/domains/checkpoint/specs/CKP-001.md)).
///
/// ## Field semantics (informal)
///
/// - **`epoch`:** Monotonic epoch id this summary closes.
/// - **`state_root`:** Post-epoch L2 state commitment.
/// - **`block_root`:** Merkle root over block hashes in the epoch ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md) will define construction).
/// - **`block_count` / `tx_count` / `total_fees`:** Scalar aggregates for light verification and scoring; `block_count` is the block factor in [`compute_score`](Checkpoint::compute_score) ([CKP-004](docs/requirements/domains/checkpoint/specs/CKP-004.md)).
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

    /// Competition score: `stake_percentage * block_count` ([CKP-004](docs/requirements/domains/checkpoint/specs/CKP-004.md), [NORMATIVE § CKP-004](docs/requirements/domains/checkpoint/NORMATIVE.md)).
    ///
    /// ## Parameters
    ///
    /// - **`stake_percentage`:** Integer stake share for the submitter, typically `0..=100` ([CKP-004](docs/requirements/domains/checkpoint/specs/CKP-004.md) implementation notes). The type is `u64` so callers can scale fixed-point stake if needed without API churn.
    ///
    /// ## Returns
    ///
    /// Product in `u64`. Favors checkpoints with more blocks and higher backing stake. **Overflow:** Uses ordinary
    /// `u64` multiplication (wraps on overflow; debug builds may panic on overflow — keep inputs within protocol bounds,
    /// e.g. epoch length ×100).
    #[must_use]
    pub fn compute_score(&self, stake_percentage: u64) -> u64 {
        stake_percentage * u64::from(self.block_count)
    }

    /// Byte length of the SHA-256 preimage for [`Self::hash`] ([SPEC §3.2](docs/resources/SPEC.md): 8+32+32+4+8+8+32+32+4).
    pub const HASH_PREIMAGE_LEN: usize = 160;

    /// Fixed-order **160-byte** preimage for [HSH-002](docs/requirements/domains/hashing/specs/HSH-002.md) /
    /// [SPEC §3.2](docs/resources/SPEC.md) (same bytes as fed to [`Self::hash`]).
    ///
    /// **Order:** `epoch`, `state_root`, `block_root`, `block_count`, `tx_count`, `total_fees`, `prev_checkpoint`,
    /// `withdrawals_root`, `withdrawal_count`.
    ///
    /// **Note:** [HSH-002 spec](docs/requirements/domains/hashing/specs/HSH-002.md) pseudocode lists some counts as `u64`;
    /// the wire table in SPEC §3.2 and this struct use `u32` LE for `block_count` and `withdrawal_count` (4 bytes each).
    #[must_use]
    pub fn hash_preimage_bytes(&self) -> [u8; Self::HASH_PREIMAGE_LEN] {
        fn put(buf: &mut [u8; Checkpoint::HASH_PREIMAGE_LEN], i: &mut usize, bytes: &[u8]) {
            buf[*i..*i + bytes.len()].copy_from_slice(bytes);
            *i += bytes.len();
        }
        let mut buf = [0u8; Self::HASH_PREIMAGE_LEN];
        let mut i = 0usize;
        put(&mut buf, &mut i, &self.epoch.to_le_bytes());
        put(&mut buf, &mut i, self.state_root.as_ref());
        put(&mut buf, &mut i, self.block_root.as_ref());
        put(&mut buf, &mut i, &self.block_count.to_le_bytes());
        put(&mut buf, &mut i, &self.tx_count.to_le_bytes());
        put(&mut buf, &mut i, &self.total_fees.to_le_bytes());
        put(&mut buf, &mut i, self.prev_checkpoint.as_ref());
        put(&mut buf, &mut i, self.withdrawals_root.as_ref());
        put(&mut buf, &mut i, &self.withdrawal_count.to_le_bytes());
        debug_assert_eq!(i, Self::HASH_PREIMAGE_LEN);
        buf
    }

    /// Canonical checkpoint identity: SHA-256 over [`Self::hash_preimage_bytes`] ([SPEC §3.2](docs/resources/SPEC.md)).
    ///
    /// **Encoding:** `epoch`, `tx_count`, `total_fees` as `u64` LE; `block_count`, `withdrawal_count` as `u32` LE
    /// (4 bytes each); four [`Bytes32`] roots as raw 32-byte slices ([HSH-002](docs/requirements/domains/hashing/specs/HSH-002.md),
    /// [NORMATIVE § HSH-002](docs/requirements/domains/hashing/NORMATIVE.md)).
    ///
    /// **Primitive:** [`Sha256`] from `chia-sha2` only (project crypto rules). [`CheckpointSubmission::hash`](CheckpointSubmission::hash) delegates here ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md)).
    #[must_use]
    pub fn hash(&self) -> Bytes32 {
        let mut hasher = Sha256::new();
        hasher.update(self.hash_preimage_bytes());
        Bytes32::new(hasher.finalize())
    }

    /// Serialize checkpoint summary to **bincode** bytes ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md), SPEC §8.2).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Checkpoint serialization should never fail")
    }

    /// Deserialize a checkpoint from **bincode** bytes ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md)).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CheckpointError> {
        bincode::deserialize(bytes).map_err(|e| CheckpointError::InvalidData(e.to_string()))
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
/// - **`score`:** Competition score, often populated from [`Checkpoint::compute_score`](Checkpoint::compute_score) ([CKP-004](docs/requirements/domains/checkpoint/specs/CKP-004.md)).
/// - **`submitter`:** Validator **index** in the epoch set who published this submission ([CKP-002](docs/requirements/domains/checkpoint/specs/CKP-002.md) implementation notes).
/// - **`submission_height` / `submission_coin`:** L1 observation metadata; [`None`] until [`record_submission`](CheckpointSubmission::record_submission).
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
    /// **`submission_height` / `submission_coin`:** Initialized to [`None`]; use [`Self::record_submission`] after L1
    /// confirmation ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md)).
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

    /// Delegates to [`Checkpoint::hash`] — canonical epoch-summary identity ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md), HSH-002).
    #[must_use]
    pub fn hash(&self) -> Bytes32 {
        self.checkpoint.hash()
    }

    /// Epoch number from the wrapped [`Checkpoint`] ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md)).
    #[must_use]
    pub fn epoch(&self) -> u64 {
        self.checkpoint.epoch
    }

    /// Validator participation as an integer percent `0..=100` — delegates to [`SignerBitmap::signing_percentage`] ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md), ATT-004).
    #[must_use]
    pub fn signing_percentage(&self) -> u64 {
        self.signer_bitmap.signing_percentage()
    }

    /// `true` iff [`Self::signing_percentage`] `>= threshold_pct` ([`SignerBitmap::has_threshold`](crate::SignerBitmap::has_threshold), CKP-005).
    #[must_use]
    pub fn meets_threshold(&self, threshold_pct: u64) -> bool {
        self.signer_bitmap.has_threshold(threshold_pct)
    }

    /// Record L1 inclusion: block height and submission coin id ([CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md)).
    ///
    /// **Normative:** Both fields become [`Some`]; [`Self::is_submitted`] becomes `true` because `submission_height` is set.
    pub fn record_submission(&mut self, height: u32, coin_id: Bytes32) {
        self.submission_height = Some(height);
        self.submission_coin = Some(coin_id);
    }

    /// `true` once `submission_height` is [`Some`] ([NORMATIVE § CKP-005](docs/requirements/domains/checkpoint/NORMATIVE.md)).
    #[must_use]
    pub fn is_submitted(&self) -> bool {
        self.submission_height.is_some()
    }

    /// Serialize submission (checkpoint + attestation material) to **bincode** bytes ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md)).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("CheckpointSubmission serialization should never fail")
    }

    /// Deserialize a submission from **bincode** bytes ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md)).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CheckpointError> {
        bincode::deserialize(bytes).map_err(|e| CheckpointError::InvalidData(e.to_string()))
    }
}
