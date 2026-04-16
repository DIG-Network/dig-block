# State Validation - Verification Matrix

- **Domain:** state_validation
- **Prefix:** STV
- **Spec:** DIG L2 Block Specification
- **Total Requirements:** 7

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| STV-001 | implemented | validate_state API | Integration: `tests/test_stv_001_validate_state_api.rs` — 9 tests: signatures of both methods (compile-time fn pointer coercion); empty-block `validate_state` returns `header.state_root`; `validate_full` happy path; tier-1 short-circuit via `SpendBundleCountMismatch`; tier-2 short-circuit via `FeesMismatch`; tier-2 output feeds tier-3 input; return type is `Bytes32`; object safety via `Box<dyn CoinLookup>`. Sub-checks STV-002..007 are stubs until their requirements land. |
| STV-002 | implemented | Coin Existence Checks | Integration: `tests/test_stv_002_coin_existence.rs` — 7 tests: persistent unspent passes; persistent spent rejects `CoinAlreadySpent`; missing non-ephemeral rejects `CoinNotFound`; ephemeral (in `exec.additions`) passes; mixed removals all pass; removal order independence; first-failure-in-list reporting. Chia Check 15 parity. |
| STV-003 | gap | Puzzle Hash Cross-Check | Unit test: matching puzzle_hash passes. Mismatched puzzle_hash returns PuzzleHashMismatch. Verify comparison uses coin_state.coin.puzzle_hash from CoinLookup. |
| STV-004 | gap | Addition Non-Existence | Unit test: new coin (not in CoinLookup) passes. Existing coin returns CoinAlreadyExists. Ephemeral coin created and spent in same block is allowed. |
| STV-005 | gap | Height/Time Lock Evaluation | Unit test per assertion type: ASSERT_HEIGHT_ABSOLUTE, ASSERT_HEIGHT_RELATIVE, ASSERT_SECONDS_ABSOLUTE, ASSERT_SECONDS_RELATIVE, BEFORE_HEIGHT_ABSOLUTE, BEFORE_HEIGHT_RELATIVE, BEFORE_SECONDS_ABSOLUTE, BEFORE_SECONDS_RELATIVE. Each with passing and failing conditions. |
| STV-006 | gap | Proposer Signature Verification | Unit test: valid proposer signature passes. Invalid signature returns InvalidProposerSignature. Wrong pubkey returns InvalidProposerSignature. |
| STV-007 | gap | State Root Verification | Unit test: correct state root after additions/removals matches header. Tampered state_root in header returns InvalidStateRoot. Verify Merkle root computation with known test vectors. |
