# Execution Validation - Verification Matrix

- **Domain:** execution_validation
- **Prefix:** EXE
- **Spec:** DIG L2 Block Specification
- **Total Requirements:** 9

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| EXE-001 | gap | validate_execution API | Unit test: call validate_execution on a valid block and verify Ok(ExecutionResult); call on invalid block and verify appropriate BlockError. Integration test: end-to-end block validation through Tier 2. |
| EXE-002 | gap | Puzzle Hash Verification | Unit test: construct CoinSpend with matching and mismatched puzzle_hash, verify tree_hash comparison. Negative test: tampered puzzle_reveal returns PuzzleHashMismatch. |
| EXE-003 | gap | CLVM Execution via dig-clvm | Unit test: mock dig_clvm::validate_spend_bundle, verify it is called for each SpendBundle. Negative test: ValidationError maps to correct BlockError. Integration test: no direct chia-consensus imports in dig-block. |
| EXE-004 | gap | Condition Parsing and Assertion Checking | Unit test: verify two-pass condition processing with known CLVM outputs. Test each assertion type individually. Verify height/time assertions and ASSERT_EPHEMERAL are deferred (present in pending_assertions). |
| EXE-005 | gap | BLS Aggregate Signature Verification | Unit test: valid aggregate signature passes through dig-clvm. Invalid signature returns SignatureFailed. Test all AGG_SIG variants. Optional: benchmark with and without BlsCache. |
| EXE-006 | gap | Coin Conservation and Fee Consistency | Unit test: block with correct total_fees passes; mismatched total_fees returns FeesMismatch. Verify per-bundle conservation delegated to dig-clvm. Test reserve fee failure propagation. |
| EXE-007 | gap | Cost Consistency Verification | Unit test: block with correct total_cost passes; mismatched total_cost returns CostMismatch. Verify cost is sum of all SpendResult.conditions.cost values. |
| EXE-008 | gap | ExecutionResult Output Type | Unit test: verify ExecutionResult contains all required fields after successful validation. Verify additions, removals, pending_assertions, total_cost, total_fees, receipts are correctly populated. |
| EXE-009 | gap | PendingAssertion Type Definition | Unit test: verify AssertionKind enum has 8 variants. Verify PendingAssertion struct has kind and coin_id fields. Verify from_condition() maps each height/time Condition correctly and returns None for others. Serialization round-trip test. |
