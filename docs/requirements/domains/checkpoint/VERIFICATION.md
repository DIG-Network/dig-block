# Checkpoint - Verification Matrix

| Field       | Value        |
|-------------|--------------|
| Domain      | checkpoint   |
| Prefix      | CKP          |
| Total Items | 6            |
| Status      | Draft        |

| ID      | Status  | Summary                        | Verification Approach                                                                          |
|---------|---------|--------------------------------|-----------------------------------------------------------------------------------------------|
| CKP-001 | Done    | Checkpoint Struct              | Unit test struct fields and types; verify new() constructor creates valid default instance (`tests/checkpoint/test_checkpoint_struct.rs`) |
| CKP-002 | Done    | CheckpointSubmission Struct    | Unit test struct fields; verify new() sets submission_height and submission_coin to None (`tests/checkpoint/test_checkpoint_submission_struct.rs`) |
| CKP-003 | Done    | CheckpointStatus Enum          | Unit test all variant patterns including data-carrying variants WinnerSelected and Finalized (`tests/checkpoint/test_checkpoint_status.rs`) |
| CKP-004 | Done    | Checkpoint Score Computation   | Unit test compute_score with various stake_percentage and block_count values (`tests/checkpoint/test_checkpoint_score.rs`) |
| CKP-005 | Done    | CheckpointSubmission Methods   | Unit test hash/epoch delegation; verify signing_percentage, meets_threshold, record_submission, is_submitted (`tests/checkpoint/test_checkpoint_submission_methods.rs`); `Checkpoint::hash` per HSH-002 / SPEC §3.2 |
| CKP-006 | Pending | CheckpointBuilder              | Unit test builder pipeline; verify Merkle root computation for block_root and withdrawals_root  |
