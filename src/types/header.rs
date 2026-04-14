// L2BlockHeader struct and methods.
// Full implementation will be added in BLK-001 and BLK-002.

use serde::{Deserialize, Serialize};

/// L2 block header containing all block metadata and state commitments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2BlockHeader {
    // Placeholder field — will be replaced with real fields in BLK-001.
    pub(crate) _placeholder: (),
}

impl L2BlockHeader {
    /// Temporary stub constructor. Will be replaced in BLK-001/BLK-002.
    pub fn stub() -> Self {
        Self { _placeholder: () }
    }
}
