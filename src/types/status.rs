//! Block and checkpoint lifecycle enums ([`BlockStatus`], [`CheckpointStatus`]).
//!
//! **ATT-003 — [`BlockStatus`]:** [ATT-003](docs/requirements/domains/attestation/specs/ATT-003.md) /
//! [NORMATIVE § ATT-003](docs/requirements/domains/attestation/NORMATIVE.md) /
//! [SPEC §2.5](docs/resources/SPEC.md).
//!
//! ## Usage
//!
//! Consensus / chain layers assign and transition status; this crate only defines the **labels** and
//! **read-only predicates** (`is_finalized`, `is_canonical`). State-machine enforcement lives outside
//! dig-block ([SPEC design note](docs/resources/SPEC.md) — status transitions are not encoded as a type-level FSM here).
//!
//! ## Rationale
//!
//! - **Bincode:** Both enums derive [`Serialize`] / [`Deserialize`] ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md))
//!   for `AttestedBlock` / checkpoint payloads.
//! - **Copy + Eq:** Small enums used in hot paths and equality checks; [`Hash`] supports bitmap/set keyed by status if needed.
//!
//! **CKP-003** will extend [`CheckpointStatus`] documentation when that requirement is implemented.

use serde::{Deserialize, Serialize};

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

/// Lifecycle status of a checkpoint submitted toward L1 (placeholder for CKP-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CheckpointStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
}
