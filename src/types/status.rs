//! Block and checkpoint lifecycle enums ([`BlockStatus`], [`CheckpointStatus`]).
//!
//! **ATT-003 — [`BlockStatus`]:** [ATT-003](docs/requirements/domains/attestation/specs/ATT-003.md) /
//! [NORMATIVE § ATT-003](docs/requirements/domains/attestation/NORMATIVE.md) /
//! [SPEC §2.5](docs/resources/SPEC.md).
//!
//! **CKP-003 — [`CheckpointStatus`]:** [CKP-003](docs/requirements/domains/checkpoint/specs/CKP-003.md) /
//! [NORMATIVE § CKP-003](docs/requirements/domains/checkpoint/NORMATIVE.md) /
//! [SPEC §2.8](docs/resources/SPEC.md).
//!
//! ## Usage
//!
//! Consensus / chain layers assign and transition status; this crate only defines the **labels** and
//! (for [`BlockStatus`]) **read-only predicates** (`is_finalized`, `is_canonical`). State-machine enforcement
//! lives outside dig-block ([SPEC design note](docs/resources/SPEC.md) — transitions are not a type-level FSM here).
//! [`CheckpointStatus`] carries winner identity in [`CheckpointStatus::WinnerSelected`] / [`CheckpointStatus::Finalized`].
//!
//! ## Rationale
//!
//! - **Bincode:** Both enums derive [`Serialize`] / [`Deserialize`] ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md))
//!   for `AttestedBlock` / checkpoint payloads.
//! - **Copy + Eq:** [`BlockStatus`] and [`CheckpointStatus`] use [`Copy`] where all payloads are [`Copy`] ([`crate::primitives::Bytes32`]).
//! - **Checkpoint ladder (informal):** Typical progression `Pending` → `Collecting` → `WinnerSelected` → `Finalized`, or terminal `Failed` ([CKP-003](docs/requirements/domains/checkpoint/specs/CKP-003.md) implementation notes).

use serde::{Deserialize, Serialize};

use crate::primitives::Bytes32;

/// Lifecycle status of an attested block ([SPEC §2.5](docs/resources/SPEC.md)).
///
/// **Semantics (ATT-003):** `Pending` is the initial constructor default for [`crate::AttestedBlock`] (ATT-001).
/// `SoftFinalized` means signing threshold met without L1 checkpoint; `HardFinalized` means L1-confirmed.
/// `Orphaned` / `Rejected` are non-canonical terminal classes for fork competition and validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockStatus {
    /// Awaiting validation / attestation progress.
    Pending,
    /// Structurally and execution-valid locally; not yet soft-final.
    Validated,
    /// Enough validator stake signed; not yet checkpointed on L1.
    SoftFinalized,
    /// Confirmed via L1 checkpoint / hard finality path.
    HardFinalized,
    /// Superseded on a competing fork (still on a side chain from this node’s view).
    Orphaned,
    /// Failed validation; must not be treated as canonical.
    Rejected,
}

impl BlockStatus {
    /// `true` iff this status represents **soft or hard** finality (ATT-003 / SPEC §2.5 derived methods).
    ///
    /// **Finality ladder:** Only [`Self::SoftFinalized`] and [`Self::HardFinalized`] count; `Validated` is explicitly
    /// non-final so callers can distinguish “passed validation” from “met stake threshold”.
    #[inline]
    pub fn is_finalized(&self) -> bool {
        matches!(self, Self::SoftFinalized | Self::HardFinalized)
    }

    /// `false` only for [`Self::Orphaned`] and [`Self::Rejected`] (non-canonical); `true` for all other variants.
    ///
    /// **Rationale:** Pending / Validated / finalized states may still appear on the canonical chain; orphaned and
    /// rejected blocks must be excluded from canonical progress metrics.
    #[inline]
    pub fn is_canonical(&self) -> bool {
        !matches!(self, Self::Orphaned | Self::Rejected)
    }
}

/// L2 checkpoint **epoch** lifecycle ([SPEC §2.8](docs/resources/SPEC.md), [CKP-003](docs/requirements/domains/checkpoint/specs/CKP-003.md)).
///
/// **Variants:** Three unit variants (`Pending`, `Collecting`, `Failed`) and two struct variants that pin the
/// winning checkpoint hash (`winner_hash`) — first with off-chain `winner_score`, then with on-chain `l1_height`
/// after L1 inclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CheckpointStatus {
    /// Epoch has not yet started checkpoint collection.
    Pending,
    /// Validators are submitting checkpoint proposals for the epoch.
    Collecting,
    /// Winning checkpoint chosen off-chain / in L2 consensus; not yet L1-confirmed.
    WinnerSelected {
        /// Identity (hash) of the winning checkpoint proposal.
        winner_hash: Bytes32,
        /// Score used for comparison during winner selection ([CKP-004](docs/requirements/domains/checkpoint/specs/CKP-004.md) will define computation).
        winner_score: u64,
    },
    /// Winning checkpoint confirmed on L1 at `l1_height`.
    Finalized {
        /// Same logical winner as in [`Self::WinnerSelected`] once finalized.
        winner_hash: Bytes32,
        /// L1 block height at which inclusion was observed.
        l1_height: u32,
    },
    /// Epoch checkpointing failed (e.g. insufficient participation).
    Failed,
}
