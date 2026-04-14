//! Compact validator participation bitmap ([SPEC §2.10](docs/resources/SPEC.md)).
//!
//! ## Requirements trace
//!
//! - **[ATT-004](docs/requirements/domains/attestation/specs/ATT-004.md)** — struct shape, `MAX_VALIDATORS`,
//!   `new` / `from_bytes` / bit accessors / counts / thresholds / `as_bytes`.
//! - **[ATT-005](docs/requirements/domains/attestation/specs/ATT-005.md)** — [`Self::merge`], [`Self::signer_indices`]
//!   (bitwise OR aggregation + sorted index list).
//! - **[NORMATIVE](docs/requirements/domains/attestation/NORMATIVE.md)** — ATT-004 / ATT-005 API obligations.
//!
//! ## Encoding
//!
//! - **Byte length:** `ceil(validator_count / 8)` — see [`Self::new`].
//! - **Bit order:** LSB-first within each byte (validator `i` → byte `i/8`, bit `i % 8`). This matches the
//!   pseudocode in ATT-004 and keeps popcount-based [`Self::signer_count`] aligned with the spec.
//!
//! ## Usage
//!
//! Construct with [`Self::new`], mark signers with [`Self::set_signed`], query with [`Self::has_signed`],
//! [`Self::signer_count`], [`Self::signing_percentage`], and [`Self::has_threshold`]. Combine peer views with
//! [`Self::merge`] and enumerate participants in order via [`Self::signer_indices`]. Raw wire bytes are exposed
//! through [`Self::as_bytes`] / [`Self::from_bytes`] for bincode payloads ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)).
//!
//! ## Safety / limits
//!
//! [`Self::new`] and [`Self::from_bytes`] **assert** `validator_count <= MAX_VALIDATORS` so a single `u32`
//! cannot force multi-gigabyte allocations in this crate; the protocol cap is **65536** validators.

use crate::SignerBitmapError;
use serde::{Deserialize, Serialize};

/// Maximum number of validators representable in protocol bitmaps (ATT-004 / NORMATIVE).
pub const MAX_VALIDATORS: u32 = 65_536;

/// Bit vector of “which validators signed,” sized for a fixed validator set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignerBitmap {
    /// Raw little-endian bit-packed bytes (see module docs).
    bits: Vec<u8>,
    /// Logical validator cardinality; indices are `0 .. validator_count`.
    validator_count: u32,
}

impl SignerBitmap {
    /// Empty bitmap: all bits zero, sized for `validator_count` validators.
    ///
    /// **Panics** if `validator_count > MAX_VALIDATORS` (see [`MAX_VALIDATORS`]).
    #[must_use]
    pub fn new(validator_count: u32) -> Self {
        assert!(
            validator_count <= MAX_VALIDATORS,
            "SignerBitmap::new: validator_count {validator_count} exceeds MAX_VALIDATORS ({MAX_VALIDATORS})"
        );
        let byte_count = (validator_count as usize).div_ceil(8);
        Self {
            bits: vec![0u8; byte_count],
            validator_count,
        }
    }

    /// Wrap existing bytes (e.g. after deserialization) with a validator count.
    ///
    /// Does **not** copy-truncate `bytes` to the canonical length; callers should supply `ceil(n/8)` bytes
    /// consistent with `validator_count`. [`Self::as_bytes`] on a value from [`Self::new`] always matches.
    ///
    /// **Panics** if `validator_count > MAX_VALIDATORS`.
    #[must_use]
    pub fn from_bytes(bytes: &[u8], validator_count: u32) -> Self {
        assert!(
            validator_count <= MAX_VALIDATORS,
            "SignerBitmap::from_bytes: validator_count {validator_count} exceeds MAX_VALIDATORS ({MAX_VALIDATORS})"
        );
        Self {
            bits: bytes.to_vec(),
            validator_count,
        }
    }

    /// `true` if validator `index` has a raised bit and `index < validator_count`.
    ///
    /// Out-of-range indices return `false` (no panic); short [`Self::bits`] tails read as zero.
    #[must_use]
    pub fn has_signed(&self, index: u32) -> bool {
        if index >= self.validator_count {
            return false;
        }
        let byte_index = (index / 8) as usize;
        let bit_index = index % 8;
        let Some(&byte) = self.bits.get(byte_index) else {
            return false;
        };
        byte & (1 << bit_index) != 0
    }

    /// Sets the bit for `index`. **Error** if `index >= validator_count` ([`SignerBitmapError::IndexOutOfBounds`]).
    pub fn set_signed(&mut self, index: u32) -> Result<(), SignerBitmapError> {
        if index >= self.validator_count {
            return Err(SignerBitmapError::IndexOutOfBounds {
                index,
                max: self.validator_count,
            });
        }
        let byte_index = (index / 8) as usize;
        let bit_index = index % 8;
        let canonical_len = (self.validator_count as usize).div_ceil(8);
        if self.bits.len() < canonical_len {
            self.bits.resize(canonical_len, 0);
        }
        self.bits[byte_index] |= 1 << bit_index;
        Ok(())
    }

    /// Popcount over **all** stored bytes (ATT-004 spec algorithm).
    ///
    /// For bitmaps produced only via [`Self::new`] + [`Self::set_signed`], unused high bits in the last
    /// byte stay zero, so the count matches “number of validators signed.”
    #[must_use]
    pub fn signer_count(&self) -> u32 {
        self.bits.iter().map(|b| b.count_ones()).sum()
    }

    /// Integer percentage `0..=100`: `(signer_count * 100) / validator_count`, or `0` if `validator_count == 0`.
    #[must_use]
    pub fn signing_percentage(&self) -> u64 {
        if self.validator_count == 0 {
            return 0;
        }
        (u64::from(self.signer_count()) * 100) / u64::from(self.validator_count)
    }

    /// `true` iff [`Self::signing_percentage`] `>= threshold_pct`.
    #[must_use]
    pub fn has_threshold(&self, threshold_pct: u64) -> bool {
        self.signing_percentage() >= threshold_pct
    }

    /// Borrow raw bitmap bytes (serialization / hashing helpers).
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bits
    }

    /// Validator cardinality configured for this bitmap (NORMATIVE field).
    #[must_use]
    pub fn validator_count(&self) -> u32 {
        self.validator_count
    }

    /// Bitwise OR of `other` into `self` (union of signer sets).
    ///
    /// **ATT-005:** [`Self::validator_count`] MUST match on both operands; otherwise [`SignerBitmapError::ValidatorCountMismatch`].
    ///
    /// ## Rationale (vs. spec pseudocode)
    ///
    /// The ATT-005 snippet zips byte vectors; if `self.bits` is shorter than `other.bits` (e.g. odd
    /// [`Self::from_bytes`] layouts), zip would **drop** trailing OR contributions. This implementation resizes
    /// `self.bits` to the canonical `ceil(validator_count / 8)` length, then ORs each index `i` with
    /// `other.bits.get(i).unwrap_or(0)`, matching “combine all signers” intent while staying commutative on
    /// well-formed bitmaps.
    pub fn merge(&mut self, other: &SignerBitmap) -> Result<(), SignerBitmapError> {
        if self.validator_count != other.validator_count {
            return Err(SignerBitmapError::ValidatorCountMismatch {
                expected: self.validator_count,
                got: other.validator_count,
            });
        }
        let n = (self.validator_count as usize).div_ceil(8);
        self.bits.resize(n, 0);
        for i in 0..n {
            let ob = other.bits.get(i).copied().unwrap_or(0);
            self.bits[i] |= ob;
        }
        Ok(())
    }

    /// All validator indices with a set bit, in **ascending** order (`0 .. validator_count` scan).
    ///
    /// **ATT-005:** Order follows the spec loop (`i` increasing); callers rely on this for deterministic
    /// serialization, tests, and UX (e.g. display). Empty bitmap → empty [`Vec`].
    #[must_use]
    pub fn signer_indices(&self) -> Vec<u32> {
        (0..self.validator_count)
            .filter(|&i| self.has_signed(i))
            .collect()
    }
}
