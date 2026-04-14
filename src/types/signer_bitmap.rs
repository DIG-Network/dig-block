// SignerBitmap for compact validator participation tracking.
// Full implementation will be added in ATT-004 and ATT-005.

use serde::{Deserialize, Serialize};

/// Compact bit array tracking which validators have signed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerBitmap {
    _placeholder: (),
}
