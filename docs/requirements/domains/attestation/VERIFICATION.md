# Attestation - Verification Matrix

| Field       | Value        |
|-------------|--------------|
| Domain      | attestation  |
| Prefix      | ATT          |
| Total Items | 5            |
| Status      | Draft        |

| ID      | Status  | Summary                              | Verification Approach                                                        |
|---------|---------|--------------------------------------|------------------------------------------------------------------------------|
| ATT-001 | Pending | AttestedBlock Struct and Constructor | Unit test struct fields; verify new() sets empty bitmap, Pending status, and proposer signature |
| ATT-002 | Pending | AttestedBlock Methods                | Unit test signing_percentage range 0-100; test has_soft_finality at boundary; verify hash delegation |
| ATT-003 | Done | BlockStatus Enum                     | `tests/attestation/test_block_status.rs`: per-variant `is_finalized` / `is_canonical` table; aggregate counts (2 finalized, 2 non-canonical). |
| ATT-004 | Done | SignerBitmap Core Methods             | `tests/attestation/test_signer_bitmap_core.rs`: ATT-004 test-plan rows (empty new, byte len, set/has, OOB `set_signed`, five-signer count, 30% / 0-validator %, threshold 67 vs 50, `from_bytes` round-trip, `MAX_VALIDATORS` 8192 bytes). |
| ATT-005 | Done | SignerBitmap Merge and Indices        | `tests/attestation/test_signer_bitmap_merge.rs`: disjoint/overlapping/empty OR, `ValidatorCountMismatch`, ascending `signer_indices`, union after merge. |
