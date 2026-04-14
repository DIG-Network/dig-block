# Error Types - Verification Matrix

> **Domain:** error_types
> **Prefix:** ERR
> **Normative:** [NORMATIVE.md](./NORMATIVE.md)
> **Tracking:** [TRACKING.yaml](./TRACKING.yaml)

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| ERR-001 | gap | BlockError Structural Variants | Unit test: construct each Tier 1 variant (InvalidData, InvalidVersion, TooLarge, CostExceeded, SpendBundleCountMismatch, InvalidSpendsRoot, InvalidReceiptsRoot, InvalidParent, InvalidSlashProposalsRoot, SlashProposalPayloadTooLarge, TooManySlashProposals, InvalidAdditionsRoot, InvalidRemovalsRoot, InvalidFilterHash, DuplicateOutput, DoubleSpendInBlock, AdditionsCountMismatch, RemovalsCountMismatch, SlashProposalCountMismatch, TimestampTooFarInFuture). Verify thiserror::Error derive produces Display and Error impls. |
| ERR-002 | gap | BlockError Execution and State Variants | Unit test: construct each Tier 2 variant (PuzzleHashMismatch, ClvmExecutionFailed, ClvmCostExceeded, AssertionFailed, AnnouncementNotFound, SignatureFailed, CoinMinting, FeesMismatch, ReserveFeeFailed, CostMismatch) and each Tier 3 variant (InvalidProposerSignature, NotFound, InvalidStateRoot, CoinNotFound, CoinAlreadySpent, CoinAlreadyExists). Verify all produce meaningful error messages. |
| ERR-003 | gap | CheckpointError Enum | Unit test: construct each variant (InvalidData, NotFound, Invalid, ScoreNotHigher, EpochMismatch, AlreadyFinalized, NotStarted). Verify thiserror::Error derive produces Display and Error impls. Verify structured fields on ScoreNotHigher and EpochMismatch. |
| ERR-004 | gap | BuilderError Enum | Unit test: construct each variant (CostBudgetExceeded, SizeBudgetExceeded, TooManySlashProposals, SlashProposalTooLarge, SigningFailed, EmptyBlock, MissingDfspRoots). Verify thiserror::Error derive produces Display and Error impls. Verify budget variants carry current/addition/max fields. |
| ERR-005 | gap | SignerBitmapError and ReceiptError Enums | Unit test: construct each SignerBitmapError variant (IndexOutOfBounds, TooManyValidators, InvalidLength, ValidatorCountMismatch) and each ReceiptError variant (InvalidData, NotFound). Verify both enums derive thiserror::Error with Display and Error impls. |
