# Crate Structure - Normative Requirements

> **Domain:** crate_structure
> **Prefix:** STR
> **Spec reference:** [SPEC.md - Sections 1.2, 7.2, 10, 11, 12](../../../resources/SPEC.md)

## Requirements

### STR-001: Cargo.toml Dependencies
Cargo.toml MUST include the following dependencies at the specified minimum versions: chia-protocol 0.26, chia-bls 0.26, dig-clvm 0.1, chia-consensus 0.26, chia-sdk-types 0.30, chia-sdk-signer 0.30, chia-sha2 0.26, chia-traits 0.26, clvm-utils 0.26, clvmr 0.14, bincode, serde (with derive feature), thiserror.
**Spec reference:** SPEC Section 1.2

### STR-002: Module Hierarchy
Module hierarchy MUST match SPEC Section 11: `types/` (header.rs, block.rs, attested.rs, checkpoint.rs, receipt.rs, signer_bitmap.rs, status.rs), `constants.rs`, `error.rs`, `hash.rs`, `validation/` (structural.rs, execution.rs, state.rs), `builder/` (block_builder.rs, checkpoint_builder.rs), `traits.rs` (CoinLookup, BlockSigner).
**Spec reference:** SPEC Section 11

### STR-003: Public Re-exports in lib.rs
lib.rs MUST re-export: all block types (L2BlockHeader, L2Block, AttestedBlock, Checkpoint, CheckpointSubmission), all status/supporting types (BlockStatus, CheckpointStatus, Receipt, ReceiptList, ReceiptStatus, SignerBitmap), all error types (BlockError, CheckpointError, BuilderError, SignerBitmapError, ReceiptError), all constants, traits (CoinLookup, BlockSigner), builder types (BlockBuilder, CheckpointBuilder), and ExecutionResult.
**Spec reference:** SPEC Section 10

### STR-004: CoinLookup and BlockSigner Trait Definitions
CoinLookup trait MUST define `get_coin_state(&Bytes32) -> Option<CoinState>`, `get_chain_height() -> u64`, and `get_chain_timestamp() -> u64`. The trait MUST use `chia-protocol::CoinState` directly. BlockSigner trait MUST define `sign_block(&Bytes32) -> Result<Signature, SignerError>`.
**Spec reference:** SPEC Section 7.2

### STR-005: Test Infrastructure
Test infrastructure MUST include a mock CoinLookup implementation, a mock BlockSigner implementation, and helper functions for creating test headers, test blocks, and test SpendBundles.
**Spec reference:** SPEC Section 12
