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
| SVL-002 | implemented | Header DFSP Root Pre-Activation Check      | `tests/test_svl_002_dfsp_root_pre_activation.rs`: per-root `InvalidData` below finite activation; `Ok` at/above activation with non-empty roots; `validate()` with sentinel `u64::MAX` rejects poisoned roots at finite height. |
| SVL-003 | implemented | Header Cost and Size Checks                | `tests/test_svl_003_cost_and_size_checks.rs`: boundaries at MAX_COST_PER_BLOCK / MAX_BLOCK_SIZE; +1 rejects with CostExceeded / TooLarge; ordering when both exceed (cost first); validate() path. |
| SVL-004 | implemented | Header Timestamp Future Bound              | `tests/test_svl_004_timestamp_future_bound.rs`: `validate_with_dfsp_activation_at_unix` with fixed `now`; boundary `now+300` Ok, `now+301` `TimestampTooFarInFuture`; past and `timestamp==now` Ok; constant 300. |
| SVL-005 | implemented | Block Count Agreement                      | `tests/test_svl_005_block_count_agreement.rs`: empty block Ok; one-bundle aligned Ok; each mismatch variant with header/actual fields. |
| SVL-006 | Not Started | Block Merkle Root and Integrity Checks     | Unit tests for each integrity sub-check: roots, duplicates, double spends, size limit |
