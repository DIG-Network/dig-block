use thiserror::Error;

/// Errors from block structural, execution, and state validation.
#[derive(Debug, Error)]
pub enum BlockError {
    /// Placeholder — variants will be added in ERR-001 and ERR-002.
    #[error("block error: {0}")]
    Other(String),
}

/// Errors from checkpoint operations.
#[derive(Debug, Error)]
pub enum CheckpointError {
    /// Placeholder — variants will be added in ERR-003.
    #[error("checkpoint error: {0}")]
    Other(String),
}

/// Errors from block/checkpoint builder operations.
#[derive(Debug, Error)]
pub enum BuilderError {
    /// Placeholder — variants will be added in ERR-004.
    #[error("builder error: {0}")]
    Other(String),
}

/// Errors from SignerBitmap operations.
///
/// - **ATT-004:** [`SignerBitmapError::IndexOutOfBounds`] — [`crate::SignerBitmap::set_signed`] when `index >= validator_count`.
/// - **ATT-005:** [`SignerBitmapError::ValidatorCountMismatch`] — [`crate::SignerBitmap::merge`] when counts differ.
///
/// Further variants: [ERR-005](docs/requirements/domains/error_types/specs/ERR-005.md).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SignerBitmapError {
    /// Validator index is not in `[0, validator_count)`.
    #[error("validator index out of bounds for this bitmap")]
    IndexOutOfBounds,
    /// [`crate::SignerBitmap::merge`] requires both operands to use the same `validator_count`.
    #[error("signer bitmap validator_count mismatch")]
    ValidatorCountMismatch,
    /// Placeholder — variants will be expanded in ERR-005.
    #[error("signer bitmap error: {0}")]
    Other(String),
}

/// Errors from Receipt operations.
#[derive(Debug, Error)]
pub enum ReceiptError {
    /// Placeholder — variants will be added in ERR-005.
    #[error("receipt error: {0}")]
    Other(String),
}
