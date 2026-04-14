# Serialization - Verification Matrix

- **Domain:** serialization
- **Prefix:** SER
- **Spec:** DIG L2 Block Specification
- **Total Requirements:** 5

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| SER-001 | gap | Bincode Serialization for All Types | Unit test: verify each type (L2BlockHeader, L2Block, AttestedBlock, Checkpoint, CheckpointSubmission) serializes with bincode. Verify output is compact binary (no JSON/schema overhead). Code review: confirm bincode is the only serialization backend. |
| SER-002 | gap | to_bytes and from_bytes Conventions | Unit test: to_bytes() returns Vec<u8> without error for valid types. from_bytes() with valid bytes returns Ok. from_bytes() with invalid/truncated bytes returns BlockError::InvalidData or CheckpointError::InvalidData. Verify to_bytes does not return Result (infallible). |
| SER-003 | gap | Genesis Block Construction | Unit test: call L2BlockHeader::genesis(network_id, l1_height, l1_hash) and verify every field. height=0, epoch=0, parent_hash=network_id, roots=EMPTY_ROOT, filter_hash=EMPTY_ROOT, extension_data=ZERO_HASH, counts/costs=0, L1 proofs=None, timestamp is recent, version matches crate. |
| SER-004 | gap | Serde Default Attributes | Unit test: serialize a type without optional fields, deserialize into the current struct with new fields. Verify defaults are applied. Code review: confirm #[serde(default)] on all post-v1 fields. |
| SER-005 | gap | Serialization Round-Trip Integrity | Property test: for each type, generate random instances and verify from_bytes(to_bytes(x)) == x. Use proptest with Arbitrary implementations. Cover edge cases: empty blocks, max-size fields, zero values. |
