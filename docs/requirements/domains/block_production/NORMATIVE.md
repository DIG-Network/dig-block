# Block Production - Normative Requirements

| Field   | Value              |
|---------|--------------------|
| Domain  | block_production   |
| Prefix  | BLD                |
| Version | 1.0                |
| Date    | 2026-04-14         |

## Requirements

### BLD-001: BlockBuilder Struct and Constructor

`BlockBuilder` MUST contain the following fields:

- `height`: `u64`
- `epoch`: `u64`
- `parent_hash`: `Bytes32`
- `l1_height`: `u32`
- `l1_hash`: `Bytes32`
- `proposer_index`: `u32`
- `spend_bundles`: `Vec<SpendBundle>`
- `slash_proposal_payloads`: `Vec<Vec<u8>>`
- `total_cost`: `Cost`
- `total_fees`: `u64`
- `additions`: `Vec<Coin>`
- `removals`: `Vec<CoinId>` (Rust surface: `Vec<Bytes32>` — coin id bytes; see BLD-001 spec notes)

`new()` MUST initialize with empty collections and zero totals.

**Spec reference:** SPEC Section 6.1, 6.2

---

### BLD-002: add_spend_bundle with Budget Enforcement

`add_spend_bundle(bundle, cost, fee)` MUST enforce the following:

- MUST reject with `BuilderError::CostBudgetExceeded` if `total_cost + cost > MAX_COST_PER_BLOCK`.
- MUST reject with `BuilderError::SizeBudgetExceeded` if estimated size exceeds `MAX_BLOCK_SIZE`.
- MUST extract additions and removals from the bundle.
- MUST update running `total_cost` and `total_fees` totals.
- MUST append the bundle to `spend_bundles`.

`remaining_cost()` MUST return `MAX_COST_PER_BLOCK.saturating_sub(total_cost)` (never underflows if `total_cost` is already at the cap).

`spend_bundle_count()` MUST return the number of SpendBundles currently added to the builder.

**Spec reference:** SPEC Section 6.3

---

### BLD-003: add_slash_proposal with Limits

`add_slash_proposal(payload)` MUST enforce the following:

- MUST reject with `BuilderError::TooManySlashProposals` if the count would exceed `MAX_SLASH_PROPOSALS_PER_BLOCK`.
- MUST reject with `BuilderError::SlashProposalTooLarge` if `payload.len() > MAX_SLASH_PROPOSAL_PAYLOAD_BYTES`.

**Spec reference:** SPEC Section 6.3

---

### BLD-004: Optional Setters

`BlockBuilder` MUST provide the following optional setter methods:

- `set_l1_proofs(collateral, reserve, prev_finalizer, curr_finalizer, network_coin)` to set L1 proof anchors.
- `set_dfsp_roots(5 Bytes32)` to set DFSP roots.
- `set_extension_data(Bytes32)` to set the extension data field.

**Spec reference:** SPEC Section 6.3

---

### BLD-005: Build Pipeline

`build(state_root, receipts_root, signer)` MUST perform the following steps:

1. Compute `spends_root` via `MerkleTree`.
2. Compute `additions_root` via `compute_merkle_set_root` (grouped by `puzzle_hash`).
3. Compute `removals_root` via `compute_merkle_set_root`.
4. Compute `filter_hash` via BIP158.
5. Compute `slash_proposals_root`.
6. Count all items.
7. Auto-detect version from height.
8. Set timestamp to current wall-clock time.
9. Compute `block_size` (two-pass: assemble with 0, measure, update).
10. Sign header hash via `BlockSigner`.
11. Return `L2Block`.

MUST reject with `BuilderError::MissingDfspRoots` if DFSP roots are required but not set.

**Spec reference:** SPEC Section 6.4

---

### BLD-006: BlockSigner Trait Integration

`build()` MUST call `signer.sign_block(&header_hash)` to produce `proposer_signature`.

- MUST map signing failure to `BuilderError::SigningFailed`.
- The `BlockSigner` trait returns `Result<Signature, SignerError>`.

**Spec reference:** SPEC Section 6.4 Step 6

---

### BLD-007: Builder Structural Validity Guarantee

Any `L2Block` **successfully returned** from `BlockBuilder::build()` / `build_with_dfsp_activation()` (i.e. `Ok(block)` — at least one spend bundle per BLD-005) MUST pass `L2Block::validate_structure()` without error.

The builder computes all derived fields correctly by construction. This is the "build correct, validate everything" design principle.

**Spec reference:** SPEC Section 1.1 (Design Principles), Section 12.2
