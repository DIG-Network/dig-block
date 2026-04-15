# Implementation Order

Phased checklist for dig-block requirements. Work top-to-bottom within each phase.
After completing a requirement: write tests, verify they pass, update TRACKING.yaml, VERIFICATION.md, and check off here.

**A requirement is NOT complete until comprehensive tests verify it.**

---

## Phase 0: Crate Structure & Foundation

- [x] STR-001 — Cargo.toml with chia/DIG crate dependencies and metadata
- [x] STR-002 — Module hierarchy (src/lib.rs root, submodule layout matching SPEC Section 11)
- [x] STR-003 — Public re-exports (chia-protocol, chia-bls, dig-clvm types)
- [x] STR-004 — CoinLookup and BlockSigner trait definitions
- [x] STR-005 — Test infrastructure (test helpers, mock CoinLookup, mock BlockSigner)

## Phase 1: Constants & Primitive Types

- [x] BLK-005 — Protocol constants (EMPTY_ROOT, ZERO_HASH, MAX_BLOCK_SIZE, MAX_COST_PER_BLOCK, etc.)
- [x] BLK-006 — Primitive types (Cost alias, version constants, DFSP_ACTIVATION_HEIGHT)

## Phase 2: Core Block Types

- [x] BLK-001 — L2BlockHeader struct with all field groups
- [x] BLK-002 — L2BlockHeader constructors (new, new_with_collateral, new_with_l1_proofs, genesis)
- [x] BLK-007 — Version auto-detection from height and DFSP_ACTIVATION_HEIGHT
- [x] BLK-003 — L2Block struct with header, spend_bundles, slash_proposal_payloads, proposer_signature
- [x] BLK-004 — L2Block helper methods (all_additions, all_removals, has_duplicate_outputs, has_double_spends, compute_size)

## Phase 3: Supporting Types

- [x] ATT-003 — BlockStatus enum (Pending, Validated, SoftFinalized, HardFinalized, Orphaned, Rejected)
- [x] ATT-004 — SignerBitmap struct and core methods (new, has_signed, set_signed, signer_count, signing_percentage)
- [x] ATT-005 — SignerBitmap merge and signer_indices
- [x] ATT-001 — AttestedBlock struct and constructor
- [x] ATT-002 — AttestedBlock methods (signing_percentage, has_soft_finality)
- [x] RCP-001 — ReceiptStatus enum
- [x] RCP-002 — Receipt struct
- [x] RCP-003 — ReceiptList struct and methods (new, from_receipts, push, finalize, get, get_by_tx_id)
- [x] RCP-004 — ReceiptList aggregate methods (len, success_count, failure_count, total_fees)
- [x] CKP-003 — CheckpointStatus enum
- [x] CKP-001 — Checkpoint struct and constructor
- [x] CKP-002 — CheckpointSubmission struct and constructor
- [x] CKP-004 — Checkpoint score computation
- [x] CKP-005 — CheckpointSubmission methods (signing_percentage, meets_threshold, record_submission, is_submitted)

## Phase 4: Error Types

- [x] ERR-001 — BlockError enum (structural validation variants)
- [x] ERR-002 — BlockError execution and state validation variants (Tier 2 and 3)
- [x] ERR-003 — CheckpointError enum
- [x] ERR-004 — BuilderError enum
- [x] ERR-005 — SignerBitmapError and ReceiptError enums

## Phase 5: Hashing

- [x] HSH-007 — Tagged Merkle hashing (0x01 leaf prefix, 0x02 node prefix domain separation)
- [x] HSH-001 — Block header hash (SHA-256, fixed-order SPEC §3.1 preimage; 710 bytes — see `L2BlockHeader::HASH_PREIMAGE_LEN`)
- [x] HSH-002 — Checkpoint hash (SHA-256, fixed-order, 160 bytes)
- [x] HSH-003 — Spends root computation (MerkleTree of SpendBundle hashes)
- [x] HSH-004 — Additions root construction (chia-consensus compute_merkle_set_root, grouped by puzzle_hash)
- [x] HSH-005 — Removals root construction (chia-consensus compute_merkle_set_root of coin IDs)
- [x] HSH-006 — Filter hash construction (BIP158 compact filter)
- [x] HSH-008 — Receipts root computation (MerkleTree of receipt hashes, used by ReceiptList)

## Phase 6: Structural Validation (Tier 1)

- [x] SVL-001 — Header validation: version check
- [x] SVL-002 — Header validation: DFSP root pre-activation check
- [x] SVL-003 — Header validation: cost and size limit checks
- [x] SVL-004 — Header validation: timestamp future bound (MAX_FUTURE_TIMESTAMP_SECONDS)
- [x] SVL-005 — Block structural validation: count agreement (spend_bundle_count, additions_count, removals_count, slash_proposal_count)
- [x] SVL-006 — Block structural validation: Merkle root checks and duplicate/double-spend detection

## Phase 7: Block Production

- [x] BLD-001 — BlockBuilder struct and new() constructor
- [x] BLD-002 — add_spend_bundle() with cost and size budget enforcement
- [x] BLD-003 — add_slash_proposal() with count and size limits
- [x] BLD-004 — set_l1_proofs(), set_dfsp_roots(), set_extension_data()
- [x] BLD-005 — build() pipeline (compute all derived fields, assemble header)
- [x] BLD-006 — BlockSigner trait and signing in build()
- [ ] BLD-007 — Builder produces structurally valid blocks by construction
- [x] CKP-006 — CheckpointBuilder (new, add_block, set_state_root, add_withdrawal, build)

## Phase 8: Serialization

- [ ] SER-001 — Bincode serialization for all block types
- [ ] SER-002 — to_bytes() infallible, from_bytes() fallible with error mapping
- [ ] SER-003 — Genesis block construction via L2BlockHeader::genesis()
- [ ] SER-004 — Serde default attributes for backwards compatibility
- [ ] SER-005 — Serialization round-trip integrity for all types

## Phase 9: Execution Validation (Tier 2)

- [ ] EXE-001 — validate_execution() API and ValidationConfig integration
- [ ] EXE-002 — Puzzle hash verification (tree_hash(puzzle_reveal) == coin.puzzle_hash)
- [ ] EXE-003 — CLVM execution via dig_clvm::validate_spend_bundle()
- [ ] EXE-004 — Condition parsing and assertion checking (announcements, concurrent spend, self-assertions)
- [ ] EXE-005 — BLS aggregate signature verification (via dig-clvm)
- [ ] EXE-006 — Coin conservation and fee consistency verification
- [ ] EXE-007 — Cost consistency verification (computed vs header)
- [ ] EXE-008 — ExecutionResult output type
- [ ] EXE-009 — PendingAssertion type definition (AssertionKind enum, from_condition factory)

## Phase 10: State Validation (Tier 3)

- [ ] STV-001 — validate_state() API and CoinLookup trait integration
- [ ] STV-002 — Coin existence checks (removals must exist and be unspent, or be ephemeral)
- [ ] STV-003 — Puzzle hash cross-check from coin state
- [ ] STV-004 — Addition non-existence check
- [ ] STV-005 — Height/time lock evaluation (8 assertion types)
- [ ] STV-006 — Proposer signature verification
- [ ] STV-007 — State root verification (apply additions/removals, compare)

---

## Summary

| Phase | Domain(s) | Requirements |
|-------|-----------|-------------|
| 0 | Crate Structure | STR-001 — STR-005 (5) |
| 1 | Block Types (constants) | BLK-005 — BLK-006 (2) |
| 2 | Block Types (core) | BLK-001 — BLK-004, BLK-007 (5) |
| 3 | Attestation, Receipt, Checkpoint | ATT-001 — ATT-005, RCP-001 — RCP-004, CKP-001 — CKP-005 (14) |
| 4 | Error Types | ERR-001 — ERR-005 (5) |
| 5 | Hashing | HSH-001 — HSH-008 (8) |
| 6 | Structural Validation | SVL-001 — SVL-006 (6) |
| 7 | Block Production | BLD-001 — BLD-007, CKP-006 (8) |
| 8 | Serialization | SER-001 — SER-005 (5) |
| 9 | Execution Validation | EXE-001 — EXE-009 (9) |
| 10 | State Validation | STV-001 — STV-007 (7) |
| **Total** | | **74** |
