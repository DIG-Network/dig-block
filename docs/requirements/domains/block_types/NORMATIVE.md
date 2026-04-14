# Block Types - Normative Requirements

> **Domain:** block_types
> **Prefix:** BLK
> **Spec reference:** [SPEC.md - Sections 2.1-2.3, 2.11](../../../resources/SPEC.md)

## Requirements

### BLK-001: L2BlockHeader Struct

L2BlockHeader MUST contain all field groups:

- **Core identity:** version (u16), height (u64), epoch (u64), parent_hash (Bytes32)
- **State commitments:** state_root, spends_root, additions_root, removals_root, receipts_root (all Bytes32)
- **L1 anchor:** l1_height (u32), l1_hash (Bytes32)
- **Block metadata:** timestamp (u64), proposer_index (u32), spend_bundle_count (u32), total_cost (Cost), total_fees (u64), additions_count (u32), removals_count (u32), block_size (u32), filter_hash (Bytes32), extension_data (Bytes32)
- **L1 proof anchors:** 5 Optional Bytes32 fields
- **Slash proposals:** slash_proposal_count (u32), slash_proposals_root (Bytes32)
- **DFSP roots:** 5 Bytes32 fields

L2BlockHeader MUST derive Serialize, Deserialize, Clone, Debug, PartialEq.

**Spec reference:** SPEC Section 2.2

### BLK-002: L2BlockHeader Constructors

L2BlockHeader MUST provide the following constructors:

- `new()` with automatic version detection based on height
- `new_with_collateral()` with L1 collateral proof
- `new_with_l1_proofs()` with all 5 L1 proof anchors
- `genesis(network_id, l1_height, l1_hash)` with parent_hash set to network_id and all counts/roots set to zero/empty

**Spec reference:** SPEC Section 2.2 (Derived methods)

### BLK-003: L2Block Struct

L2Block MUST contain the following fields:

- header: L2BlockHeader
- spend_bundles: Vec\<SpendBundle\>
- slash_proposal_payloads: Vec\<Vec\<u8\>\>
- proposer_signature: Signature

L2Block MUST provide a `new()` constructor, `hash()` delegating to `header.hash()`, and `height()` and `epoch()` delegation methods.

**Spec reference:** SPEC Section 2.3

### BLK-004: L2Block Helper Methods

L2Block MUST provide the following helper methods:

- `compute_spends_root()` - Merkle root of spend bundles
- `compute_additions_root()` - Merkle root of additions (Chia parity: grouped by puzzle_hash)
- `compute_removals_root()` - Merkle root of removals (Chia parity)
- `compute_filter_hash()` - BIP158 compact block filter hash
- `compute_slash_proposals_root()` - Merkle root of slash proposals
- `slash_proposal_leaf_hash()` - hash of a single slash proposal leaf
- `all_additions() -> Vec<Coin>` - collect all additions across spend bundles
- `all_addition_ids()` - collect all addition coin IDs
- `all_removals() -> Vec<CoinId>` - collect all removals across spend bundles
- `has_duplicate_outputs() -> Option<Bytes32>` - detect duplicate output coins
- `has_double_spends() -> Option<Bytes32>` - detect double-spent inputs
- `compute_size() -> usize` - compute serialized block size

**Spec reference:** SPEC Section 2.3, 10.6

### BLK-005: Protocol Constants

The crate MUST define the following protocol constants:

- `EMPTY_ROOT` = SHA256("") = 0xe3b0c442...
- `ZERO_HASH` = all zeros
- `MAX_BLOCK_SIZE` = 10_000_000
- `MAX_COST_PER_BLOCK: Cost` = 550_000_000_000
- `MAX_SLASH_PROPOSALS_PER_BLOCK` = 64
- `MAX_SLASH_PROPOSAL_PAYLOAD_BYTES` = 65_536
- `DFSP_ACTIVATION_HEIGHT` = u64::MAX
- `MAX_FUTURE_TIMESTAMP_SECONDS` = 300
- `HASH_LEAF_PREFIX` = 0x01
- `HASH_TREE_PREFIX` = 0x02

**Spec reference:** SPEC Section 2.11

### BLK-006: Primitive Types

The crate MUST define:

- `Cost` type alias as u64
- `VERSION_V1: u16` = 1
- `VERSION_V2: u16` = 2

Bytes32 MUST be re-exported from chia-protocol. Signature and PublicKey MUST be re-exported from chia-bls.

**Spec reference:** SPEC Section 2.1

### BLK-007: Version Auto-Detection

L2BlockHeader constructors MUST auto-detect version from height: if height >= DFSP_ACTIVATION_HEIGHT then VERSION_V2, else VERSION_V1. If DFSP_ACTIVATION_HEIGHT == u64::MAX, version is always VERSION_V1. Callers MUST NOT manually specify version.

**Spec reference:** SPEC Section 2.2 (Version semantics), Design Decision #10
