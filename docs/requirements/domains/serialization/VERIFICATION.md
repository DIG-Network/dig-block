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
| SER-004 | implemented | Serde Default Attributes | Integration: `tests/test_ser_004_serde_defaults.rs` — `L2BlockHeader::genesis` round-trips with L1 proof Options=None, DFSP roots=EMPTY_ROOT, slash fields=0/EMPTY_ROOT, extension_data=ZERO_HASH; full-field round-trip preserves all 33 fields; `Checkpoint` and `CheckpointSubmission` round-trip stable (incl. submission_height/coin Options). Code review: `#[serde(default)]` on L1 proof anchor fields in `src/types/header.rs`. |
| SER-005 | implemented | Serialization Round-Trip Integrity | Integration: `tests/test_ser_005_roundtrip_integrity.rs` — `proptest` strategies for all wire types; PartialEq round-trip check for `L2BlockHeader`, `Checkpoint`, `Receipt`, `ReceiptList`, `SignerBitmap`, `BlockStatus`, `CheckpointStatus`; byte-stable `to_bytes(from_bytes(to_bytes(x))) == to_bytes(x)` for `L2Block` / `AttestedBlock` / `CheckpointSubmission` (embedded `chia-protocol` / `chia-bls` types lack `PartialEq`). Edge cases: empty body, `u*::MAX`, all-zero hashes, `MAX_VALIDATORS` bitmap, empty `ReceiptList`. |
