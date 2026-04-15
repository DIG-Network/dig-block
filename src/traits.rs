//! External integration traits: [`CoinLookup`] and [`BlockSigner`].
//!
//! ## Requirements trace
//!
//! - **[STR-004](docs/requirements/domains/crate_structure/specs/STR-004.md)** — trait definitions, method signatures, object safety.
//! - **[NORMATIVE § STR-004](docs/requirements/domains/crate_structure/NORMATIVE.md)** — `CoinLookup` uses `chia-protocol::CoinState` directly;
//!   `BlockSigner` returns `chia-bls::Signature`.
//! - **[SPEC §7.2](docs/resources/SPEC.md)** — validation context traits.
//!
//! ## Design decisions
//!
//! - **[`CoinLookup`] returns `Option<CoinState>`, not `Result`:** A missing coin is a normal validation
//!   condition (Tier 3 checks for ephemeral coins — STV-002), not an I/O error. Implementors that wrap a
//!   database should map DB errors to `None` or propagate them through a higher-level API.
//!
//! - **[`CoinState`] from `chia-protocol`:** Reuses the same type Chia's peer protocol returns for
//!   `register_for_coin_updates` responses ([`chia_protocol::CoinState`]). This means `dig-coinstore` can
//!   implement [`CoinLookup`] without type conversion.
//!   Reference: [`chia-blockchain/chia/protocols/wallet_protocol.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/protocols/wallet_protocol.py).
//!
//! - **[`BlockSigner`] is object-safe:** Both traits can be used as `dyn CoinLookup` / `dyn BlockSigner`
//!   so validation and builder code can be generic over the signing backend (HSM, in-memory key, remote signer).
//!
//! - **[`SignerError`]:** Kept minimal (one variant) because the signing failure reason is opaque to
//!   block validation — it only matters for logging. [`crate::BlockBuilder::build`](crate::BlockBuilder::build) maps
//!   failures to [`BuilderError::SigningFailed`](crate::BuilderError::SigningFailed) using [`SignerError::to_string`]
//!   (BLD-006 / [ERR-004](docs/requirements/domains/error_types/specs/ERR-004.md) — the `BuilderError` variant carries a
//!   `String`, not a nested `SignerError`, so integration tests assert diagnostic text rather than type equality).
//!
//! ## Downstream implementors
//!
//! ```text
//! dig-coinstore  ──► implements CoinLookup  (returns chia-protocol::CoinState)
//! proposer       ──► implements BlockSigner (returns chia-bls::Signature)
//! tests/common   ──► MockCoinLookup, MockBlockSigner (STR-005)
//! ```

use chia_bls::Signature;
use chia_protocol::{Bytes32, CoinState};

/// Coin state lookup for Tier 3 (state) validation ([SPEC §7.2](docs/resources/SPEC.md), [STV-001](docs/requirements/domains/state_validation/specs/STV-001.md)).
///
/// Implementors provide access to the persistent coin set. The three methods supply the chain context
/// needed for:
/// - **Coin existence checks** (STV-002): `get_coin_state` returns the coin's lifecycle.
/// - **Height/time lock evaluation** (STV-005): `get_chain_height` / `get_chain_timestamp` provide
///   the reference clock for `ASSERT_HEIGHT_*` / `ASSERT_SECONDS_*` conditions.
///
/// ## Object safety
///
/// All methods take `&self` and return owned/copyable types — no `Self` in return position, no
/// generic parameters. `Box<dyn CoinLookup>` and `&dyn CoinLookup` are valid.
///
/// ## Chia parity
///
/// Method signatures mirror [`chia-blockchain/chia/consensus/block_body_validation.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py)
/// where `CoinStore.get_coin_record(coin_id)` returns `Optional[CoinRecord]` (Check 15).
pub trait CoinLookup {
    /// Look up a coin's current state by its ID.
    ///
    /// Returns `None` if the coin is unknown to this lookup source. Callers (STV-002)
    /// then check the ephemeral set (`ExecutionResult.additions`) before rejecting.
    fn get_coin_state(&self, coin_id: &Bytes32) -> Option<CoinState>;

    /// Current chain tip height as observed by this lookup source.
    ///
    /// Used by STV-005 for `ASSERT_HEIGHT_ABSOLUTE` / `BEFORE_HEIGHT_ABSOLUTE` evaluation.
    fn get_chain_height(&self) -> u64;

    /// Current chain tip timestamp (Unix seconds) as observed by this lookup source.
    ///
    /// Used by STV-005 for `ASSERT_SECONDS_ABSOLUTE` / `BEFORE_SECONDS_ABSOLUTE` evaluation.
    fn get_chain_timestamp(&self) -> u64;
}

/// Signing failure from a [`BlockSigner`] implementation.
///
/// Kept intentionally simple — the failure reason is an opaque string because signing backends
/// vary (HSM timeout, key-not-found, permission denied). The [`crate::BuilderError::SigningFailed`]
/// variant (ERR-004) wraps this for builder callers.
#[derive(Debug, thiserror::Error)]
pub enum SignerError {
    /// The signing operation failed; `0` carries a human-readable diagnostic.
    #[error("signing failed: {0}")]
    SigningFailed(String),
}

/// Block header signing hook for [`crate::builder::BlockBuilder::build`] ([SPEC §7.2](docs/resources/SPEC.md), BLD-006).
///
/// The proposer calls `sign_block(&header_hash)` at the end of the build pipeline to produce
/// the BLS signature stored in [`crate::L2Block::proposer_signature`].
///
/// ## Object safety
///
/// Same constraints as [`CoinLookup`] — `&self`, no generics, returns concrete types.
/// `Box<dyn BlockSigner>` is valid for dependency injection.
///
/// ## Chia parity
///
/// Signing uses `chia-bls::sign(secret_key, message)` internally. The message is the
/// header hash bytes (`Bytes32::as_ref()` → `&[u8; 32]`), matching the pattern in
/// [`chia-blockchain/chia/consensus/block_creation.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py)
/// where the farmer signs the header hash.
pub trait BlockSigner {
    /// Sign the given block header hash, producing a BLS12-381 signature.
    ///
    /// Returns [`SignerError`] if the key is unavailable or the signing backend fails.
    fn sign_block(&self, header_hash: &Bytes32) -> Result<Signature, SignerError>;
}
