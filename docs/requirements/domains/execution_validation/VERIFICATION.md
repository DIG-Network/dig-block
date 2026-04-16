# Execution Validation - Verification Matrix

- **Domain:** execution_validation
- **Prefix:** EXE
- **Spec:** DIG L2 Block Specification
- **Total Requirements:** 9

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| EXE-001 | implemented | validate_execution API | Integration: `tests/test_exe_001_validate_execution_api.rs` — 6 tests: empty block returns empty `ExecutionResult`; `fn(&L2Block, &ValidationConfig, &Bytes32)` signature matches NORMATIVE (compile-time function-pointer coercion); non-zero `header.total_fees` on empty body → `FeesMismatch` (EXE-006); non-zero `header.total_cost` → `CostMismatch` (EXE-007); determinism; accepts `dig_clvm::ValidationConfig::default()`. Full CLVM pipeline scope belongs to EXE-003. |
| EXE-002 | gap | Puzzle Hash Verification | Unit test: construct CoinSpend with matching and mismatched puzzle_hash, verify tree_hash comparison. Negative test: tampered puzzle_reveal returns PuzzleHashMismatch. |
| EXE-003 | gap | CLVM Execution via dig-clvm | Unit test: mock dig_clvm::validate_spend_bundle, verify it is called for each SpendBundle. Negative test: ValidationError maps to correct BlockError. Integration test: no direct chia-consensus imports in dig-block. |
| EXE-004 | gap | Condition Parsing and Assertion Checking | Unit test: verify two-pass condition processing with known CLVM outputs. Test each assertion type individually. Verify height/time assertions and ASSERT_EPHEMERAL are deferred (present in pending_assertions). |
| EXE-005 | gap | BLS Aggregate Signature Verification | Unit test: valid aggregate signature passes through dig-clvm. Invalid signature returns SignatureFailed. Test all AGG_SIG variants. Optional: benchmark with and without BlsCache. |
| EXE-006 | gap | Coin Conservation and Fee Consistency | Unit test: block with correct total_fees passes; mismatched total_fees returns FeesMismatch. Verify per-bundle conservation delegated to dig-clvm. Test reserve fee failure propagation. |
| EXE-007 | gap | Cost Consistency Verification | Unit test: block with correct total_cost passes; mismatched total_cost returns CostMismatch. Verify cost is sum of all SpendResult.conditions.cost values. |
| EXE-008 | implemented | ExecutionResult Output Type | Integration: `tests/test_exe_008_execution_result.rs` — 9 tests covering `Default` emptiness, all-fields-populated struct-literal construction (proves field visibility + type), additions as concrete `Coin`, removals as `Bytes32`, `PendingAssertion` passthrough, one-receipt-per-bundle, scalar totals, bincode round-trip, and `&ExecutionResult` borrow (simulates STV-001 handoff). |
| EXE-009 | implemented | PendingAssertion Type Definition | Integration: `tests/test_exe_009_pending_assertion.rs` — 13 tests: one per height/time variant (4 `ASSERT_*`, 4 `ASSERT_BEFORE_*`) with coin_id capture; `None` for `CREATE_COIN` and `ASSERT_EPHEMERAL`; bincode round-trip for struct and every `AssertionKind` variant; integration with `ExecutionResult::pending_assertions` (EXE-008). |
