//! Fundamental type aliases, protocol version tags, and Chia type re-exports (BLK-006).
//!
//! **Requirement:** [BLK-006](docs/requirements/domains/block_types/specs/BLK-006.md) /
//! [NORMATIVE § BLK-006](docs/requirements/domains/block_types/NORMATIVE.md#blk-006-primitive-types) /
//! [SPEC §2.1](docs/resources/SPEC.md).
//!
//! ## Usage
//!
//! Downstream crates should import [`Cost`], [`VERSION_V1`], [`VERSION_V2`], [`Bytes32`], [`Signature`],
//! and [`PublicKey`] from **`dig_block`** so DIG L2 code shares one type identity with validation and
//! builders in this crate (see [`crate::constants`] for limits that use [`Cost`]).
//!
//! ## Rationale
//!
//! - **`Cost`** documents intent: a `u64` in header or bundle context is CLVM cost, not an arbitrary
//!   scalar (fees, heights, timestamps remain plain integers per domain).
//! - **Re-exports** keep dependency graphs shallow: consumers need not add direct `chia-protocol` /
//!   `chia-bls` edges only to name hashes or BLS material aligned with blocks produced here.
//!
//! ## Decisions
//!
//! - Version constants are `u16` to match header wire format (BLK-007 auto-detection). Values `1` and `2`
//!   are stable protocol identifiers; changing them is a breaking network upgrade.
//! - This module is declared **before** [`crate::constants`] in [`crate`] so limit constants can use
//!   [`Cost`] and [`Bytes32`] without circular imports.

pub use chia_bls::{PublicKey, Signature};
pub use chia_protocol::Bytes32;

/// CLVM execution cost unit (alias of `u64`) ([SPEC §2.1](docs/resources/SPEC.md)).
///
/// Used for per-bundle and per-block budgets ([`crate::constants::MAX_COST_PER_BLOCK`] — [SPEC §2.11](docs/resources/SPEC.md),
/// execution validation EXE-* — [SPEC §7.4.6](docs/resources/SPEC.md)).
pub type Cost = u64;

/// Protocol version for blocks strictly before DFSP activation height ([SPEC §2.2](docs/resources/SPEC.md) version semantics).
///
/// **Semantics:** When `height < DFSP_ACTIVATION_HEIGHT`, header version must be this value. When
/// [`DFSP_ACTIVATION_HEIGHT`](crate::constants::DFSP_ACTIVATION_HEIGHT) is `u64::MAX` (DFSP disabled),
/// every practical height uses V1. See [SPEC §1.3 Decision 10](docs/resources/SPEC.md) (version auto-detection).
pub const VERSION_V1: u16 = 1;

/// Protocol version for blocks at or after DFSP activation height ([SPEC §2.2](docs/resources/SPEC.md) version semantics).
///
/// **Semantics:** Selected when `height >= DFSP_ACTIVATION_HEIGHT` and activation is not permanently
/// disabled. V2 blocks MUST carry correct DFSP SMT roots ([SPEC §1.3 Decision 7](docs/resources/SPEC.md)).
pub const VERSION_V2: u16 = 2;
