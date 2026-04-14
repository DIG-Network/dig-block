//! Shared test infrastructure for dig-block.
//!
//! Provides mock trait implementations and helper functions for constructing
//! test fixtures. Used across all integration test files.

use std::collections::HashMap;

use chia_bls::{sign, PublicKey, SecretKey, Signature};
use chia_protocol::{Bytes32, Coin, CoinSpend, CoinState, Program, SpendBundle};

use dig_block::traits::SignerError;
use dig_block::{BlockSigner, CoinLookup, Cost, L2Block, L2BlockHeader, EMPTY_ROOT};

// ---------------------------------------------------------------------------
// MockCoinLookup
// ---------------------------------------------------------------------------

/// Mock coin state lookup for testing validation logic.
pub struct MockCoinLookup {
    coins: HashMap<Bytes32, CoinState>,
    chain_height: u64,
    chain_timestamp: u64,
}

impl MockCoinLookup {
    /// Create an empty mock with height 0 and timestamp 0.
    pub fn new() -> Self {
        Self {
            coins: HashMap::new(),
            chain_height: 0,
            chain_timestamp: 0,
        }
    }

    /// Register a coin state for lookup by coin ID.
    pub fn add_coin_state(&mut self, coin_id: Bytes32, state: CoinState) {
        self.coins.insert(coin_id, state);
    }

    /// Set the chain height returned by `get_chain_height()`.
    pub fn set_chain_height(&mut self, height: u64) {
        self.chain_height = height;
    }

    /// Set the chain timestamp returned by `get_chain_timestamp()`.
    pub fn set_chain_timestamp(&mut self, timestamp: u64) {
        self.chain_timestamp = timestamp;
    }
}

impl CoinLookup for MockCoinLookup {
    fn get_coin_state(&self, coin_id: &Bytes32) -> Option<CoinState> {
        self.coins.get(coin_id).cloned()
    }

    fn get_chain_height(&self) -> u64 {
        self.chain_height
    }

    fn get_chain_timestamp(&self) -> u64 {
        self.chain_timestamp
    }
}

// ---------------------------------------------------------------------------
// MockBlockSigner
// ---------------------------------------------------------------------------

/// Deterministic BLS signer for testing block production.
pub struct MockBlockSigner {
    secret_key: SecretKey,
}

impl MockBlockSigner {
    /// Create a signer with a deterministic test key (seeded from fixed bytes).
    pub fn new() -> Self {
        // Deterministic 32-byte seed for reproducible tests.
        let seed: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ];
        let secret_key = SecretKey::from_seed(&seed);
        Self { secret_key }
    }

    /// Get the public key corresponding to this signer's secret key.
    pub fn public_key(&self) -> PublicKey {
        self.secret_key.public_key()
    }
}

impl BlockSigner for MockBlockSigner {
    fn sign_block(&self, header_hash: &Bytes32) -> Result<Signature, SignerError> {
        Ok(sign(&self.secret_key, header_hash.as_ref()))
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Create a test [`L2BlockHeader`] with sensible defaults (`height == 1`).
///
/// **Spec links:** [BLK-001](docs/requirements/domains/block_types/specs/BLK-001.md),
/// [BLK-002](docs/requirements/domains/block_types/specs/BLK-002.md) (`L2BlockHeader::new` via [`test_header_at_height`]).
pub fn test_header() -> L2BlockHeader {
    test_header_at_height(1)
}

/// Create a test [`L2BlockHeader`] at a given `height` (all other scalars minimal, roots empty).
///
/// Uses [`L2BlockHeader::new`](L2BlockHeader::new) (BLK-002) so `version` tracks height; leaves `timestamp` at
/// 0 as in SPEC-derived `new`. For a fixed non-zero timestamp, mutate the returned header in the test.
pub fn test_header_at_height(height: u64) -> L2BlockHeader {
    L2BlockHeader::new(
        height,
        0,
        Bytes32::new([0xee; 32]),
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        1,
        Bytes32::new([0xdd; 32]),
        0,
        0,
        0 as Cost,
        0,
        0,
        0,
        0,
        EMPTY_ROOT,
    )
}

/// Create a test L2Block containing a header and minimal SpendBundle.
///
/// Note: Returns a placeholder struct until BLK-003 adds real fields.
pub fn test_block() -> L2Block {
    // Placeholder — will be replaced in BLK-003 when L2Block has fields.
    L2Block::stub()
}

/// Create a minimal but structurally complete SpendBundle for testing.
///
/// The SpendBundle has one CoinSpend with a nil puzzle and nil solution.
/// It is not consensus-valid but is structurally complete.
pub fn test_spend_bundle() -> SpendBundle {
    let parent = Bytes32::default();
    let puzzle_hash = Bytes32::default();
    let amount = 1_000_000u64;

    let coin = Coin::new(parent, puzzle_hash, amount);

    // Nil CLVM programs: (q) = 0x01 (quote operator) encoding a nil value.
    let puzzle_reveal = Program::from(vec![0x01]);
    let solution = Program::from(vec![0x80]); // nil atom

    let coin_spend = CoinSpend::new(coin, puzzle_reveal, solution);

    SpendBundle::new(vec![coin_spend], Signature::default())
}

/// Create a test CoinState for use with MockCoinLookup.
pub fn test_coin_state(coin: Coin, created_height: u32, spent_height: Option<u32>) -> CoinState {
    CoinState {
        coin,
        created_height: Some(created_height),
        spent_height,
    }
}
