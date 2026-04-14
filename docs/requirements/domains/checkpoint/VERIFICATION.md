# Checkpoint - Verification Matrix

| Field       | Value        |
|-------------|--------------|
| Domain      | checkpoint   |
| Prefix      | CKP          |
| Total Items | 6            |
| Status      | Draft        |

| ID      | Status  | Summary                        | Verification Approach                                                                          |
|---------|---------|--------------------------------|-----------------------------------------------------------------------------------------------|
| CKP-001 | Pending | Checkpoint Struct              | Unit test struct fields and types; verify new() constructor creates valid default instance      |
| CKP-002 | Pending | CheckpointSubmission Struct    | Unit test struct fields; verify new() sets submission_height and submission_coin to None        |
| CKP-003 | Done    | CheckpointStatus Enum          | Unit test all variant patterns including data-carrying variants WinnerSelected and Finalized (`tests/checkpoint/test_checkpoint_status.rs`) |
| CKP-004 | Pending | Checkpoint Score Computation   | Unit test compute_score with various stake_percentage and block_count values                    |
| CKP-005 | Pending | CheckpointSubmission Methods   | Unit test hash/epoch delegation; verify signing_percentage, meets_threshold, record_submission, is_submitted |
| CKP-006 | Pending | CheckpointBuilder              | Unit test builder pipeline; verify Merkle root computation for block_root and withdrawals_root  |
