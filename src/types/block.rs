// L2Block struct (header + SpendBundles + proposer_signature).
// Full implementation will be added in BLK-003 and BLK-004.

use serde::{Deserialize, Serialize};

/// Complete L2 block: header plus transaction body and proposer signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Block {
    // Placeholder field — will be replaced with real fields in BLK-003.
    pub(crate) _placeholder: (),
}

impl L2Block {
    /// Temporary stub constructor. Will be replaced in BLK-003.
    pub fn stub() -> Self {
        Self { _placeholder: () }
    }
}
