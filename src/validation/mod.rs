//! Three-tier block validation pipeline ([SPEC §5, §7](docs/resources/SPEC.md)).
//!
//! ## Tiers
//!
//! | Tier | Module | SPEC | Requirements | External state | Chia crates used |
//! |------|--------|------|-------------|----------------|-----------------|
//! | 1 — Structural | [`structural`] | [§5](docs/resources/SPEC.md) | SVL-001 — SVL-006 | None | `chia-consensus::compute_merkle_set_root`, `chia-sdk-types::MerkleTree` |
//! | 2 — Execution | [`execution`] | [§7.4](docs/resources/SPEC.md) | EXE-001 — EXE-009 | `clvmr::Allocator` | `dig-clvm::validate_spend_bundle` (wraps chia-consensus, chia-bls, clvmr) |
//! | 3 — State | [`state`] | [§7.5](docs/resources/SPEC.md) | STV-001 — STV-007 | [`crate::CoinLookup`] | `chia-bls::verify` |
//!
//! ## Validation flow
//!
//! ```text
//! L2Block
//!   │
//!   ├─► Tier 1: validate_structure()     ← no external state; cheapest checks first
//!   │     ├── header version vs height (SVL-001)
//!   │     ├── DFSP roots pre-activation (SVL-002)
//!   │     ├── cost/size limits (SVL-003)
//!   │     ├── timestamp future bound (SVL-004)
//!   │     ├── count agreement (SVL-005)
//!   │     └── Merkle roots + integrity (SVL-006)
//!   │
//!   ├─► Tier 2: validate_execution()     ← needs CLVM allocator
//!   │     ├── puzzle hash verification (EXE-002)
//!   │     ├── CLVM execution via dig-clvm (EXE-003)
//!   │     ├── condition parsing + assertion checking (EXE-004)
//!   │     ├── BLS signature verification (EXE-005)
//!   │     ├── conservation + fee consistency (EXE-006)
//!   │     ├── cost consistency (EXE-007)
//!   │     └── → ExecutionResult (EXE-008) with PendingAssertion (EXE-009)
//!   │
//!   └─► Tier 3: validate_state()         ← needs CoinLookup
//!         ├── coin existence (STV-002)
//!         ├── puzzle hash cross-check (STV-003)
//!         ├── addition non-existence (STV-004)
//!         ├── height/time lock evaluation (STV-005)
//!         ├── proposer signature (STV-006)
//!         └── state root verification (STV-007)
//! ```
//!
//! ## Composite method ([SPEC §10.3](docs/resources/SPEC.md))
//!
//! [`crate::L2Block::validate_full`] ([SPEC §7.1](docs/resources/SPEC.md), STV-001) chains all three
//! tiers. If Tier 1 fails, Tiers 2 and 3 are never reached. If Tier 2 fails, Tier 3 is never reached.
//! Returns the first error encountered or `Ok(computed_state_root)` on success.
//!
//! ## Chia parity ([SPEC §1.4](docs/resources/SPEC.md))
//!
//! The three-tier split mirrors Chia's validation in
//! [`block_body_validation.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py):
//! - Checks 1-14 ≈ Tier 1 (structural: counts, roots, duplicates, size — [SPEC §5.1–§5.2](docs/resources/SPEC.md))
//! - Checks 15-22 ≈ Tier 2+3 (CLVM execution, coin existence, signatures, conservation — [SPEC §7.4–§7.5](docs/resources/SPEC.md))
//!
//! DIG separates execution (CLVM) from state (coin lookups) for cleaner testing and partial
//! validation ([SPEC §1.1 Design Principle: Layered validation](docs/resources/SPEC.md)).

pub mod execution;
pub mod state;
pub mod structural;
