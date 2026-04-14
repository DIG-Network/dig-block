// BlockStatus and CheckpointStatus enumerations.
// Full implementation will be added in ATT-003 and CKP-003.

use serde::{Deserialize, Serialize};

/// Lifecycle status of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockStatus {
    Pending,
    Validated,
    SoftFinalized,
    HardFinalized,
    Orphaned,
    Rejected,
}

/// Lifecycle status of a checkpoint on L1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CheckpointStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
}
