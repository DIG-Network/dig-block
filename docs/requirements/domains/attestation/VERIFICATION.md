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
| ATT-003 | Pending | BlockStatus Enum                     | Unit test all variants for is_finalized() and is_canonical() expected values  |
| ATT-004 | Pending | SignerBitmap Core Methods             | Unit test new/from_bytes round-trip; test set_signed/has_signed; verify count, percentage, threshold; test MAX_VALIDATORS boundary |
| ATT-005 | Pending | SignerBitmap Merge and Indices        | Unit test merge bitwise OR; test mismatched validator_count rejection; verify signer_indices ordering |
