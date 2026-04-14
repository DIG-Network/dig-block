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

/// CLVM execution cost unit (alias of `u64`).
///
/// Used for per-bundle and per-block budgets ([`crate::constants::MAX_COST_PER_BLOCK`], execution
/// validation EXE-* requirements).
pub type Cost = u64;

/// Protocol version for blocks strictly before DFSP activation height.
///
/// **Semantics:** See BLK-007 — when `height < DFSP_ACTIVATION_HEIGHT`, header version must be this
/// value (and when [`crate::constants::DFSP_ACTIVATION_HEIGHT`] is `u64::MAX`, every practical height
/// uses V1).
pub const VERSION_V1: u16 = 1;

/// Protocol version for blocks at or after DFSP activation height.
///
/// **Semantics:** BLK-007 selects this when `height >= DFSP_ACTIVATION_HEIGHT` and activation is not
/// permanently disabled.
pub const VERSION_V2: u16 = 2;
