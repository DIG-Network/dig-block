//! STR-004: [`CoinLookup`] and [`BlockSigner`] trait definitions ([spec](docs/requirements/domains/crate_structure/specs/STR-004.md), [SPEC §7.2](docs/resources/SPEC.md)).
//!
//! ## What this proves
//!
//! - **Surface:** Traits expose chain context (`get_chain_height`, `get_chain_timestamp`) and coin state lookup without a custom coin record type — [`chia_protocol::CoinState`] is used directly ([start.md](docs/prompt/start.md) hard requirements).
//! - **Object safety:** `Box<dyn CoinLookup>` / `Box<dyn BlockSigner>` can be built so validation code can take trait objects.
//! - **Signing hook:** [`BlockSigner::sign_block`] returns `chia_bls::Signature` or [`dig_block::traits::SignerError`].

use chia_bls::Signature;
use chia_protocol::{Bytes32, CoinState};
use dig_block::traits::SignerError;
use dig_block::{BlockSigner, CoinLookup};

/// Minimal local mock — distinct from `tests/common::MockCoinLookup` (STR-005 full fixture).
struct MockCoinLookup {
    height: u64,
    timestamp: u64,
}

impl CoinLookup for MockCoinLookup {
    fn get_coin_state(&self, _coin_id: &Bytes32) -> Option<CoinState> {
        None
    }

    fn get_chain_height(&self) -> u64 {
        self.height
    }

    fn get_chain_timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// Minimal local mock for object-safety checks.
struct MockBlockSigner;

impl BlockSigner for MockBlockSigner {
    fn sign_block(&self, _header_hash: &Bytes32) -> Result<Signature, SignerError> {
        Ok(Signature::default())
    }
}

#[test]
fn coin_lookup_defined() {
    let mock = MockCoinLookup {
        height: 100,
        timestamp: 1_700_000_000,
    };
    assert_eq!(mock.get_chain_height(), 100);
    assert_eq!(mock.get_chain_timestamp(), 1_700_000_000);
    assert!(mock.get_coin_state(&Bytes32::default()).is_none());
}

#[test]
fn block_signer_defined() {
    let signer = MockBlockSigner;
    let result = signer.sign_block(&Bytes32::default());
    assert!(result.is_ok());
}

#[test]
fn coin_lookup_object_safe() {
    let mock = MockCoinLookup {
        height: 42,
        timestamp: 0,
    };
    // Object safety: can create a trait object.
    let boxed: Box<dyn CoinLookup> = Box::new(mock);
    assert_eq!(boxed.get_chain_height(), 42);
}

#[test]
fn block_signer_object_safe() {
    let signer = MockBlockSigner;
    // Object safety: can create a trait object.
    let boxed: Box<dyn BlockSigner> = Box::new(signer);
    assert!(boxed.sign_block(&Bytes32::default()).is_ok());
}

#[test]
fn uses_chia_coinstate() {
    // Verify the return type is chia_protocol::CoinState.
    let mock = MockCoinLookup {
        height: 1,
        timestamp: 1,
    };
    let result: Option<CoinState> = mock.get_coin_state(&Bytes32::default());
    assert!(result.is_none());
}

#[test]
fn mock_implements_traits() {
    let lookup = MockCoinLookup {
        height: 500,
        timestamp: 1_600_000_000,
    };
    let signer = MockBlockSigner;

    // CoinLookup methods return expected values.
    assert_eq!(lookup.get_chain_height(), 500);
    assert_eq!(lookup.get_chain_timestamp(), 1_600_000_000);
    assert!(lookup.get_coin_state(&Bytes32::default()).is_none());

    // BlockSigner returns a valid signature.
    let sig = signer.sign_block(&Bytes32::default()).unwrap();
    assert_eq!(sig, Signature::default());
}
