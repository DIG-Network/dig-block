# Error Types - Verification Matrix

> **Domain:** error_types
> **Prefix:** ERR
> **Normative:** [NORMATIVE.md](./NORMATIVE.md)
> **Tracking:** [TRACKING.yaml](./TRACKING.yaml)

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| ERR-001 | implemented | BlockError Structural Variants | `tests/test_block_error_structural.rs`: construct Tier 1 variants; assert Display embeds spec payloads; `std::error::Error` + `Clone`. |
| ERR-002 | implemented | BlockError Execution and State Variants | `tests/test_block_error_execution_state.rs`: Tier 2 + Tier 3 variants; Display payloads; `Clone` + `std::error::Error`. |
| ERR-003 | implemented | CheckpointError Enum | `tests/test_checkpoint_error.rs`: all variants; Display payloads; `Clone` + `std::error::Error`. |
| ERR-004 | gap | BuilderError Enum | Unit test: construct each variant (CostBudgetExceeded, SizeBudgetExceeded, TooManySlashProposals, SlashProposalTooLarge, SigningFailed, EmptyBlock, MissingDfspRoots). Verify thiserror::Error derive produces Display and Error impls. Verify budget variants carry current/addition/max fields. |
| ERR-005 | gap | SignerBitmapError and ReceiptError Enums | Unit test: construct each SignerBitmapError variant (IndexOutOfBounds, TooManyValidators, InvalidLength, ValidatorCountMismatch) and each ReceiptError variant (InvalidData, NotFound). Verify both enums derive thiserror::Error with Display and Error impls. |
