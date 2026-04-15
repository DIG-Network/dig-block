# Serialization - Verification Matrix

- **Domain:** serialization
- **Prefix:** SER
- **Spec:** DIG L2 Block Specification
- **Total Requirements:** 5

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| SER-001 | implemented | Bincode Serialization for All Types | Integration: `tests/test_ser_001_bincode_all_types.rs` — bincode round-trip per wire type; bincode vs JSON size probe on header; types embedding `SpendBundle` use stable re-serialize byte equality. Code review: consensus wire remains bincode ([NORMATIVE](NORMATIVE.md#ser-001-bincode-serialization-for-all-types)). |
| SER-002 | implemented | to_bytes and from_bytes Conventions | Integration: `tests/test_ser_002_to_from_bytes.rs` — infallible `to_bytes` on wire types; `from_bytes` round-trip and empty/truncated/garbage → `InvalidData` with non-empty message; checkpoint vs block error domains. |
| SER-003 | implemented | Genesis Block Construction | Integration: `tests/test_ser_003_genesis_header.rs` — every NORMATIVE field for `genesis()`; timestamp within 5s of wall clock; version matches `protocol_version_for_height(0)`; SER-002 wire round-trip. |
| SER-004 | gap | Serde Default Attributes | Unit test: serialize a type without optional fields, deserialize into the current struct with new fields. Verify defaults are applied. Code review: confirm #[serde(default)] on all post-v1 fields. |
| SER-005 | gap | Serialization Round-Trip Integrity | Property test: for each type, generate random instances and verify from_bytes(to_bytes(x)) == x. Use proptest with Arbitrary implementations. Cover edge cases: empty blocks, max-size fields, zero values. |
