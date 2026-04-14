//! Checkpoint domain types: [`Checkpoint`], [`CheckpointSubmission`] (submission filled in CKP-002+).
//!
//! ## Requirements trace
//!
//! - **[CKP-001](docs/requirements/domains/checkpoint/specs/CKP-001.md)** — [`Checkpoint`]: nine public fields + [`Checkpoint::new`] default instance.
//! - **[NORMATIVE § CKP-001](docs/requirements/domains/checkpoint/NORMATIVE.md)** — field names and types (`epoch`, roots, counts, fees).
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
//! - **`CheckpointSubmission` placeholder:** Retains serde derives so STR-003 / SER-001 scaffolding keeps compiling until [CKP-002](docs/requirements/domains/checkpoint/specs/CKP-002.md) replaces the stub.

use serde::{Deserialize, Serialize};

use crate::primitives::Bytes32;

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

/// Signed checkpoint submission for the competition ([CKP-002](docs/requirements/domains/checkpoint/specs/CKP-002.md), [CKP-005](docs/requirements/domains/checkpoint/specs/CKP-005.md)).
///
/// **Status:** Stub until CKP-002 defines fields; serde retained for API stability ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSubmission {
    _placeholder: (),
}
