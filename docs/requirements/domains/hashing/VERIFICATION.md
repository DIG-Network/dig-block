# Hashing - Verification Matrix

> **Domain:** hashing
> **Prefix:** HSH
> **Normative:** [NORMATIVE.md](./NORMATIVE.md)
> **Tracking:** [TRACKING.yaml](./TRACKING.yaml)

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| HSH-001 | implemented | Block Header Hash | `tests/test_hsh_001_block_header_hash.rs`: `hash()` vs SHA-256(`hash_preimage_bytes()`); LE spot checks; `None` → ZERO_HASH slots; preimage length [`L2BlockHeader::HASH_PREIMAGE_LEN`] (710; SPEC table sum). |
| HSH-002 | gap | Checkpoint Hash | Unit test: construct checkpoint with known 9 fields, compute hash(), verify SHA-256 over 160-byte fixed-order concatenation (LE). Verify chia-sha2::Sha256 is used. |
| HSH-003 | gap | Spends Root Computation | Unit test: build block with known spend bundles, compute spends_root, verify Merkle root matches chia-sdk-types::MerkleTree over sha256(spend_bundle) in block order. Verify EMPTY_ROOT returned when no bundles. |
| HSH-004 | gap | Additions Root Construction | Unit test: build block with known additions, compute additions_root via chia-consensus::compute_merkle_set_root() over additions grouped by puzzle_hash. Verify [puzzle_hash, hash_coin_ids(coin_ids)] items. Cross-validate against Chia's block_body_validation.py:158-175. |
| HSH-005 | gap | Removals Root Construction | Unit test: build block with known removals, compute removals_root via chia-consensus::compute_merkle_set_root() over all removed coin IDs. Cross-validate against Chia's block_body_validation.py:185. |
| HSH-006 | gap | Filter Hash Construction | Unit test: build block with known additions and removals, compute filter_hash as SHA-256 of BIP158 compact filter over puzzle_hashes of additions and coin_ids of removals. Verify Golomb-Rice coding. Verify light client membership testing works. |
| HSH-007 | implemented | Tagged Merkle Hashing | `tests/test_hsh_007_tagged_merkle.rs`: `hash_leaf` / `hash_node` match `chia_sdk_types::MerkleTree`; domain separation; BLK-005 prefix values. `src/hash.rs` documents that `merkle_set` uses a different radix hash (HSH-004/5). |
| HSH-008 | gap | Receipts Root Computation | Unit test: compute_receipts_root over known receipts via chia-sdk-types::MerkleTree over SHA-256 of bincode-serialized receipts. Verify EMPTY_ROOT for empty list. Verify determinism. Verify order matters. Verify matches ReceiptList.root after finalize(). |
