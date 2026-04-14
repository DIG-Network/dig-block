# Attestation - Verification Matrix

| Field       | Value        |
|-------------|--------------|
| Domain      | attestation  |
| Prefix      | ATT          |
| Total Items | 5            |
| Status      | Draft        |

| ID      | Status  | Summary                              | Verification Approach                                                        |
|---------|---------|--------------------------------------|------------------------------------------------------------------------------|
| ATT-001 | Done | AttestedBlock Struct and Constructor | `tests/test_attested_block_constructor.rs`: field wiring, empty `SignerBitmap`, `Pending`, `aggregate_signature` == proposer `Signature`. |
| ATT-002 | Done | AttestedBlock Methods                | `tests/test_attested_block_methods.rs`: 0/50/100% paths, bitmap delegation, soft finality below/at/above 67%, `hash` == `block.hash()`. |
| ATT-003 | Done | BlockStatus Enum                     | `tests/test_block_status.rs`: per-variant `is_finalized` / `is_canonical` table; aggregate counts (2 finalized, 2 non-canonical). |
| ATT-004 | Done | SignerBitmap Core Methods             | `tests/test_signer_bitmap_core.rs`: ATT-004 test-plan rows (empty new, byte len, set/has, OOB `set_signed`, five-signer count, 30% / 0-validator %, threshold 67 vs 50, `from_bytes` round-trip, `MAX_VALIDATORS` 8192 bytes). |
| ATT-005 | Done | SignerBitmap Merge and Indices        | `tests/test_signer_bitmap_merge.rs`: disjoint/overlapping/empty OR, `ValidatorCountMismatch`, ascending `signer_indices`, union after merge. |
