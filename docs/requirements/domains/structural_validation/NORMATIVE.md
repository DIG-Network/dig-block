# Structural Validation - Normative Requirements

| Field   | Value                   |
|---------|-------------------------|
| Domain  | structural_validation   |
| Prefix  | SVL                     |
| Version | 1.0                     |
| Date    | 2026-04-14              |

## Requirements

### SVL-001: Header Version Check

`L2BlockHeader::validate()` MUST compute `expected_version` from `height` and `DFSP_ACTIVATION_HEIGHT`.

- If `height >= DFSP_ACTIVATION_HEIGHT`, expected MUST be `VERSION_V2` (2); otherwise `VERSION_V1` (1).
- If `DFSP_ACTIVATION_HEIGHT == u64::MAX`, expected MUST always be `VERSION_V1`.
- The implementation MUST reject with `BlockError::InvalidVersion` if `version != expected`.

**Spec reference:** SPEC Section 5.1 Step 1

---

### SVL-002: Header DFSP Root Pre-Activation Check

`L2BlockHeader::validate()` MUST check that if `height < DFSP_ACTIVATION_HEIGHT`, all five DFSP roots equal `EMPTY_ROOT`:

- `collateral_registry_root`
- `cid_state_root`
- `node_registry_root`
- `namespace_update_root`
- `dfsp_finalize_commitment_root`

The implementation MUST reject with `BlockError::InvalidData` if any DFSP root is non-empty before activation.

**Spec reference:** SPEC Section 5.1 Step 2

---

### SVL-003: Header Cost and Size Checks

`L2BlockHeader::validate()` MUST enforce the following limits:

- MUST reject with `BlockError::CostExceeded` if `total_cost > MAX_COST_PER_BLOCK`.
- MUST reject with `BlockError::TooLarge` if `block_size > MAX_BLOCK_SIZE`.

**Spec reference:** SPEC Section 5.1 Steps 3-4

---

### SVL-004: Header Timestamp Future Bound

`L2BlockHeader::validate()` MUST reject with `BlockError::TimestampTooFarInFuture` if `timestamp > now() + MAX_FUTURE_TIMESTAMP_SECONDS` (default 300s).

This requirement is adopted from Chia `block_header_validation.py` Check 26a.

**Spec reference:** SPEC Section 5.1 Step 5

---

### SVL-005: Block Count Agreement

`L2Block::validate_structure()` MUST verify that the following header counts match the actual block contents:

- `header.spend_bundle_count == spend_bundles.len()` -- reject with `SpendBundleCountMismatch`
- `header.additions_count == computed additions count` -- reject with `AdditionsCountMismatch`
- `header.removals_count == computed removals count` -- reject with `RemovalsCountMismatch`
- `header.slash_proposal_count == slash_proposal_payloads.len()` -- reject with `SlashProposalCountMismatch`

**Spec reference:** SPEC Section 5.2 Steps 2, 4, 5, 13

---

### SVL-006: Block Merkle Root and Integrity Checks

`L2Block::validate_structure()` MUST perform the following integrity checks:

- Recompute `spends_root` and compare -- reject with `InvalidSpendsRoot`
- Check no duplicate outputs (Chia Check 13) -- reject with `DuplicateOutput`
- Check no double spends (Chia Check 14) -- reject with `DoubleSpendInBlock`
- Recompute `additions_root` -- reject with `InvalidAdditionsRoot`
- Recompute `removals_root` -- reject with `InvalidRemovalsRoot`
- Recompute `filter_hash` via BIP158 -- reject with `InvalidFilterHash`
- Check `slash_proposal_count <= MAX_SLASH_PROPOSALS_PER_BLOCK` -- reject with `TooManySlashProposals`
- Check each payload `<= MAX_SLASH_PROPOSAL_PAYLOAD_BYTES` -- reject with `SlashProposalPayloadTooLarge`
- Recompute `slash_proposals_root` -- reject with `InvalidSlashProposalsRoot`
- Compute serialized size `<= MAX_BLOCK_SIZE` -- reject with `TooLarge`

**Spec reference:** SPEC Section 5.2 Steps 3, 6-15
