// External integration traits.
// Full implementation will be added in STR-004.

use chia_bls::Signature;
use chia_protocol::{Bytes32, CoinState};

/// Trait for looking up coin state during validation.
pub trait CoinLookup {
    /// Look up a coin's state by its ID.
    fn get_coin_state(&self, coin_id: &Bytes32) -> Option<CoinState>;
    /// Get the current chain tip height.
    fn get_chain_height(&self) -> u64;
    /// Get the current chain tip timestamp.
    fn get_chain_timestamp(&self) -> u64;
}

/// Error type for block signing operations.
#[derive(Debug, thiserror::Error)]
pub enum SignerError {
    #[error("signing failed: {0}")]
    SigningFailed(String),
}

/// Trait for signing block headers.
pub trait BlockSigner {
    /// Sign a block header hash, producing a BLS signature.
    fn sign_block(&self, header_hash: &Bytes32) -> Result<Signature, SignerError>;
}
