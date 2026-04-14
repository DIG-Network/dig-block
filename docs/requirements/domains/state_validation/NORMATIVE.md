# State Validation - Normative Requirements

- **Domain:** state_validation
- **Prefix:** STV
- **Spec:** DIG L2 Block Specification
- **Total Requirements:** 7

## Requirements

### STV-001: validate_state API

L2Block **MUST** provide `validate_state(&self, exec: &ExecutionResult, coins: &dyn CoinLookup, proposer_pubkey: &PublicKey) -> Result<Bytes32, BlockError>`.

Returns the computed `state_root` on success. Additionally, `validate_full()` **MUST** be provided combining all 3 tiers of validation (structural, execution, state) into a single call.

**Spec reference:** SPEC Section 7.5

---

### STV-002: Coin Existence Checks

For each removal: `coins.get_coin_state(coin_id)` **MUST** return `Some`. If `None`, the validator **MUST** check if the coin is ephemeral (present in `exec.additions`). If neither found, reject with `BlockError::CoinNotFound`.

If `spent_height.is_some()`, the validator **MUST** reject with `BlockError::CoinAlreadySpent`.

Chia parity: Check 15.

**Spec reference:** SPEC Section 7.5.1

---

### STV-003: Puzzle Hash Cross-Check

For each removal, `coin_state.coin.puzzle_hash` **MUST** equal `coin_spend.coin.puzzle_hash`. If they do not match, reject with `BlockError::PuzzleHashMismatch`.

Uses `chia-protocol::CoinState.coin` field.

Chia parity: Check 20.

**Spec reference:** SPEC Section 7.5.2

---

### STV-004: Addition Non-Existence

For each addition, `coins.get_coin_state(addition.coin_id())` **MUST** return `None` (unless the coin is ephemeral and spent in the same block). If a coin already exists, reject with `BlockError::CoinAlreadyExists`.

**Spec reference:** SPEC Section 7.5.3

---

### STV-005: Height/Time Lock Evaluation

Pending assertions from Tier 2 **MUST** be evaluated against chain context:

- `ASSERT_HEIGHT_ABSOLUTE(h)`: `chain_height >= h`
- `ASSERT_HEIGHT_RELATIVE(h)`: `chain_height >= confirmed_height + h`
- `ASSERT_SECONDS_ABSOLUTE(t)`: `chain_timestamp >= t`
- `ASSERT_SECONDS_RELATIVE(t)`: `chain_timestamp >= coin_timestamp + t`
- `BEFORE_HEIGHT_ABSOLUTE(h)`: `chain_height < h`
- `BEFORE_HEIGHT_RELATIVE(h)`: `chain_height < confirmed_height + h`
- `BEFORE_SECONDS_ABSOLUTE(t)`: `chain_timestamp < t`
- `BEFORE_SECONDS_RELATIVE(t)`: `chain_timestamp < coin_timestamp + t`

If any assertion fails, reject with `BlockError::AssertionFailed`.

Chia parity: Check 21.

**Spec reference:** SPEC Section 7.5.4

---

### STV-006: Proposer Signature Verification

`verify(proposer_pubkey, header.hash(), block.proposer_signature)` **MUST** pass. Uses `chia-bls::verify`. If verification fails, reject with `BlockError::InvalidProposerSignature`.

**Spec reference:** SPEC Section 7.5.5

---

### STV-007: State Root Verification

After applying all additions and removals: start from the parent state, mark removals as spent, insert additions, and recompute the Merkle root. `computed_state_root` **MUST** equal `header.state_root`. If they do not match, reject with `BlockError::InvalidStateRoot`.

On success, return the computed state root.

**Spec reference:** SPEC Section 7.5.6
