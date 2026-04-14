//! STR-005: Shared test infrastructure — `tests/common.rs` mocks and fixtures ([spec](docs/requirements/domains/crate_structure/specs/STR-005.md)).
//!
//! ## What this proves
//!
//! - **MockCoinLookup:** Register and fetch [`CoinState`], chain height/timestamp mutators — used by future STV/EXE tests ([`CoinLookup`]).
//! - **MockBlockSigner:** Deterministic BLS key; [`sign_block`] + [`chia_bls::verify`] round-trip.
//! - **Fixtures:** `test_header`, `test_header_at_height`, `test_block`, `test_spend_bundle` produce structurally populated values for BLK-* integration tests without duplicating setup.

mod common;

use chia_bls::verify;
use chia_protocol::{Bytes32, Coin};

use dig_block::{BlockSigner, CoinLookup};

use common::{
    test_block, test_coin_state, test_header, test_header_at_height, test_spend_bundle,
    MockBlockSigner, MockCoinLookup,
};

#[test]
fn mock_coin_lookup_basic() {
    let mut lookup = MockCoinLookup::new();

    let coin = Coin::new(Bytes32::default(), Bytes32::default(), 1000);
    let coin_id = coin.coin_id();
    let state = test_coin_state(coin, 10, None);

    lookup.add_coin_state(coin_id, state);

    let result = lookup.get_coin_state(&coin_id);
    assert!(result.is_some());
    let retrieved = result.unwrap();
    assert_eq!(retrieved.coin.amount, 1000);
    assert_eq!(retrieved.created_height, Some(10));
    assert_eq!(retrieved.spent_height, None);
}

#[test]
fn mock_coin_lookup_missing() {
    let lookup = MockCoinLookup::new();
    let unknown_id = Bytes32::new([0xff; 32]);
    assert!(lookup.get_coin_state(&unknown_id).is_none());
}

#[test]
fn mock_coin_lookup_height() {
    let mut lookup = MockCoinLookup::new();
    assert_eq!(lookup.get_chain_height(), 0);

    lookup.set_chain_height(12345);
    assert_eq!(lookup.get_chain_height(), 12345);
}

#[test]
fn mock_coin_lookup_timestamp() {
    let mut lookup = MockCoinLookup::new();
    assert_eq!(lookup.get_chain_timestamp(), 0);

    lookup.set_chain_timestamp(1_700_000_000);
    assert_eq!(lookup.get_chain_timestamp(), 1_700_000_000);
}

#[test]
fn mock_block_signer_sign() {
    let signer = MockBlockSigner::new();
    let hash = Bytes32::new([0xab; 32]);
    let result = signer.sign_block(&hash);
    assert!(result.is_ok());
}

#[test]
fn mock_block_signer_verify() {
    let signer = MockBlockSigner::new();
    let hash = Bytes32::new([0xcd; 32]);
    let sig = signer.sign_block(&hash).unwrap();
    let pk = signer.public_key();

    // Verify the signature against the public key and message.
    assert!(verify(&sig, &pk, hash.as_ref()));
}

#[test]
fn test_header_valid() {
    let header = test_header();
    assert_eq!(header.height, 1);
    assert_eq!(header.version, dig_block::VERSION_V1);
}

#[test]
fn test_header_height() {
    let header = test_header_at_height(42);
    assert_eq!(header.height, 42);
}

#[test]
fn test_block_valid() {
    // test_block() returns a structurally populated L2Block (BLK-003).
    let block = test_block();
    let _ = format!("{:?}", block);
}

#[test]
fn test_spend_bundle_valid() {
    let bundle = test_spend_bundle();
    // SpendBundle must have at least one CoinSpend.
    assert!(
        !bundle.coin_spends.is_empty(),
        "test_spend_bundle must have at least one CoinSpend"
    );
    // Verify the coin has a non-zero amount.
    assert!(bundle.coin_spends[0].coin.amount > 0);
}
