# dig-block

[![Crate](https://img.shields.io/badge/crate-dig--block-blue)](https://github.com/DIG-Network/dig-block)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

**`dig-block`** is the DIG Network L2 block format, production, and validation library. It
owns the canonical on-wire shape of every L2 block, the builder pipeline that assembles blocks
from spend bundles, and the three-tier validation pipeline (structural → execution → state)
that rejects any block that does not match consensus rules.

## Scope

This crate is **single-block scoped**: every public function operates on one block (or
one checkpoint) in isolation. External state (coin set, chain tip, wall clock, validator set)
is injected through two traits:

- `CoinLookup` — coin-set queries for Tier 3 state validation.
- `BlockSigner` — proposer signing hook for block production.

`dig-block` never reads from disk, never makes network calls, and never maintains state across
blocks. Downstream crates (`dig-coinstore`, `dig-epoch`, `dig-gossip`) supply the trait
implementations, storage, chain management, and networking.

## Install

```toml
[dependencies]
dig-block = "0.1"
```

Rust edition 2021; minimum supported Rust version **1.75**.

## Public API at a glance

| Concern | Key items |
|---------|-----------|
| **Block types** | `L2BlockHeader`, `L2Block`, `AttestedBlock`, `Checkpoint`, `CheckpointSubmission`, `Receipt`, `ReceiptList`, `SignerBitmap`, `BlockStatus`, `CheckpointStatus` |
| **Block production** | `BlockBuilder`, `CheckpointBuilder` |
| **Validation** | `L2Block::validate_structure`, `L2Block::validate_execution`, `L2Block::validate_state`, `L2Block::validate_full` |
| **Validation output** | `ExecutionResult`, `PendingAssertion`, `AssertionKind` |
| **Integration traits** | `CoinLookup`, `BlockSigner` |
| **Hashing** | `hash_leaf`, `hash_node`, `compute_spends_root`, `compute_additions_root`, `compute_removals_root`, `compute_filter_hash`, `compute_receipts_root`, `compute_state_root_from_delta` |
| **Errors** | `BlockError`, `CheckpointError`, `BuilderError`, `SignerBitmapError`, `ReceiptError` |
| **Constants** | `EMPTY_ROOT`, `ZERO_HASH`, `MAX_BLOCK_SIZE`, `MAX_COST_PER_BLOCK`, `MAX_SLASH_PROPOSALS_PER_BLOCK`, `MAX_SLASH_PROPOSAL_PAYLOAD_BYTES`, `DFSP_ACTIVATION_HEIGHT`, `MAX_FUTURE_TIMESTAMP_SECONDS`, `HASH_LEAF_PREFIX`, `HASH_TREE_PREFIX`, `VERSION_V1`, `VERSION_V2`, `MAX_VALIDATORS` |
| **Primitives** | `Bytes32`, `Cost`, `Signature`, `PublicKey` (re-exported from `chia-protocol` / `chia-bls`) |

A convenience glob import is provided:

```rust
use dig_block::prelude::*;
```

## Quickstart — build a block

```rust,no_run
use dig_block::prelude::*;
use dig_block::traits::SignerError;

// 1. Implement BlockSigner for your key backend.
struct MySigner;
impl BlockSigner for MySigner {
    fn sign_block(&self, _header_hash: &Bytes32) -> Result<Signature, SignerError> {
        Ok(Signature::default())
    }
}

// 2. Build an anchored builder.
let parent = Bytes32::default();
let l1_hash = Bytes32::default();
let mut builder = BlockBuilder::new(
    /*height=*/ 1, /*epoch=*/ 0, parent,
    /*l1_height=*/ 100, l1_hash, /*proposer_index=*/ 0,
);

// 3. (Optional) add spend bundles / slash proposals / set L1 proofs etc.
//    builder.add_spend_bundle(bundle, cost, fee)?;

// 4. Finalize and sign.
let state_root = Bytes32::default();
let receipts_root = Bytes32::default();
let block = builder.build(state_root, receipts_root, &MySigner)
    .expect("well-formed builder output");
```

The output is **structurally valid by construction**: `block.validate_structure()` always passes
on anything `BlockBuilder::build` produces (BLD-007).

## Quickstart — validate a block

```rust,no_run
use dig_block::prelude::*;
use chia_protocol::CoinState;
use dig_clvm::ValidationConfig;

struct MyCoinLookup;
impl CoinLookup for MyCoinLookup {
    fn get_coin_state(&self, _id: &Bytes32) -> Option<CoinState> { None }
    fn get_chain_height(&self) -> u64 { 0 }
    fn get_chain_timestamp(&self) -> u64 { 0 }
}

fn validate(block: &L2Block, pk: &PublicKey) -> Result<Bytes32, BlockError> {
    // Runs Tier 1 → Tier 2 → Tier 3 with short-circuit on failure.
    // Returns the computed state root on success.
    block.validate_full(
        &ValidationConfig::default(),
        &Bytes32::default(),       // genesis challenge
        &MyCoinLookup,
        pk,
    )
}
```

## Validation pipeline

```text
┌─────────────────────────────────────────────────────────────────────┐
│  L2Block                                                             │
│    │                                                                 │
│    ▼                                                                 │
│  Tier 1: L2Block::validate_structure()                               │
│    • no external state                                               │
│    • SVL-001..006: version, DFSP roots, cost/size, timestamp,        │
│      count agreement, Merkle roots, duplicates                       │
│    │                                                                 │
│    ▼                                                                 │
│  Tier 2: L2Block::validate_execution(config, genesis_challenge)      │
│    • dig_clvm::validate_spend_bundle per SpendBundle                 │
│    • EXE-002..007: puzzle hash, CLVM, conditions, BLS, conservation, │
│      cost                                                            │
│    • Produces ExecutionResult → PendingAssertion vec                 │
│    │                                                                 │
│    ▼                                                                 │
│  Tier 3: L2Block::validate_state(exec, coins, pubkey)                │
│    • CoinLookup for persistent coin state                            │
│    • STV-002..007: coin existence, puzzle-hash cross-check,          │
│      addition uniqueness, height/time locks, proposer signature,     │
│      state root recompute                                            │
│    │                                                                 │
│    ▼                                                                 │
│  Ok(computed_state_root) / Err(BlockError)                           │
└─────────────────────────────────────────────────────────────────────┘
```

Each tier can also be invoked independently, e.g. for light clients that only need structural
verification, or for validators that want to cache `ExecutionResult` for replay.

## Dependencies

`dig-block` reuses the Chia Rust ecosystem and does not reimplement CLVM, BLS, or Merkle primitives:

| Concern | Crate |
|---------|-------|
| Core protocol types (`Bytes32`, `Coin`, `SpendBundle`, `CoinSpend`, `CoinState`) | `chia-protocol` |
| BLS12-381 signatures (`Signature`, `PublicKey`, `verify`) | `chia-bls` |
| CLVM execution + condition parsing | `dig-clvm` (wraps `chia-consensus`) |
| Merkle set roots (additions, removals) | `chia-consensus` |
| Binary Merkle trees (spends, receipts, slash proposals) | `chia-sdk-types` |
| SHA-256 | `chia-sha2` |
| CLVM tree hashing | `clvm-utils` |
| Bincode serialization | `bincode` + `serde` |
| BIP-158 compact block filter | `bitcoin::bip158` |

CLVM execution is **always** routed through `dig_clvm::validate_spend_bundle`; dig-block never
calls `chia-consensus::run_spendbundle` directly. This architectural boundary is enforced by a
grep-based lint in `tests/test_exe_003_clvm_delegation.rs`.

## Testing

74 normative requirements; one dedicated integration test file per requirement. The full suite:

```bash
cargo test --release
```

Runs ~600 tests against the public API (no unit tests reach into private modules). Property-based
coverage via `proptest` in `tests/test_ser_005_roundtrip_integrity.rs`.

## Specification

All behavior is derived from the authoritative crate specification:
[`docs/resources/SPEC.md`](docs/resources/SPEC.md).

Each requirement has a three-document trace:

- `docs/requirements/domains/{domain}/NORMATIVE.md` — authoritative MUST/SHOULD statements
- `docs/requirements/domains/{domain}/specs/{PREFIX-NNN}.md` — detailed spec + test plan
- `docs/requirements/domains/{domain}/VERIFICATION.md` / `TRACKING.yaml` — status + test refs

## License

MIT. See [`LICENSE`](LICENSE).

## Versioning

Semantic versioning. Wire-format changes to the block header bump the minor version pre-1.0 and
the major version post-1.0. Adding new protocol versions (`VERSION_V3`, etc.) follows the same
rule.
