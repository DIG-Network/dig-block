# Error Types - Verification Matrix

> **Domain:** error_types
> **Prefix:** ERR
> **Normative:** [NORMATIVE.md](./NORMATIVE.md)
> **Tracking:** [TRACKING.yaml](./TRACKING.yaml)

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| ERR-001 | implemented | BlockError Structural Variants | `tests/test_block_error_structural.rs`: construct Tier 1 variants; assert Display embeds spec payloads; `InvalidVersion { expected, actual }` (SVL-001 diagnostics); `std::error::Error` + `Clone`. |
| ERR-002 | implemented | BlockError Execution and State Variants | `tests/test_block_error_execution_state.rs`: Tier 2 + Tier 3 variants; Display payloads; `Clone` + `std::error::Error`. |
| ERR-003 | implemented | CheckpointError Enum | `tests/test_checkpoint_error.rs`: all variants; Display payloads; `Clone` + `std::error::Error`. |
| ERR-004 | implemented | BuilderError Enum | `tests/test_builder_error.rs`: all seven variants; Display matches ERR-004 `#[error]` templates; `Clone` + `std::error::Error` (`.source()`); budget variants expose `current` / `addition` / `max`. |
| ERR-005 | implemented | SignerBitmapError and ReceiptError Enums | `tests/test_signer_bitmap_error.rs`: four SignerBitmap variants; Display; `Clone` + `std::error::Error`. `tests/test_receipt_error.rs`: InvalidData + NotFound(Bytes32); Display; `Clone` + `std::error::Error`. `SignerBitmap::set_signed` / `merge` updated to populate structured fields (ATT-004/ATT-005). |
