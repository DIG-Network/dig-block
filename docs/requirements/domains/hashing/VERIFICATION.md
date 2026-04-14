# Hashing - Verification Matrix

> **Domain:** hashing
> **Prefix:** HSH
> **Normative:** [NORMATIVE.md](./NORMATIVE.md)
> **Tracking:** [TRACKING.yaml](./TRACKING.yaml)

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| HSH-001 | gap | Block Header Hash | Unit test: construct header with known field values, compute hash(), verify SHA-256 over fixed-order 33-field concatenation (626 bytes LE). Verify determinism: identical fields always produce the same Bytes32. Verify Optional fields hash as ZERO_HASH when None. Verify chia-sha2::Sha256 is used. |
| HSH-002 | gap | Checkpoint Hash | Unit test: construct checkpoint with known 9 fields, compute hash(), verify SHA-256 over 160-byte fixed-order concatenation (LE). Verify chia-sha2::Sha256 is used. |
| HSH-003 | gap | Spends Root Computation | Unit test: build block with known spend bundles, compute spends_root, verify Merkle root matches chia-sdk-types::MerkleTree over sha256(spend_bundle) in block order. Verify EMPTY_ROOT returned when no bundles. |
| HSH-004 | gap | Additions Root Construction | Unit test: build block with known additions, compute additions_root via chia-consensus::compute_merkle_set_root() over additions grouped by puzzle_hash. Verify [puzzle_hash, hash_coin_ids(coin_ids)] items. Cross-validate against Chia's block_body_validation.py:158-175. |
| HSH-005 | gap | Removals Root Construction | Unit test: build block with known removals, compute removals_root via chia-consensus::compute_merkle_set_root() over all removed coin IDs. Cross-validate against Chia's block_body_validation.py:185. |
| HSH-006 | gap | Filter Hash Construction | Unit test: build block with known additions and removals, compute filter_hash as SHA-256 of BIP158 compact filter over puzzle_hashes of additions and coin_ids of removals. Verify Golomb-Rice coding. Verify light client membership testing works. |
| HSH-007 | gap | Tagged Merkle Hashing | Unit test: verify leaf hash equals SHA-256(0x01 || data), node hash equals SHA-256(0x02 || left || right). Verify chia-consensus::merkle_set uses these tags. Verify dig-block does NOT redefine HASH_LEAF_PREFIX or HASH_TREE_PREFIX constants for Merkle computation. |
| HSH-008 | gap | Receipts Root Computation | Unit test: compute_receipts_root over known receipts via chia-sdk-types::MerkleTree over SHA-256 of bincode-serialized receipts. Verify EMPTY_ROOT for empty list. Verify determinism. Verify order matters. Verify matches ReceiptList.root after finalize(). |
