# Hashing - Normative Requirements

> **Domain:** hashing
> **Prefix:** HSH
> **Spec reference:** [SPEC.md - Sections 3.1-3.3, 3.4-3.6](../../../resources/SPEC.md)

## Requirements

### HSH-001: Block Header Hash

Block header hash MUST be SHA-256 over a fixed-order concatenation of ALL header fields in the exact order specified in SPEC Section 3.1 (33 fields, 626 bytes total). Numeric fields MUST be encoded in little-endian byte order. Optional fields MUST hash as ZERO_HASH when None. The implementation MUST use chia-sha2::Sha256 as the hashing primitive. Given identical field values, hash() MUST always produce the same Bytes32 (deterministic output).

**Spec reference:** SPEC Section 3.1

### HSH-002: Checkpoint Hash

Checkpoint hash MUST be SHA-256 over 9 fields in fixed order (160 bytes total): epoch (LE), state_root, block_root, block_count (LE), tx_count (LE), total_fees (LE), prev_checkpoint, withdrawals_root, withdrawal_count (LE). The implementation MUST use chia-sha2::Sha256.

**Spec reference:** SPEC Section 3.2

### HSH-003: Spends Root Computation

spends_root MUST be computed via chia-sdk-types::MerkleTree over sha256(spend_bundle) for each SpendBundle in block order. MUST return EMPTY_ROOT if no bundles are present.

**Spec reference:** SPEC Section 3.3

### HSH-004: Additions Root Construction

additions_root MUST be computed via chia-consensus::compute_merkle_set_root() over additions grouped by puzzle_hash. For each puzzle_hash, items are \[puzzle_hash, hash_coin_ids(coin_ids)\]. This MUST match Chia's block_body_validation.py:158-175 exactly.

**Spec reference:** SPEC Section 3.4

### HSH-005: Removals Root Construction

removals_root MUST be computed via chia-consensus::compute_merkle_set_root() over all removed coin IDs. This MUST match Chia's block_body_validation.py:185.

**Spec reference:** SPEC Section 3.5

### HSH-006: Filter Hash Construction

filter_hash MUST be SHA-256 of a BIP158 compact filter. Filter input MUST include the puzzle_hash of each addition and the coin_id of each removal. The filter MUST be Golomb-Rice coded. This enables light client membership testing.

**Spec reference:** SPEC Section 3.6

### HSH-007: Tagged Merkle Hashing

All block-level Merkle trees MUST use tagged hashing: leaf = SHA-256(0x01 || data), node = SHA-256(0x02 || left || right). This is built into chia-consensus::merkle_set -- dig-block MUST NOT redefine these constants. Tagged hashing prevents second-preimage attacks. Chia parity: merkle_utils.py.

**Spec reference:** SPEC Section 3.3

### HSH-008: Receipts Root Computation

receipts_root MUST be computed via chia-sdk-types::MerkleTree over SHA-256 hashes of each Receipt (bincode-serialized) in block order. MUST return EMPTY_ROOT if no receipts are present. This is the same MerkleTree construction used for spends_root (HSH-003) and slash_proposals_root. Used by ReceiptList::finalize() and ReceiptList::from_receipts().

**Spec reference:** SPEC Section 3.3
