# Execution Validation - Normative Requirements

- **Domain:** execution_validation
- **Prefix:** EXE
- **Spec:** DIG L2 Block Specification
- **Total Requirements:** 9

## Requirements

### EXE-001: validate_execution API

L2Block **MUST** provide `validate_execution(&self, clvm_config: &ValidationConfig, genesis_challenge: &Bytes32) -> Result<ExecutionResult, BlockError>`.

`ValidationConfig` is sourced from `dig-clvm`. Internally, the method **MUST** create a `clvmr::Allocator` and process each `SpendBundle` in block order, delegating CLVM execution and condition validation to `dig-clvm`.

**Spec reference:** SPEC Section 7.4

---

### EXE-002: Puzzle Hash Verification

For each `CoinSpend`, `tree_hash(puzzle_reveal)` **MUST** equal `coin.puzzle_hash`.

The implementation **MUST** use `clvm-utils::tree_hash()` for hashing. If the hashes do not match, the validator **MUST** reject the block with `BlockError::PuzzleHashMismatch`.

Chia parity: `block_body_validation.py` Check 20.

**Spec reference:** SPEC Section 7.4.2

---

### EXE-003: CLVM Execution via dig-clvm

Each `SpendBundle` **MUST** be validated by calling `dig_clvm::validate_spend_bundle()`. This function handles CLVM execution via `chia-consensus::run_spendbundle()`, condition parsing, and cost limit enforcement.

dig-block **MUST NOT** call `chia-consensus` directly. The implementation **MUST** map `dig-clvm` `ValidationError` variants to appropriate `BlockError` variants.

**Spec reference:** SPEC Section 7.4.3

---

### EXE-004: Condition Parsing and Assertion Checking

Conditions from CLVM output **MUST** be validated in two passes:

**Pass 1** collects:
- `CREATE_COIN` outputs
- `CREATE_COIN_ANNOUNCEMENT`
- `CREATE_PUZZLE_ANNOUNCEMENT`
- `RESERVE_FEE`

**Pass 2** validates:
- `ASSERT_COIN_ANNOUNCEMENT`
- `ASSERT_PUZZLE_ANNOUNCEMENT`
- `ASSERT_CONCURRENT_SPEND`
- `ASSERT_CONCURRENT_PUZZLE`
- `ASSERT_MY_COIN_ID`
- `ASSERT_MY_PARENT_ID`
- `ASSERT_MY_PUZZLEHASH`
- `ASSERT_MY_AMOUNT`

Height/time assertions **MUST** be collected but deferred to Tier 3. `ASSERT_EPHEMERAL` **MUST** also be collected and deferred to Tier 3 (coin must be created in the same block). The implementation **MUST** use `chia-sdk-types::Condition` directly.

**Spec reference:** SPEC Section 7.4.4

---

### EXE-005: BLS Aggregate Signature Verification

BLS signature verification **MUST** be handled inside `dig_clvm::validate_spend_bundle()`. dig-block **MUST NOT** perform separate signature verification.

The implementation **MUST** support all `AGG_SIG` variants. An optional `BlsCache` **MAY** be used for performance. If verification fails, the validator **MUST** reject with `BlockError::SignatureFailed`.

Chia parity: `block_body_validation.py` Check 22.

**Spec reference:** SPEC Section 7.4.5

---

### EXE-006: Coin Conservation and Fee Consistency

Per-bundle conservation **MUST** be checked inside `dig-clvm` (`total_input >= total_output`).

At the block level, `computed_total_fees` **MUST** equal `header.total_fees`. If they do not match, the validator **MUST** reject with `BlockError::FeesMismatch`.

Reserve fee **MUST** be checked by `dig-clvm` internally (`ReserveFeeFailed`).

Chia parity: Check 16, Check 19.

**Spec reference:** SPEC Section 7.4.6

---

### EXE-007: Cost Consistency Verification

`computed_total_cost` (the sum of all `SpendResult.conditions.cost`) **MUST** equal `header.total_cost`. If they do not match, the validator **MUST** reject with `BlockError::CostMismatch`.

Chia parity: Check 9.

**Spec reference:** SPEC Section 7.4.6

---

### EXE-008: ExecutionResult Output Type

`ExecutionResult` **MUST** contain the following fields:

- `additions: Vec<Coin>`
- `removals: Vec<Bytes32>`
- `pending_assertions: Vec<PendingAssertion>`
- `total_cost: Cost`
- `total_fees: u64`
- `receipts: Vec<Receipt>`

This type carries validated outputs from Tier 2 to Tier 3.

**Spec reference:** SPEC Section 7.4.7

---

### EXE-009: PendingAssertion Type Definition

`PendingAssertion` **MUST** be defined as a struct containing:

- `kind: AssertionKind` — an enum with 8 variants matching the height/time lock condition opcodes (`HeightAbsolute`, `HeightRelative`, `SecondsAbsolute`, `SecondsRelative`, `BeforeHeightAbsolute`, `BeforeHeightRelative`, `BeforeSecondsAbsolute`, `BeforeSecondsRelative`)
- `coin_id: Bytes32` — the coin ID that produced the assertion (needed for relative lookups)

A `from_condition(condition, coin_spend)` factory method **MUST** map `chia-sdk-types::Condition` height/time variants to `AssertionKind` and return `None` for non-height/time conditions.

Both `PendingAssertion` and `AssertionKind` **MUST** derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`.

**Spec reference:** SPEC Sections 7.4.4, 7.4.7, 7.5.4
