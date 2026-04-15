# Error Types - Normative Requirements

> **Domain:** error_types
> **Prefix:** ERR
> **Spec reference:** [SPEC.md - Sections 4.1-4.4, 6.5, 7.6](../../../resources/SPEC.md)

## Requirements

### ERR-001: BlockError Structural Variants

BlockError MUST define variants for structural validation (Tier 1):

- `InvalidData(String)` - generic invalid data
- `InvalidVersion { expected: u16, actual: u16 }` - header version does not match height / DFSP activation (SVL-001)
- `TooLarge { size, max }` - block exceeds maximum size
- `CostExceeded { cost, max }` - block exceeds cost budget
- `SpendBundleCountMismatch { header, actual }` - header count does not match actual bundle count
- `InvalidSpendsRoot { expected, computed }` - spends root does not match recomputed value
- `InvalidReceiptsRoot { expected, computed }` - receipts root does not match recomputed value
- `InvalidParent { expected, got }` - parent hash mismatch
- `InvalidSlashProposalsRoot` - slash proposals root mismatch
- `SlashProposalPayloadTooLarge` - single slash proposal exceeds size limit
- `TooManySlashProposals` - slash proposal count exceeds maximum
- `InvalidAdditionsRoot` - additions root mismatch
- `InvalidRemovalsRoot` - removals root mismatch
- `InvalidFilterHash` - filter hash mismatch
- `DuplicateOutput { coin_id }` - duplicate output coin detected
- `DoubleSpendInBlock { coin_id }` - same coin spent twice in one block
- `AdditionsCountMismatch` - header additions count does not match actual
- `RemovalsCountMismatch` - header removals count does not match actual
- `SlashProposalCountMismatch` - header slash proposal count does not match actual
- `TimestampTooFarInFuture { timestamp, max_allowed }` - block timestamp exceeds allowed future offset

BlockError MUST derive thiserror::Error.

**Spec reference:** SPEC Section 4.1

### ERR-002: BlockError Execution and State Variants

BlockError MUST also define Tier 2 (execution) variants:

- `PuzzleHashMismatch { coin_id, expected, computed }` - puzzle hash does not match
- `ClvmExecutionFailed { coin_id, reason }` - CLVM execution error
- `ClvmCostExceeded { coin_id, cost, remaining }` - individual spend exceeds remaining cost budget
- `AssertionFailed { condition, reason }` - CLVM assertion condition failed
- `AnnouncementNotFound { announcement_hash }` - expected announcement not found
- `SignatureFailed { bundle_index }` - aggregate signature verification failed
- `CoinMinting { removed, added }` - additions exceed removals (coin creation without authority)
- `FeesMismatch { header, computed }` - header fees do not match computed fees
- `ReserveFeeFailed { required, actual }` - spend bundle does not meet reserve fee
- `CostMismatch { header, computed }` - header cost does not match computed cost

And Tier 3 (state) variants:

- `InvalidProposerSignature` - proposer signature verification failed
- `NotFound(Bytes32)` - referenced block not found
- `InvalidStateRoot { expected, computed }` - state root mismatch after applying block
- `CoinNotFound { coin_id }` - spent coin does not exist in state
- `CoinAlreadySpent { coin_id, spent_height }` - coin was already spent at given height
- `CoinAlreadyExists { coin_id }` - output coin already exists in state

**Spec reference:** SPEC Section 4.1, 7.6

### ERR-003: CheckpointError Enum

CheckpointError MUST define the following variants:

- `InvalidData(String)` - generic invalid checkpoint data
- `NotFound(u64)` - checkpoint at given epoch not found
- `Invalid(String)` - checkpoint validation failed
- `ScoreNotHigher { current, submitted }` - submitted checkpoint score does not exceed current
- `EpochMismatch { expected, got }` - checkpoint epoch does not match expected
- `AlreadyFinalized` - checkpoint has already been finalized
- `NotStarted` - checkpoint process has not started

CheckpointError MUST derive thiserror::Error.

**Spec reference:** SPEC Section 4.2

### ERR-004: BuilderError Enum

BuilderError MUST define the following variants:

- `CostBudgetExceeded { current, addition, max }` - adding spend would exceed cost budget
- `SizeBudgetExceeded { current, addition, max }` - adding spend would exceed size budget
- `TooManySlashProposals { max }` - slash proposal count would exceed maximum
- `SlashProposalTooLarge { size, max }` - slash proposal payload exceeds size limit
- `SigningFailed(String)` - block signing operation failed
- `EmptyBlock` - block builder has no spend bundles
- `MissingDfspRoots` - DFSP roots required but not provided

BuilderError MUST derive thiserror::Error.

**Spec reference:** SPEC Section 6.5

### ERR-005: SignerBitmapError and ReceiptError Enums

SignerBitmapError MUST define the following variants:

- `IndexOutOfBounds { index, max }` - signer index exceeds bitmap capacity
- `TooManyValidators(usize)` - validator count exceeds maximum supported
- `InvalidLength { expected, got }` - bitmap byte length does not match expected
- `ValidatorCountMismatch { expected, got }` - validator count does not match expected

ReceiptError MUST define the following variants:

- `InvalidData(String)` - generic invalid receipt data
- `NotFound(Bytes32)` - receipt with given hash not found

Both SignerBitmapError and ReceiptError MUST derive thiserror::Error.

**Spec reference:** SPEC Section 4.3, 4.4
