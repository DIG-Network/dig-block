# Hashing - Verification Matrix

> **Domain:** hashing
> **Prefix:** HSH
> **Normative:** [NORMATIVE.md](./NORMATIVE.md)
> **Tracking:** [TRACKING.yaml](./TRACKING.yaml)

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| HSH-001 | implemented | Block Header Hash | `tests/test_hsh_001_block_header_hash.rs`: `hash()` vs SHA-256(`hash_preimage_bytes()`); LE spot checks; `None` → ZERO_HASH slots; preimage length [`L2BlockHeader::HASH_PREIMAGE_LEN`] (710; SPEC table sum). |
| HSH-002 | implemented | Checkpoint Hash | `tests/test_hsh_002_checkpoint_hash.rs`: `hash()` vs SHA-256(`hash_preimage_bytes()`); LE spot checks; length 160; submission hash delegation. |
| HSH-003 | implemented | Spends Root Computation | `tests/test_hsh_003_spends_root.rs`: `compute_spends_root` empty → `EMPTY_ROOT`; leaf = SHA-256(`to_bytes()`); leaf matches `SpendBundle::name`; Merkle roots vs `MerkleTree`; order sensitivity; `L2Block::compute_spends_root` delegates. |
| HSH-004 | implemented | Additions Root Construction | `tests/test_hsh_004_additions_root.rs`: empty → `EMPTY_ROOT`; single/multi group vs manual `compute_merkle_set_root`; same-`puzzle_hash` grouping; first-seen order; parity with two-bundle BLK-004 scenario; `L2Block::compute_additions_root` delegates to `compute_additions_root`. |
| HSH-005 | implemented | Removals Root Construction | `tests/test_hsh_005_removals_root.rs`: empty → `EMPTY_ROOT`; single/multi vs manual `compute_merkle_set_root`; permutation invariance; spend-bundle parity; `L2Block::compute_removals_root` delegates to `compute_removals_root`. |
| HSH-006 | implemented | Filter Hash Construction | `tests/test_hsh_006_filter_hash.rs`: empty determinism; additions-only / removals-only / combined `BlockFilter::match_any`; unlikely-element negative probe; repeatability; `L2Block::compute_filter_hash` delegates to `compute_filter_hash`. |
| HSH-007 | implemented | Tagged Merkle Hashing | `tests/test_hsh_007_tagged_merkle.rs`: `hash_leaf` / `hash_node` match `chia_sdk_types::MerkleTree`; domain separation; BLK-005 prefix values. `src/hash.rs` documents that `merkle_set` uses a different radix hash (HSH-004/5). |
| HSH-008 | gap | Receipts Root Computation | Unit test: compute_receipts_root over known receipts via chia-sdk-types::MerkleTree over SHA-256 of bincode-serialized receipts. Verify EMPTY_ROOT for empty list. Verify determinism. Verify order matters. Verify matches ReceiptList.root after finalize(). |
