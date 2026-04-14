# Attestation - Normative Requirements

| Field       | Value        |
|-------------|--------------|
| Domain      | attestation  |
| Prefix      | ATT          |
| Total Items | 5            |
| Status      | Draft        |

## Requirements

### ATT-001: AttestedBlock Struct and Constructor

`AttestedBlock` MUST contain the following fields:

- `block`: `L2Block`
- `signer_bitmap`: `SignerBitmap`
- `aggregate_signature`: `Signature`
- `receipts`: `ReceiptList`
- `status`: `BlockStatus`

`new(block, validator_count, receipts)` MUST create an `AttestedBlock` with an empty signer bitmap and `Pending` status. The initial `aggregate_signature` MUST be set to the proposer's signature.

**Spec Reference:** Section 2.4

### ATT-002: AttestedBlock Methods

`AttestedBlock` MUST provide the following methods:

- `signing_percentage() -> u64` — Returns the signing percentage as an integer in the range 0-100.
- `has_soft_finality(threshold_pct) -> bool` — Returns `true` if the signing percentage meets or exceeds `threshold_pct`.
- `hash()` — MUST delegate to `block.hash()`.

**Spec Reference:** Section 2.4

### ATT-003: BlockStatus Enum

`BlockStatus` MUST define the following variants:

- `Pending`
- `Validated`
- `SoftFinalized`
- `HardFinalized`
- `Orphaned`
- `Rejected`

`BlockStatus` MUST provide the following methods:

- `is_finalized() -> bool` — MUST return `true` for `SoftFinalized` and `HardFinalized`; `false` otherwise.
- `is_canonical() -> bool` — MUST return `false` for `Orphaned` and `Rejected`; `true` otherwise.

**Spec Reference:** Section 2.5

### ATT-004: SignerBitmap Core Methods

`SignerBitmap` MUST contain the following fields:

- `bits`: `Vec<u8>`
- `validator_count`: `u32`

`MAX_VALIDATORS` MUST be `65536`.

`SignerBitmap` MUST provide the following methods:

- `new(validator_count)` — Creates a new empty bitmap for the given validator count.
- `from_bytes(bytes, validator_count)` — Creates a bitmap from raw bytes.
- `has_signed(index) -> bool` — Returns whether the validator at `index` has signed.
- `set_signed(index) -> Result` — Marks the validator at `index` as signed.
- `signer_count() -> u32` — Returns the number of validators that have signed.
- `signing_percentage() -> u64` — Returns the signing percentage (0-100).
- `has_threshold(threshold_pct) -> bool` — Returns whether the signing percentage meets the threshold.
- `as_bytes() -> &[u8]` — Returns the raw bitmap bytes.

**Spec Reference:** Section 2.10

### ATT-005: SignerBitmap Merge and Indices

`SignerBitmap` MUST provide the following methods:

- `merge(other)` — Performs a bitwise OR of two bitmaps. MUST reject mismatched `validator_count`.
- `signer_indices() -> Vec<u32>` — Returns an ordered list of indices for validators that have signed.

**Spec Reference:** Section 2.10
