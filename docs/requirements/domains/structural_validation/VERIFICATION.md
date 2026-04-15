# Structural Validation - Verification Matrix

| Field   | Value                   |
|---------|-------------------------|
| Domain  | structural_validation   |
| Prefix  | SVL                     |
| Version | 1.0                     |
| Date    | 2026-04-14              |

| ID      | Status      | Summary                                    | Verification Approach                                                                 |
|---------|-------------|--------------------------------------------|---------------------------------------------------------------------------------------|
| SVL-001 | implemented | Header Version Check                       | `tests/test_svl_001_header_version_check.rs`: finite activation vs V1/V2; `validate` with `DFSP_ACTIVATION_HEIGHT == u64::MAX`; `InvalidVersion` expected/actual; `validate` vs `validate_with_dfsp_activation`.      |
| SVL-002 | Not Started | Header DFSP Root Pre-Activation Check      | Unit tests with non-empty DFSP roots at pre-activation heights                        |
| SVL-003 | Not Started | Header Cost and Size Checks                | Unit tests with total_cost and block_size at boundary values around the limits         |
| SVL-004 | Not Started | Header Timestamp Future Bound              | Unit tests with timestamps at and beyond the 300s future window                       |
| SVL-005 | Not Started | Block Count Agreement                      | Unit tests with mismatched counts for each of the four count fields                   |
| SVL-006 | Not Started | Block Merkle Root and Integrity Checks     | Unit tests for each integrity sub-check: roots, duplicates, double spends, size limit |
