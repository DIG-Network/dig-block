# Block Types - Verification Matrix

> **Domain:** block_types
> **Prefix:** BLK
> **Normative:** [NORMATIVE.md](./NORMATIVE.md)
> **Tracking:** [TRACKING.yaml](./TRACKING.yaml)

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| BLK-001 | done | L2BlockHeader Struct | `tests/block_types/test_l2_block_header_struct.rs`: all field groups populated/typed; bincode round-trip; clone/debug; optional L1 proof `None` + serde defaults. `tests/common` helpers updated for full header literals. |
| BLK-002 | gap | L2BlockHeader Constructors | Unit test: call new(), new_with_collateral(), new_with_l1_proofs(), genesis() and verify returned headers have correct field values. Verify genesis sets parent_hash to network_id with zeroed counts/roots. |
| BLK-003 | gap | L2Block Struct | Unit test: construct L2Block with header, spend_bundles, slash_proposal_payloads, proposer_signature. Verify hash() delegates to header.hash(), height() and epoch() return header values. |
| BLK-004 | gap | L2Block Helper Methods | Unit test: build block with known spend bundles, verify compute_spends_root(), compute_additions_root(), compute_removals_root(), compute_filter_hash(), compute_slash_proposals_root() produce expected Merkle roots. Test all_additions(), all_removals(), has_duplicate_outputs(), has_double_spends(), compute_size(). |
| BLK-005 | done | Protocol Constants | `tests/block_types/test_protocol_constants.rs` (integration): EMPTY_ROOT vs `chia_sha2::Sha256` of `""`, ZERO_HASH zeros, limits/prefixes per BLK-005; public re-export smoke test. |
| BLK-006 | done | Primitive Types | `tests/block_types/test_primitive_types.rs`: Cost/u64 assignment, VERSION_V1/V2 values, pass `dig_block::{Bytes32, Signature, PublicKey}` into functions typed with `chia_protocol` / `chia_bls`; glob import smoke. |
| BLK-007 | gap | Version Auto-Detection | Unit test: construct header at height < DFSP_ACTIVATION_HEIGHT, verify VERSION_V1. Construct at height >= DFSP_ACTIVATION_HEIGHT, verify VERSION_V2. With DFSP_ACTIVATION_HEIGHT == u64::MAX, verify always VERSION_V1. |
