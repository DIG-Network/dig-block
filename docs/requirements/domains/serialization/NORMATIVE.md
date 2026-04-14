# Serialization - Normative Requirements

- **Domain:** serialization
- **Prefix:** SER
- **Spec:** DIG L2 Block Specification
- **Total Requirements:** 5

## Requirements

### SER-001: Bincode Serialization for All Types

All block types (`L2BlockHeader`, `L2Block`, `AttestedBlock`, `Checkpoint`, `CheckpointSubmission`) **MUST** use bincode for serialization. All supporting types (`SignerBitmap`, `Receipt`, `ReceiptList`, `BlockStatus`, `CheckpointStatus`, `ExecutionResult`, `PendingAssertion`, `RewardDistribution`) **MUST** also derive `Serialize` and `Deserialize` for bincode compatibility. Bincode produces compact binary with no schema overhead, making it suitable for high-throughput block processing and network transmission.

**Spec reference:** SPEC Section 8.1

---

### SER-002: to_bytes and from_bytes Conventions

`to_bytes()` **MUST** be infallible (panics on failure, which should never happen with well-formed types). `from_bytes()` **MUST** be fallible, returning `BlockError::InvalidData` or `CheckpointError::InvalidData` on failure.

**Spec reference:** SPEC Section 8.2

---

### SER-003: Genesis Block Construction

`L2BlockHeader::genesis(network_id, l1_height, l1_hash)` **MUST** set:

- `height` = 0
- `epoch` = 0
- `parent_hash` = `network_id`
- All roots = `EMPTY_ROOT`
- `filter_hash` = `EMPTY_ROOT`
- `extension_data` = `ZERO_HASH`
- All counts/costs = 0
- All L1 proofs = `None`
- `timestamp` = current wall-clock time
- `version` = auto-detected from crate version

**Spec reference:** SPEC Section 8.3

---

### SER-004: Serde Default Attributes

Fields added in later protocol versions **MUST** use `#[serde(default)]` or `#[serde(default = "...")]` for backwards compatibility with older serialized data. This ensures that deserialization of blocks from earlier protocol versions succeeds even when new fields are not present.

**Spec reference:** SPEC Section 8.2

---

### SER-005: Serialization Round-Trip Integrity

For all types: `from_bytes(to_bytes(x))` **MUST** equal `x`. No data loss **MUST** occur through serialization round-trips. This is a property test requirement that **SHOULD** be verified using proptest or similar property-based testing frameworks.

**Spec reference:** SPEC Section 12.1 (Serialization round-trips), 12.5
