# Changelog

All notable changes to this project are documented here.
This project adheres to [Semantic Versioning](https://semver.org) and
[Conventional Commits](https://www.conventionalcommits.org).

## [0.2.1] - 2026-09-04

### Documentation
- Add CONTRIBUTING.md (#3)

## [0.2.0] - 2026-08-08

### Chores
- **deps:** Bump dig-clvm 0.1 -> 0.2 and release dig-block 0.2.0 (#2)

## [0.1.1] - 2026-07-12

### Testing
- Cover reachable edge branches (bitmap bounds/resize, empty-list, Default, decode errors)

### CI
- Measure line coverage and gate at >=80% (cargo-llvm-cov)- Enforce version increment in PRs (package.json / Cargo.toml)- Enforce Conventional Commits with commitlint on PRs- Enforce Conventional Commits with commitlint on PRs- Release automation (git-cliff changelog + tag on merge); publish is manual workflow_dispatch (#230)- Re-arm crates.io auto-publish on version tag (token in org secrets; auto-publish-everything #230)- Add flaky-test management (#489) (#1)

### Chores
- **changelog:** Add git-cliff config for Conventional-Commit changelog

## [0.1.0] - 2026-04-16

### Features
- **BLK-005:** Protocol constants with spec-aligned tests and tracking- **BLK-006:** Primitive types module and dedicated tests- **BLK-001:** L2BlockHeader field groups, derives, and dedicated tests- **BLK-002:** L2BlockHeader constructors and genesis- **BLK-007:** Version auto-detection tests and parameterized helper- **block:** BLK-003 L2Block and header hash (HSH-001)- **block:** BLK-004 L2Block helpers (Merkle, BIP158, integrity)- **attestation:** ATT-003 BlockStatus predicates and tests- **attestation:** ATT-004 SignerBitmap core API- **attestation:** ATT-005 SignerBitmap merge and signer_indices- **attestation:** ATT-001 AttestedBlock struct and new()- **attestation:** ATT-002 AttestedBlock query methods- **receipt:** RCP-001 ReceiptStatus repr(u8) and wire helpers- **receipt:** RCP-002 Receipt struct and constructor- **receipt:** RCP-003 ReceiptList and receipts Merkle root- **receipt:** RCP-004 ReceiptList aggregate methods- **checkpoint:** Implement CKP-003 CheckpointStatus enum- **checkpoint:** Implement CKP-001 Checkpoint struct- **checkpoint:** Implement CKP-002 CheckpointSubmission- **checkpoint:** Implement CKP-004 Checkpoint::compute_score- **checkpoint:** Implement CKP-005 submission methods and Checkpoint::hash- **builder:** Implement CKP-006 CheckpointBuilder- **ERR-001:** BlockError Tier-1 structural variants- **ERR-002:** Execution/state BlockError variants; flat tests/- **error:** Implement CheckpointError per ERR-003- **error:** Implement BuilderError per ERR-004- **error:** Implement ERR-005 SignerBitmapError and ReceiptError- **hash:** Implement HSH-007 tagged Merkle helpers- **hash:** HSH-001 block header preimage + tests- **hash:** HSH-002 checkpoint hash preimage + tests- **hashing:** Implement HSH-003 compute_spends_root and tests- **hashing:** HSH-004 compute_additions_root and dedicated tests- **hashing:** HSH-005 compute_removals_root and dedicated tests- **hashing:** HSH-006 compute_filter_hash and compact_block_filter_encoded- **hashing:** HSH-008 public compute_receipts_root and integration tests- **validation:** SVL-001 header version check and InvalidVersion context- **SVL-003:** Enforce header cost and block size limits- **SVL-004:** Enforce header timestamp future bound- **SVL-005:** L2Block validate_structure count agreement- **validation:** Implement SVL-006 Merkle and integrity checks- **builder:** Implement BLD-001 BlockBuilder fields and new()- **builder:** Implement BLD-002 add_spend_bundle with cost/size budgets- **builder:** Implement BLD-003 add_slash_proposal limits- **builder:** Implement BLD-004 optional L1/DFSP/extension setters- **BLD-005:** BlockBuilder build pipeline and integration tests- **ser-001:** Bincode round-trip tests and execution serde surface- **ser-002:** To_bytes/from_bytes on wire types with typed decode errors- **ser-003:** Genesis header tests and spec alignment- **ser-004:** Serde default attributes for backwards compat- **ser-005:** Property-based round-trip integrity for all wire types- **exe-008:** ExecutionResult Tier-2 to Tier-3 bridge struct- **exe-009:** Dedicated test file for PendingAssertion / AssertionKind- **exe-001:** L2Block::validate_execution API surface- **exe-002:** Per-CoinSpend puzzle hash verification- **exe-003:** Dig-clvm CLVM delegation + ValidationError mapping- **exe-004:** Condition parsing -- pending-assertion collection- **exe-005:** BLS signature verification delegated to dig-clvm- **exe-006:** Coin conservation + block-level fee consistency- **exe-007:** Block-level cost consistency verification- **stv-001:** L2Block::validate_state + validate_full API surface- **stv-002:** Coin existence checks for Tier-3 removals- **stv-003:** Tier-3 puzzle hash cross-check vs coin state- **stv-004:** Tier-3 addition non-existence with ephemeral exception- **stv-005:** Height / time lock evaluation for PendingAssertion- **stv-006:** Proposer signature verification via chia_bls::verify- **stv-007:** State root verification via delta-hash helper- **crate:** Wire dig-block as standalone crate with public surface

### Bug Fixes
- **ci:** Split __blk004 re-export into its own group for rustfmt stability

### Refactor
- **tests:** One STR-* file per requirement

### Testing
- **BLD-006:** BlockSigner integration tests and spec alignment- **BLD-007:** Builder outputs pass validate_structure

### CI
- Add crates.io publish workflow on v* tags

### Chores
- Add root package.json for local gitnexus (npm/npx on Windows)- Silence clippy warnings across test suite


