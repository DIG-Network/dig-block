//! Tier 3 **state validation** (STV-*): coin existence, puzzle hash cross-checks, lock evaluation,
//! proposer signature, and state root verification.
//!
//! ## Requirements trace
//!
//! - **[STV-001](docs/requirements/domains/state_validation/specs/STV-001.md)** — `validate_state()` API + `validate_full()` composite.
//! - **[STV-002](docs/requirements/domains/state_validation/specs/STV-002.md)** — coin existence checks: removals must exist and be unspent, or be ephemeral (created in same block). Also handles `ASSERT_EPHEMERAL` condition.
//! - **[STV-003](docs/requirements/domains/state_validation/specs/STV-003.md)** — puzzle hash cross-check: `CoinState.coin.puzzle_hash == CoinSpend.coin.puzzle_hash`.
//! - **[STV-004](docs/requirements/domains/state_validation/specs/STV-004.md)** — addition non-existence: created coins must not already exist in coin set (exception: ephemeral spent in same block).
//! - **[STV-005](docs/requirements/domains/state_validation/specs/STV-005.md)** — height/time lock evaluation: 8 assertion types from `PendingAssertion` (EXE-009) evaluated against chain context from [`crate::CoinLookup`].
//! - **[STV-006](docs/requirements/domains/state_validation/specs/STV-006.md)** — proposer signature: `chia-bls::verify(pubkey, header.hash(), block.proposer_signature)`.
//! - **[STV-007](docs/requirements/domains/state_validation/specs/STV-007.md)** — state root verification: apply additions/removals, recompute Merkle root, compare to `header.state_root`.
//! - **[NORMATIVE](docs/requirements/domains/state_validation/NORMATIVE.md)** — full state validation domain.
//! - **[SPEC §7.5](docs/resources/SPEC.md)** — Tier 3 state validation pipeline.
//!
//! ## API signatures (STV-001)
//!
//! ```text
//! L2Block::validate_state(&self, exec: &ExecutionResult, coins: &dyn CoinLookup, proposer_pubkey: &PublicKey)
//!     -> Result<Bytes32, BlockError>
//!     // Returns computed state_root on success
//!
//! L2Block::validate_full(&self, config: &ValidationConfig, genesis: &Bytes32, coins: &dyn CoinLookup, pubkey: &PublicKey)
//!     -> Result<Bytes32, BlockError>
//!     // Chains: validate_structure() → validate_execution() → validate_state()
//!     // Returns first error or Ok(state_root)
//! ```
//!
//! ## Height/time lock assertion types (STV-005)
//!
//! | Assertion | Evaluation |
//! |-----------|-----------|
//! | `ASSERT_HEIGHT_ABSOLUTE(h)` | `chain_height >= h` |
//! | `ASSERT_HEIGHT_RELATIVE(h)` | `chain_height >= coin_confirmed_height + h` |
//! | `ASSERT_SECONDS_ABSOLUTE(t)` | `chain_timestamp >= t` |
//! | `ASSERT_SECONDS_RELATIVE(t)` | `chain_timestamp >= coin_timestamp + t` |
//! | `BEFORE_HEIGHT_ABSOLUTE(h)` | `chain_height < h` |
//! | `BEFORE_HEIGHT_RELATIVE(h)` | `chain_height < coin_confirmed_height + h` |
//! | `BEFORE_SECONDS_ABSOLUTE(t)` | `chain_timestamp < t` |
//! | `BEFORE_SECONDS_RELATIVE(t)` | `chain_timestamp < coin_timestamp + t` |
//!
//! Relative assertions require looking up the coin's `created_height` / timestamp from [`crate::CoinLookup`].
//! For ephemeral coins (created in the same block), `created_height` is the current block height.
//!
//! ## Chia parity
//!
//! - Coin existence: [`block_body_validation.py` Check 15](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py) (`UNKNOWN_UNSPENT`, `DOUBLE_SPEND`).
//! - Height/time locks: [`block_body_validation.py` Check 21](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py).
//! - Proposer signature: Analogous to Chia's farmer signature verification in [`block_header_validation.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py).
//!
//! ## Status
//!
//! Stub — full implementation in STV-001 through STV-007.
