// Receipt, ReceiptList, ReceiptStatus types.
// Full implementation will be added in RCP-001 through RCP-004.

use serde::{Deserialize, Serialize};

/// Transaction execution outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReceiptStatus {
    Success = 0,
    InsufficientBalance = 1,
    InvalidNonce = 2,
    InvalidSignature = 3,
    AccountNotFound = 4,
    Failed = 255,
}

/// Individual transaction receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    _placeholder: (),
}

/// Ordered list of receipts with a Merkle root.
///
/// **ATT-001:** [`Default`] yields an empty placeholder so [`crate::AttestedBlock::new`] can be tested before
/// [RCP-003](docs/requirements/domains/receipt/specs/RCP-003.md) fills in real storage APIs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptList {
    _placeholder: (),
}
