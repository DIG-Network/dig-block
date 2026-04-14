// AttestedBlock struct (block + signer attestations).
// Full implementation will be added in ATT-001 and ATT-002.

use serde::{Deserialize, Serialize};

/// Block with validator attestation data (signer bitmap, aggregate signature, receipts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestedBlock {
    _placeholder: (),
}
