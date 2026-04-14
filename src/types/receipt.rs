//! Receipt domain types: [`ReceiptStatus`], [`Receipt`], [`ReceiptList`].
//!
//! ## Requirements trace
//!
//! - **[RCP-001](docs/requirements/domains/receipt/specs/RCP-001.md)** — [`ReceiptStatus`] discriminants `0..=4` and `255`.
//! - **[NORMATIVE § RCP-001](docs/requirements/domains/receipt/NORMATIVE.md)** — numeric representation for wire / Merkle.
//! - **[SPEC §2.9](docs/resources/SPEC.md)** — receipt payload context.
//! - **RCP-002 — RCP-004:** [`Receipt`] / [`ReceiptList`] behavior is specified in later requirements; placeholders remain until then.
//!
//! ## Rationale
//!
//! - **`#[repr(u8)]`:** Stable single-byte tags for bincode payloads and receipt Merkle leaves ([RCP-001](docs/requirements/domains/receipt/specs/RCP-001.md) implementation notes).
//! - **`Failed = 255`:** Leaves `5..=254` for future specific failure codes without renumbering existing wire values.
//! - **`ReceiptStatus::from_u8`:** Unknown bytes map to [`ReceiptStatus::Failed`] so forward-compatible decoders never panic (RCP-001 implementation notes).

use serde::{Deserialize, Serialize};

/// Outcome of applying one transaction in a block ([SPEC §2.9](docs/resources/SPEC.md), RCP-001).
///
/// **Wire:** Use [`Self::as_u8`] / [`Self::from_u8`] for deterministic `u8` ↔ enum mapping; serde derives are
/// retained for schema evolution ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)) and may be
/// tuned in SER-* tasks for integer tagging.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReceiptStatus {
    /// Transaction executed successfully.
    Success = 0,
    /// Sender balance insufficient for the transaction cost.
    InsufficientBalance = 1,
    /// Nonce did not match account sequence.
    InvalidNonce = 2,
    /// Cryptographic signature verification failed.
    InvalidSignature = 3,
    /// Sender account missing from state.
    AccountNotFound = 4,
    /// Generic or reserved-range execution failure (`255` — see module docs).
    Failed = 255,
}

impl ReceiptStatus {
    /// Discriminant as a single byte (same as `self as u8` with [`#[repr(u8)]`](ReceiptStatus)).
    #[inline]
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Decode a wire / stored byte into [`ReceiptStatus`].
    ///
    /// **Unknown values:** Any byte other than `0..=4` or `255` maps to [`Self::Failed`] so new failure codes
    /// can be introduced later without breaking old decoders (they will classify unknowns as failed execution).
    #[must_use]
    pub fn from_u8(byte: u8) -> Self {
        match byte {
            0 => Self::Success,
            1 => Self::InsufficientBalance,
            2 => Self::InvalidNonce,
            3 => Self::InvalidSignature,
            4 => Self::AccountNotFound,
            255 => Self::Failed,
            _ => Self::Failed,
        }
    }
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
