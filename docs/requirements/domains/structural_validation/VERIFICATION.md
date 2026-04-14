# Structural Validation - Verification Matrix

| Field   | Value                   |
|---------|-------------------------|
| Domain  | structural_validation   |
| Prefix  | SVL                     |
| Version | 1.0                     |
| Date    | 2026-04-14              |

| ID      | Status      | Summary                                    | Verification Approach                                                                 |
|---------|-------------|--------------------------------------------|---------------------------------------------------------------------------------------|
| SVL-001 | Not Started | Header Version Check                       | Unit tests with heights above/below DFSP_ACTIVATION_HEIGHT and u64::MAX sentinel      |
| SVL-002 | Not Started | Header DFSP Root Pre-Activation Check      | Unit tests with non-empty DFSP roots at pre-activation heights                        |
| SVL-003 | Not Started | Header Cost and Size Checks                | Unit tests with total_cost and block_size at boundary values around the limits         |
| SVL-004 | Not Started | Header Timestamp Future Bound              | Unit tests with timestamps at and beyond the 300s future window                       |
| SVL-005 | Not Started | Block Count Agreement                      | Unit tests with mismatched counts for each of the four count fields                   |
| SVL-006 | Not Started | Block Merkle Root and Integrity Checks     | Unit tests for each integrity sub-check: roots, duplicates, double spends, size limit |
