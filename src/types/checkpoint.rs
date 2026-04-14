// Checkpoint and CheckpointSubmission structs.
// Full implementation will be added in CKP-001 through CKP-005.

use serde::{Deserialize, Serialize};

/// Epoch summary checkpoint submitted to L1 for hard finality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    _placeholder: (),
}

/// Signed checkpoint submission for the competition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSubmission {
    _placeholder: (),
}
