# Checkpoint - Normative Requirements

| Field       | Value        |
|-------------|--------------|
| Domain      | checkpoint   |
| Prefix      | CKP          |
| Total Items | 6            |
| Status      | Draft        |

## Requirements

### CKP-001: Checkpoint Struct

`Checkpoint` MUST contain the following fields:

- `epoch`: `u64`
- `state_root`: `Bytes32`
- `block_root`: `Bytes32`
- `block_count`: `u32`
- `tx_count`: `u64`
- `total_fees`: `u64`
- `prev_checkpoint`: `Bytes32`
- `withdrawals_root`: `Bytes32`
- `withdrawal_count`: `u32`

`Checkpoint` MUST provide a `new()` constructor.

**Spec Reference:** Section 2.6

### CKP-002: CheckpointSubmission Struct

`CheckpointSubmission` MUST contain the following fields:

- `checkpoint`: `Checkpoint`
- `signer_bitmap`: `SignerBitmap`
- `aggregate_signature`: `Signature`
- `aggregate_pubkey`: `PublicKey`
- `score`: `u64`
- `submitter`: `u32`
- `submission_height`: `Option<u32>`
- `submission_coin`: `Option<Bytes32>`

`new()` MUST create a submission with no L1 submission recorded (i.e., `submission_height` and `submission_coin` are `None`).

**Spec Reference:** Section 2.7

### CKP-003: CheckpointStatus Enum

`CheckpointStatus` MUST define the following variants:

- `Pending`
- `Collecting`
- `WinnerSelected { winner_hash: Bytes32, winner_score: u64 }`
- `Finalized { winner_hash: Bytes32, l1_height: u32 }`
- `Failed`

**Spec Reference:** Section 2.8

### CKP-004: Checkpoint Score Computation

`Checkpoint` MUST provide the method:

- `compute_score(stake_percentage: u64) -> u64` — Returns `stake_percentage * block_count`.

**Spec Reference:** Section 2.6

### CKP-005: CheckpointSubmission Methods

`CheckpointSubmission` MUST provide the following methods:

- `hash()` — Delegates to `checkpoint.hash()`.
- `epoch()` — Delegates to `checkpoint.epoch`.
- `signing_percentage() -> u64` — Returns the signing percentage from the signer bitmap.
- `meets_threshold(threshold_pct) -> bool` — Returns whether the signing percentage meets the threshold.
- `record_submission(height, coin_id)` — Records the L1 submission height and coin ID.
- `is_submitted() -> bool` — Returns `true` if `submission_height` is `Some`.

**Spec Reference:** Section 2.7

### CKP-006: CheckpointBuilder

`CheckpointBuilder` MUST provide the following methods:

- `new(epoch, prev_checkpoint)` — Creates a new builder for the given epoch.
- `add_block(block_hash, tx_count, fees)` — Adds a block to the checkpoint.
- `set_state_root(state_root)` — Sets the final state root.
- `add_withdrawal(withdrawal_hash)` — Adds a withdrawal to the checkpoint.
- `build() -> Checkpoint` — Builds the final checkpoint. MUST compute `block_root` and `withdrawals_root` as Merkle trees over the added blocks and withdrawals respectively.

**Spec Reference:** Section 6.6
