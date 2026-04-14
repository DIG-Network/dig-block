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
#[derive(Debug, Error)]
pub enum SignerBitmapError {
    /// Placeholder — variants will be added in ERR-005.
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
