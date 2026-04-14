# Receipt - Normative Requirements

| Field       | Value    |
|-------------|----------|
| Domain      | receipt  |
| Prefix      | RCP      |
| Total Items | 4        |
| Status      | Draft    |

## Requirements

### RCP-001: ReceiptStatus Enum

`ReceiptStatus` MUST define the following variants with numeric representation:

- `Success = 0`
- `InsufficientBalance = 1`
- `InvalidNonce = 2`
- `InvalidSignature = 3`
- `AccountNotFound = 4`
- `Failed = 255`

`ReceiptStatus` MUST derive a numeric `repr` for serialization.

**Spec Reference:** Section 2.9

### RCP-002: Receipt Struct

`Receipt` MUST contain the following fields:

- `tx_id`: `Bytes32`
- `block_height`: `u64`
- `tx_index`: `u32`
- `status`: `ReceiptStatus`
- `fee_charged`: `u64`
- `post_state_root`: `Bytes32`
- `cumulative_fees`: `u64`

**Spec Reference:** Section 2.9

### RCP-003: ReceiptList Methods

`ReceiptList` MUST contain the following fields:

- `receipts`: `Vec<Receipt>`
- `root`: `Bytes32`

`ReceiptList` MUST provide the following methods:

- `new()` — Creates an empty receipt list with `EMPTY_ROOT`.
- `from_receipts(Vec<Receipt>)` — Creates a list and computes the Merkle root.
- `push(Receipt)` — Appends a receipt to the list.
- `finalize()` — Recomputes the Merkle root.
- `get(index)` — Returns the receipt at the given index.
- `get_by_tx_id(Bytes32)` — Returns the receipt matching the given transaction ID.

**Spec Reference:** Section 2.9

### RCP-004: ReceiptList Aggregates

`ReceiptList` MUST provide the following aggregate methods:

- `len() -> usize` — Returns the number of receipts.
- `success_count() -> usize` — Returns the count of receipts with `ReceiptStatus::Success`.
- `failure_count() -> usize` — Returns the count of receipts with non-success status.
- `total_fees() -> u64` — Returns the sum of `fee_charged` across all receipts.

**Spec Reference:** Section 2.9
