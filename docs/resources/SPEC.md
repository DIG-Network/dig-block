# dig-block Specification

**Version:** 0.1.0
**Status:** Draft
**Date:** 2026-04-14

## 1. Overview

`dig-block` is a self-contained Rust crate that owns three concerns for the DIG Network L2 blockchain: **defining the block format**, **building blocks**, and **validating blocks**. It covers both **regular blocks** (L2 transaction blocks) and **checkpoint blocks** (epoch summaries submitted to L1 for hard finality). The crate provides everything needed to construct a valid block from its inputs, verify that a received block is correct, and interact with its contents — all scoped to a single, isolated block.

The crate **does** own:
- **Block format** — Type definitions for `L2BlockHeader`, `L2Block`, `AttestedBlock`, `Checkpoint`, `CheckpointSubmission`, and all supporting types.
- **Block production** — `BlockBuilder` that accumulates SpendBundles, computes all derived header fields (Merkle roots, counts, costs, fees, filter hash), and emits a signed `L2Block`. The builder enforces limits during construction so the produced block is structurally valid by design.
- **Block validation** — The full validation pipeline for a received block:
  - *Structural validation* — header constraints, Merkle root recomputation, size/cost limits, duplicate detection.
  - *Execution validation* — CLVM puzzle execution per CoinSpend, condition parsing and assertion checking, announcement resolution, coin property assertions, height/time lock evaluation.
  - *Signature validation* — BLS aggregate signature verification per SpendBundle (AGG_SIG_ME, AGG_SIG_UNSAFE, etc.) and proposer signature over the header.
  - *Conservation validation* — coin value conservation (no minting), fee consistency.
  - *State root validation* — verifying the header's `state_root` matches the computed state after applying all additions/removals.
- **Block hashing** — deterministic SHA-256 hash computation for both regular blocks and checkpoints.
- **Serialization** — `to_bytes()` / `from_bytes()` via bincode for all block types.
- **Helper functions** — computing Merkle roots, extracting additions/removals, BIP158 filter construction.
- **Supporting types** — `BlockStatus`, `CheckpointStatus`, `Cost`, `Receipt`, `ReceiptList`, `SignerBitmap`, error enums.
- **Protocol constants** — `MAX_BLOCK_SIZE`, `MAX_COST_PER_BLOCK`, etc.

The crate does **not** own:
- **Block storage** (persisting blocks to disk, indexing blocks by height/hash) — handled by the chain store layer.
- **Chain management** (maintaining the canonical chain, fork choice, parent-hash continuity across blocks) — handled by the chain manager.
- **Global state management** (maintaining the CoinSet database, rollback, queries) — handled by `dig-coinstore`. However, this crate *does* compute the state root delta for a single block and accepts coin lookup via a trait.
- **Transaction selection** (choosing which SpendBundles from the mempool to include, fee prioritization) — handled by the mempool/proposer layer. The `BlockBuilder` accepts whatever SpendBundles are given to it.
- **Consensus** (validator set management, attestation pooling, checkpoint competition, finality tracking) — handled by the consensus layer.
- **Networking** (block gossip, peer sync, checkpoint relay) — handled by the networking layer.

**Hard boundary:** The crate operates on a **single, isolated block**. External state required for validation (coin existence, chain tip height, current timestamp) is injected through traits, not stored. The crate never reads from a database, never makes network calls, and never maintains state across blocks.

### 1.1 Design Principles

- **Single-block scope**: Every function operates on one block in isolation. External state (coin existence, chain height) is injected via traits, never stored. The crate maintains no state across blocks.
- **Build correct, validate everything**: The `BlockBuilder` computes all derived fields so the output is structurally valid by construction. The `BlockValidator` re-derives everything from scratch and rejects any mismatch — it trusts nothing from the header.
- **Deterministic hashing**: Given identical field values, `hash()` always produces the same `Bytes32`. The hash covers every header field in a fixed, documented order using little-endian encoding. Optional fields hash as `ZERO_HASH` when `None`.
- **Layered validation**: Validation is split into tiers — structural (no external state), execution (requires CLVM runner), and state (requires coin lookup). Callers can stop at any tier. Each tier is a strict superset of the previous.
- **Two block types, one crate**: Regular blocks and checkpoint blocks share a crate because they share foundational types (`Bytes32`, `Signature`, `SignerBitmap`, `EMPTY_ROOT`) and because downstream crates (chain store, consensus, sync) need both. They remain separate type hierarchies with no inheritance relationship.
- **Chia type compatibility**: The block format uses Chia ecosystem types (`Bytes32`, `Signature`, `PublicKey`, `SpendBundle`, `CoinSpend`) from `chia-protocol` and `chia-bls`. This ensures wire-level interoperability with Chia tooling.
- **Existing crates, not custom abstractions**: CLVM execution uses `dig-clvm` (the same engine as `dig-mempool`). Condition types, Merkle set computation, and SHA-256 come from Chia crates used as-is. Only coin state lookup is injected via a trait (`CoinLookup`) because the storage backend varies by deployment.
- **Versioned headers**: Block headers carry a `version` field that tracks protocol upgrades. Version 1 (pre-DFSP) and Version 2 (post-DFSP activation) are defined, with the version automatically determined by block height relative to the DFSP activation height.
- **Zero-copy deserialization path**: Serialization uses bincode for compact binary encoding. Block types derive `Serialize`/`Deserialize` for direct conversion.

### 1.2 Crate Dependencies

The crate maximally reuses the Chia Rust ecosystem to avoid reimplementing production-hardened primitives. The principle is: **if a Chia crate already provides it, use it — don't rewrite it.**

| Crate | Version | Purpose |
|-------|---------|---------|
| `chia-protocol` | 0.26 | Core protocol types: `Bytes32`, `SpendBundle`, `CoinSpend`, `Coin`, `CoinState`, `Program`. SpendBundle is the atomic transaction unit in the block body. `CoinState` is used for lightweight coin lookups during validation. |
| `chia-bls` | 0.26 | BLS12-381 cryptography: `Signature`, `PublicKey`, `SecretKey`. Functions: `sign()`, `verify()`, `aggregate_verify()`. Used for proposer signatures, per-SpendBundle aggregate signature verification, and checkpoint aggregate signatures. |
| `dig-clvm` | 0.1 | **DIG L2 CLVM consensus engine.** Thin orchestration layer over `chia-consensus`. Provides `validate_spend_bundle()` (full CLVM execution + BLS signature verification + conservation check), `ValidationContext` (coin records, chain height/timestamp, network constants), `ValidationConfig` (cost limits, mempool mode flags), `SpendResult` (fee, additions, removals, conditions). Also re-exports `chia-consensus` types. The single integration point for all CLVM execution in the DIG ecosystem — `dig-mempool` already depends on it. |
| `chia-consensus` | 0.26 | **Block-level validation primitives** (used directly for Merkle roots, used via `dig-clvm` for CLVM execution). `compute_merkle_set_root()` — computes additions/removals Merkle roots identically to Chia L1. `make_aggsig_final_message()` — constructs AGG_SIG message bytes. `OwnedSpendBundleConditions` — parsed condition output from CLVM execution. `opcodes` — canonical condition opcode constants. |
| `chia-sdk-types` | 0.30 | **High-level type abstractions.** `Condition` enum (43 variants covering all CLVM condition opcodes). `MerkleTree` / `MerkleProof` — Merkle tree construction and proof generation. `MAINNET_CONSTANTS` / `TESTNET11_CONSTANTS` — network consensus constants. |
| `chia-sdk-signer` | 0.30 | **Signature requirement extraction.** `RequiredSignature::from_coin_spend()` — extracts all AGG_SIG requirements from a CoinSpend's conditions and constructs the correct message bytes. `AggSigConstants` — domain separation constants for each AGG_SIG variant. Eliminates manual message construction. |
| `chia-sha2` | 0.26 | SHA-256 implementation (`Sha256` hasher). Used for all hashing: block header hash, checkpoint hash, Merkle leaf/node hashing, coin ID computation. Ensures hash compatibility with the Chia ecosystem. Same implementation used internally by `Coin::coin_id()`. |
| `chia-traits` | 0.26 | `Streamable` trait for Chia-canonical binary serialization. Used for wire-format serialization of protocol types (`CoinState`, `SpendBundle`). Internal block storage uses bincode; `Streamable` is used for cross-crate protocol interop. |
| `clvmr` | 0.14 | CLVM runtime (transitive via `dig-clvm`): `Allocator` (memory management), `ChiaDialect` (Chia's CLVM operator set). The low-level engine behind `dig-clvm::validate_spend_bundle()`. |
| `clvm-utils` | 0.26 | `tree_hash()`, `curry_tree_hash()`, `TreeHash`. Used for puzzle hash computation and verification (`sha256tree` of puzzle reveal must match coin's `puzzle_hash`). |
| `bincode` | — | Compact binary serialization for block types (`to_bytes()` / `from_bytes()`). Used for internal storage format. |
| `serde` | — | Serialization/deserialization framework. All block types derive `Serialize` + `Deserialize`. |
| `thiserror` | — | Error type derivation for `BlockError`, `CheckpointError`, `BuilderError`. |

**Key types used from the Chia ecosystem:**

| Type | From Crate | Usage in dig-block |
|------|-----------|-------------------|
| `Bytes32` | chia-protocol | Hashes, coin IDs, Merkle roots, block hashes — everywhere. |
| `Coin` | chia-protocol | Coin identity (`parent_coin_info`, `puzzle_hash`, `amount`). `Coin::coin_id()` computes `sha256(parent \|\| puzzle_hash \|\| amount)`. |
| `CoinSpend` | chia-protocol | Coin + puzzle_reveal + solution. The atomic unit inside a SpendBundle. |
| `SpendBundle` | chia-protocol | `coin_spends: Vec<CoinSpend>` + `aggregated_signature: Signature`. Block body content. |
| `CoinState` | chia-protocol | Lightweight coin state (`coin`, `created_height`, `spent_height`) for validation lookups. |
| `Program` | chia-protocol | Serialized CLVM program. Used for puzzle_reveal and solution in CoinSpend. |
| `Condition` | chia-sdk-types | Full enum of 43 CLVM condition opcodes (CREATE_COIN, AGG_SIG_ME, ASSERT_HEIGHT_*, etc.). Used directly in validation — no custom condition types. |
| `RequiredSignature` | chia-sdk-signer | Extracted signature requirement from a CoinSpend. Handles all AGG_SIG variants and message construction. |
| `AggSigConstants` | chia-sdk-signer | Domain separation constants for AGG_SIG message construction. |
| `ValidationContext` | dig-clvm | Chain state for CLVM validation: coin records, height, timestamp, network constants. |
| `ValidationConfig` | dig-clvm | CLVM execution config: cost limits, mempool mode flags, signature validation flags. |
| `SpendResult` | dig-clvm | Result of validating a SpendBundle: fee, additions, removals, parsed conditions. |
| `OwnedSpendBundleConditions` | chia-consensus (via dig-clvm) | Parsed CLVM output: cost, fees, per-spend conditions, aggregate signature pairs. |
| `Sha256` | chia-sha2 | SHA-256 hasher for all hashing operations. |
| `Allocator` | clvmr (via dig-clvm) | CLVM memory allocator, required for puzzle execution. |

### 1.3 Design Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Block hash covers all header fields | Every header field participates in the hash to prevent malleability. Optional fields (`l1_collateral_coin_id`, etc.) hash as `ZERO_HASH` (`0x0000...0000`) when `None`. This is simpler than excluding optional fields and ensures any two headers with different field values produce different hashes. |
| 2 | Bincode serialization (not Streamable) | Bincode is used for all serialization because block types include Chia BLS types (`Signature`, `PublicKey`) that don't implement Chia's `Streamable` trait. Bincode is also more compact for complex nested structures. |
| 3 | `BlockStatus` is a simple enum, not a state machine | Status transitions (Pending -> Validated -> SoftFinalized -> HardFinalized) are enforced by the consensus layer, not the block type. The block crate only defines the enum values. |
| 4 | Separate `L2BlockHeader` and `L2Block` types | The header is independently hashable and transmittable (e.g., in attestation messages). The full block includes the heavy body (SpendBundles). This separation mirrors Ethereum's header/body split and allows header-only operations without deserializing the body. |
| 5 | `AttestedBlock` wraps `L2Block` | Attestation data (signer bitmap, aggregate signature, receipts) is logically separate from the block itself. The proposer creates an `L2Block`; validators attest it, producing an `AttestedBlock`. This keeps block construction and attestation as distinct concerns. |
| 6 | `Checkpoint` is a separate type from `L2Block` | Checkpoints summarize an entire epoch (multiple blocks) and are submitted to L1. They have fundamentally different fields (block_root, tx_count, withdrawals_root) and a different lifecycle (competition, L1 submission, L1 finalization). Unifying them would add complexity without benefit. |
| 7 | DFSP roots in header with activation gating | Five DFSP Sparse Merkle Tree roots are included in the header from the start (defaulting to `EMPTY_ROOT`). Before DFSP activation height, validation rejects non-empty DFSP roots. After activation, blocks must carry correct DFSP roots. This avoids a hard fork in the header format. |
| 8 | Slash proposals as opaque payloads | Slash proposal payloads are stored as `Vec<Vec<u8>>` (bincode-encoded `SlashingEvidence`). The block crate does not parse them — it only validates count, size limits, and Merkle root. Parsing is the consensus layer's responsibility. |
| 9 | `EMPTY_ROOT` as canonical empty Merkle root | `SHA256("")` = `0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` is used as the root when a Merkle tree has no leaves. This is consistent across spends root, receipts root, slash proposals root, and all DFSP roots. |
| 10 | Version auto-detection from height | `L2BlockHeader` constructors automatically set `version` based on `height` relative to `DFSP_ACTIVATION_HEIGHT`. Callers do not manually specify the version, preventing version/height mismatches. |
| 11 | Separate `additions_root` and `removals_root` | Adopted from Chia's `FoliageTransactionBlock` ([`block_body_validation.py:158-187`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L158)). Chia commits separate Merkle roots for additions (coins created) and removals (coins spent) in every transaction block. This enables light client proofs of individual coin creation or spending without downloading the full block body. DIG adopts this pattern with the addition that `additions_root` groups coins by `puzzle_hash` for efficient wallet lookups, matching Chia's grouping strategy ([`block_body_validation.py:160-175`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L160)). |
| 12 | BIP158 transaction filter (`filter_hash`) | Adopted from Chia's `FoliageTransactionBlock.filter_hash` ([`block_creation.py:205-206`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L205), [`block_header_validation.py` Check 25](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py)). Chia includes a BIP158 compact filter encoding puzzle hashes of additions and coin IDs of removals. This lets light clients quickly determine if a block might contain relevant transactions without downloading it. DIG commits the hash of this filter in the header. |
| 13 | Tagged Merkle hashing (domain separation) | Adopted from Chia's Merkle set construction ([`merkle_utils.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/wallet/util/merkle_utils.py)). Chia prefixes leaf hashes with `0x01` and internal node hashes with `0x02` to prevent second-preimage attacks where a valid proof for a leaf could be reinterpreted as an internal node or vice versa. DIG adopts the same prefix scheme for all block-level Merkle trees. |
| 14 | `extension_data` field for future-proofing | Adopted from Chia's `FoliageBlockData.extension_data: bytes32` ([`block_creation.py:53-283`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L53)). A reserved 32-byte field in the header that can carry protocol extension data in future versions without changing the header format. Defaults to `ZERO_HASH` (`0x0000...0000`). |
| 15 | Within-block duplicate output detection | Adopted from Chia's block body validation Check 13 ([`block_body_validation.py:396`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L396), error `DUPLICATE_OUTPUT`). No coin ID may appear twice in a block's additions. This is a structural check (computable from the block alone) that catches invalid block construction early. |
| 16 | Within-block double-spend detection | Adopted from Chia's block body validation Check 14 ([`block_body_validation.py:400`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L400), error `DOUBLE_SPEND`). No coin ID may appear twice in a block's removals. Like duplicate output detection, this is computable from the block alone. |
| 17 | Coin conservation as block invariant | Adopted from Chia's block body validation Check 16 ([`block_body_validation.py:431`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L431), error `MINTING_COIN`). Total value of removals must be >= total value of additions (no coin minting). The difference is the fee. DIG validates `total_fees == total_removed_value - total_added_value` as a structural check when additions/removals are available. |
| 18 | Timestamp upper bound | Adopted from Chia's block header validation Check 26a ([`block_header_validation.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py), error `TIMESTAMP_TOO_FAR_IN_FUTURE`). Chia rejects blocks with timestamps more than 5 minutes in the future. DIG adopts a configurable `MAX_FUTURE_TIMESTAMP_SECONDS` (default 300s) for structural validation, preventing clock-skew attacks. |
| 19 | Maximal reuse of Chia and DIG crates — no custom reimplementations | If an existing crate already provides a type, function, or algorithm, dig-block uses it directly rather than reimplementing. **DIG crate:** CLVM execution via `dig-clvm::validate_spend_bundle()` (the same engine `dig-mempool` uses — not a separate CLVM wrapper). **Chia crates:** condition types from `chia-sdk-types::Condition` (not a custom enum), Merkle set roots from `chia-consensus::compute_merkle_set_root()` (not a custom Merkle tree), puzzle hashing from `clvm-utils::tree_hash()` (not a custom sha256tree), SHA-256 from `chia-sha2::Sha256` (not a generic sha2 crate), and coin state from `chia-protocol::CoinState` (not a custom CoinRecord). This ensures bit-for-bit compatibility with Chia L1, reduces maintenance burden, and inherits bug fixes and new features automatically. |

### 1.4 Chia Block Format Analysis

The DIG L2 block format draws on patterns from Chia's production L1 block format. This section documents which Chia patterns were adopted, which were adapted, and which were not applicable, with references to the Chia source code.

#### 1.4.1 Adopted Patterns

| # | Chia Pattern | Chia Source | DIG Adaptation |
|---|-------------|-------------|----------------|
| 1 | Separate additions/removals Merkle roots | [`FoliageTransactionBlock.additions_root`, `removals_root`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L158) | Header fields `additions_root` and `removals_root` with identical Merkle set construction. |
| 2 | Additions grouped by puzzle_hash | [`block_body_validation.py:160-175`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L160) — additions Merkle set contains `[puzzle_hash, hash(coin_ids)]` pairs | Same grouping for wallet-efficient lookups. |
| 3 | BIP158 transaction filter | [`FoliageTransactionBlock.filter_hash`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L205), [`block_header_validation.py` Check 25](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py) | `filter_hash` in header. Filter contains addition puzzle hashes + removal coin IDs. |
| 4 | Tagged Merkle hashing | [`merkle_utils.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/wallet/util/merkle_utils.py) — `HASH_LEAF_PREFIX = 0x01`, `HASH_TREE_PREFIX = 0x02` | Same prefix scheme for all block-level Merkle trees. |
| 5 | Extension data field | [`FoliageBlockData.extension_data: bytes32`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L53) | `extension_data: Bytes32` in header, defaults to `ZERO_HASH`. |
| 6 | Duplicate output rejection | [`block_body_validation.py` Check 13](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L396), `DUPLICATE_OUTPUT` | `BlockError::DuplicateOutput` in structural validation. |
| 7 | Within-block double-spend rejection | [`block_body_validation.py` Check 14](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L400), `DOUBLE_SPEND` | `BlockError::DoubleSpendInBlock` in structural validation. |
| 8 | Coin conservation check | [`block_body_validation.py` Check 16](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L431), `MINTING_COIN` | Fee consistency validation: `total_fees == removed_value - added_value`. |
| 9 | Timestamp future-bound | [`block_header_validation.py` Check 26a](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py), `TIMESTAMP_TOO_FAR_IN_FUTURE` | `MAX_FUTURE_TIMESTAMP_SECONDS` with configurable bound (default 300s). |
| 10 | Hash = SHA-256 of serialized struct | [`streamable.py:get_hash()`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/util/streamable.py) — `SHA256(bytes(self))` | All block types use SHA-256 for identity hashing. |
| 11 | Header/body separation | Chia's `HeaderBlock` strips the generator, keeping only metadata + filter | `L2BlockHeader` is independently hashable/transmittable; `L2Block` adds the heavy body. |
| 12 | Cost ceiling per block | [`block_body_validation.py` Check 7](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py), `BLOCK_COST_EXCEEDS_MAX`, `MAX_BLOCK_COST_CLVM` | `MAX_COST_PER_BLOCK` checked in structural validation. |
| 13 | Generator root as hash commitment | [`TransactionsInfo.generator_root`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L238) — `SHA256(generator_program)` or `ZERO_HASH` if absent | `spends_root` commits to SpendBundle hashes (DIG uses explicit bundles instead of a generator program). |
| 14 | Reward coin incorporation in block | [`TransactionsInfo.reward_claims_incorporated`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L238) | Not adopted directly — DIG's reward model differs. But the pattern of committing reward data in the block is noted for future consideration. |

#### 1.4.2 Chia Patterns Not Adopted (with rationale)

| # | Chia Pattern | Why Not Adopted |
|---|-------------|-----------------|
| 1 | VDF proofs in block header | DIG L2 uses BLS-based proof-of-stake, not proof-of-time. VDF data (`VDFInfo`, `VDFProof`, `ClassgroupElement`) is consensus-mechanism-specific and not applicable. |
| 2 | Proof of Space in block | DIG uses validator signatures, not farmer proofs. `ProofOfSpace` ([`RewardChainBlock.proof_of_space`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L470)) is Chia-consensus-specific. |
| 3 | Sub-slot / signage point structure | Chia's time-based sub-slots (`EndOfSubSlotBundle`, `ChallengeChainSubSlot`, etc.) are artifacts of the proof-of-time consensus. DIG uses epoch-based finality instead. |
| 4 | Transaction block vs non-transaction block distinction | In Chia, some blocks only carry VDF/PoSpace data without transactions ([`FullBlock.is_transaction_block()`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/types/full_block.py)). DIG L2 blocks always carry transactions (or are empty). |
| 5 | Generator program with ref_list compression | Chia compresses transactions into a CLVM generator program that references previous block generators (`transactions_generator_ref_list`) for deduplication ([`block_body_validation.py:331-362`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L331)). DIG uses explicit SpendBundles — less compact but simpler to validate and parallelize. |
| 6 | Three-chain structure (Challenge, Reward, Infused Challenge) | Chia's consensus requires three interleaved VDF chains. Not applicable to proof-of-stake. |
| 7 | Foliage / FoliageBlockData layering | Chia's multi-layer header structure (Foliage > FoliageBlockData > FoliageTransactionBlock > TransactionsInfo) is driven by the need to separate consensus data from transaction data, since non-transaction blocks exist. DIG uses a flat header since all blocks are transaction blocks. |
| 8 | Weight / difficulty tracking in header | Chia tracks cumulative `weight: uint128` (sum of difficulties) for fork choice ([`RewardChainBlock.weight`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L470)). DIG uses validator attestation count for fork choice, not accumulated difficulty. |
| 9 | Pool target / farmer reward in block | Chia blocks specify reward destinations (`pool_target`, `farmer_reward_puzzle_hash`) because farming rewards are distributed per-block. DIG handles validator rewards at the epoch level through checkpoints. |

#### 1.4.3 Improvements Over Chia L1

| # | Improvement | Description |
|---|-------------|-------------|
| 1 | Flat header instead of nested foliage | Chia requires 4 levels of nesting (Foliage → FoliageBlockData → FoliageTransactionBlock → TransactionsInfo) to compute the header hash. DIG uses a single flat header with a direct SHA-256 hash over all fields. Simpler to implement, debug, and verify. |
| 2 | Epoch-based checkpoints | Chia has no concept of epoch checkpoints with aggregate BLS signatures. DIG's `Checkpoint` + `CheckpointSubmission` types enable efficient L1 finality for batches of blocks. |
| 3 | L1 anchor proofs in header | DIG headers carry L1 coin IDs (`l1_collateral_coin_id`, `l1_network_coin_id`, etc.) that prove L1 state. Chia L1 has no equivalent cross-chain anchoring. |
| 4 | DFSP data layer roots | DIG headers commit to five Sparse Merkle Tree roots for the decentralized file storage protocol. Chia has no equivalent data availability layer. |
| 5 | Slash proposal commitments | DIG headers commit to slash proposal payloads via count + Merkle root, enabling on-chain accountability. Chia has no slashing mechanism. |
| 6 | Explicit SpendBundles instead of generator | DIG blocks carry SpendBundles directly instead of a CLVM generator program. This trades some compression for simpler validation, better parallelism, and no need for generator reference lookups across blocks. |
| 7 | Execution receipts per transaction | DIG's `ReceiptList` provides per-SpendBundle execution status, fee, and post-state root. Chia has no per-transaction receipts — success/failure is all-or-nothing at the block level. |
| 8 | Validator attestation bitmap | DIG's `AttestedBlock` tracks exactly which validators signed via a compact `SignerBitmap`. Chia's consensus doesn't have an equivalent per-block attestation record. |

## 2. Data Model

### 2.1 Primitive Types

| Type | Definition | Usage |
|------|-----------|-------|
| `Bytes32` | `[u8; 32]` (from `chia-protocol`) | Hashes, coin IDs, Merkle roots, block hashes. |
| `Signature` | BLS12-381 signature (from `chia-bls`) | Proposer block signatures, aggregate attestation signatures, checkpoint aggregate signatures. |
| `PublicKey` | BLS12-381 public key (from `chia-bls`) | Checkpoint aggregate public key. |
| `SpendBundle` | Atomic transaction (from `chia-protocol`) | Block body content. Contains `coin_spends: Vec<CoinSpend>` and `aggregated_signature: Signature`. |
| `Cost` | `u64` | CLVM execution cost units. |

### 2.2 L2BlockHeader

The block header contains all metadata and Merkle commitments for a single L2 block.

```rust
pub struct L2BlockHeader {
    // ── Core identity ──
    pub version: u16,                    // Protocol version (1 = pre-DFSP, 2 = post-DFSP)
    pub height: u64,                     // Block height (0-indexed, genesis = 0)
    pub epoch: u64,                      // Epoch number
    pub parent_hash: Bytes32,            // Hash of the parent block header

    // ── State commitments ──
    pub state_root: Bytes32,             // CoinSet state Merkle root after this block
    pub spends_root: Bytes32,            // Merkle root of SpendBundle hashes
    pub additions_root: Bytes32,         // Merkle root of coin additions (grouped by puzzle_hash)
    pub removals_root: Bytes32,          // Merkle root of removed coin IDs
    pub receipts_root: Bytes32,          // Merkle root of receipts

    // ── L1 anchor ──
    pub l1_height: u32,                  // L1 block height this block references
    pub l1_hash: Bytes32,               // L1 block hash this block references

    // ── Block metadata ──
    pub timestamp: u64,                  // Unix timestamp (seconds)
    pub proposer_index: u32,             // Proposer validator index
    pub spend_bundle_count: u32,         // Number of SpendBundles in body
    pub total_cost: Cost,                // Total CLVM cost of all spends
    pub total_fees: u64,                 // Total fees (value_in - value_out)
    pub additions_count: u32,            // Coins created
    pub removals_count: u32,             // Coins spent
    pub block_size: u32,                 // Serialized block size in bytes
    pub filter_hash: Bytes32,            // BIP158 transaction filter hash (light client support)
    pub extension_data: Bytes32,         // Reserved for future protocol extensions

    // ── L1 proof anchors ──
    pub l1_collateral_coin_id: Option<Bytes32>,            // Proposer's L1 collateral proof
    pub l1_reserve_coin_id: Option<Bytes32>,               // Network validator collateral set anchor
    pub l1_prev_epoch_finalizer_coin_id: Option<Bytes32>,  // Previous epoch finalization proof
    pub l1_curr_epoch_finalizer_coin_id: Option<Bytes32>,  // Current epoch finalizer state
    pub l1_network_coin_id: Option<Bytes32>,               // Network singleton existence proof

    // ── Slash proposals ──
    pub slash_proposal_count: u32,       // Number of slash proposals in body
    pub slash_proposals_root: Bytes32,   // Merkle root of sha256(payload) per proposal

    // ── DFSP Data Layer Roots (v2+) ──
    pub collateral_registry_root: Bytes32,       // Collateral registry SMT root
    pub cid_state_root: Bytes32,                 // CID lifecycle state machine root
    pub node_registry_root: Bytes32,             // Node registry SMT root
    pub namespace_update_root: Bytes32,          // Namespace update delta root (this block only)
    pub dfsp_finalize_commitment_root: Bytes32,  // DFSP epoch-boundary commitment digest
}
```

**Field groups:**

| Group | Fields | Purpose |
|-------|--------|---------|
| Core identity | `version`, `height`, `epoch`, `parent_hash` | Locates the block in the chain. `parent_hash` links to the previous block. |
| State commitments | `state_root`, `spends_root`, `additions_root`, `removals_root`, `receipts_root` | Merkle roots committing to the block's effect on state. `additions_root` and `removals_root` are adopted from Chia's [`FoliageTransactionBlock`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L158) for light client proofs. |
| L1 anchor | `l1_height`, `l1_hash` | Anchors this L2 block to a specific L1 block, enabling cross-chain verification. |
| Block metadata | `timestamp`, `proposer_index`, `spend_bundle_count`, `total_cost`, `total_fees`, `additions_count`, `removals_count`, `block_size`, `filter_hash`, `extension_data` | Summary statistics about block contents. `filter_hash` adopted from Chia's [`FoliageTransactionBlock.filter_hash`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L205) for light client filtering. `extension_data` adopted from Chia's [`FoliageBlockData.extension_data`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L53) for future protocol extensions. |
| L1 proof anchors | `l1_collateral_coin_id`, `l1_reserve_coin_id`, `l1_prev_epoch_finalizer_coin_id`, `l1_curr_epoch_finalizer_coin_id`, `l1_network_coin_id` | Optional coin IDs proving L1 state. Used by validators to verify proposer stake, epoch transitions, and network validity. |
| Slash proposals | `slash_proposal_count`, `slash_proposals_root` | Commits to slash proposal payloads in the body. |
| DFSP roots | `collateral_registry_root`, `cid_state_root`, `node_registry_root`, `namespace_update_root`, `dfsp_finalize_commitment_root` | Five Sparse Merkle Tree roots anchoring the DFSP data layer into consensus. All default to `EMPTY_ROOT` before DFSP activation. |

**Version semantics:**

| Version | Name | Active when | DFSP roots |
|---------|------|-------------|------------|
| 1 | Pre-DFSP | `height < DFSP_ACTIVATION_HEIGHT` | Must be `EMPTY_ROOT` |
| 2 | Post-DFSP | `height >= DFSP_ACTIVATION_HEIGHT` | Must be correct SMT roots |

**Derived methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(height, epoch, parent_hash, state_root, spends_root, additions_root, removals_root, receipts_root, l1_height, l1_hash, proposer_index, spend_bundle_count, total_cost, total_fees, additions_count, removals_count, block_size, filter_hash) -> Self` | Construct a header with automatic version detection. L1 proof anchors default to `None`, DFSP roots to `EMPTY_ROOT`, `extension_data` to `ZERO_HASH`. |
| `new_with_collateral()` | `(..., l1_collateral_coin_id) -> Self` | Construct with L1 collateral proof. |
| `new_with_l1_proofs()` | `(..., l1_collateral_coin_id, l1_reserve_coin_id, l1_prev_epoch_finalizer_coin_id, l1_curr_epoch_finalizer_coin_id, l1_network_coin_id) -> Self` | Construct with all L1 proof anchors. |
| `genesis()` | `(network_id, l1_height, l1_hash) -> Self` | Construct genesis header. Uses `network_id` as `parent_hash`. All counts and roots are zero/empty. |
| `hash()` | `(&self) -> Bytes32` | Compute block hash (see Section 3.1). |
| `validate()` | `(&self) -> Result<(), BlockError>` | Validate header constraints (see Section 5.1). |
| `to_bytes()` | `(&self) -> Vec<u8>` | Serialize via bincode. |
| `from_bytes()` | `(&[u8]) -> Result<Self, BlockError>` | Deserialize via bincode. |

### 2.3 L2Block

A complete L2 block containing the header and body.

```rust
pub struct L2Block {
    pub header: L2BlockHeader,                   // Block header
    pub spend_bundles: Vec<SpendBundle>,          // Atomic transaction units
    pub slash_proposal_payloads: Vec<Vec<u8>>,    // Opaque slash proposals (bincode SlashingEvidence)
    pub proposer_signature: Signature,            // BLS signature over the header
}
```

**Derived methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(header, spend_bundles, proposer_signature) -> Self` | Construct a block with no slash proposals. |
| `hash()` | `(&self) -> Bytes32` | Delegates to `header.hash()`. |
| `height()` | `(&self) -> u64` | Delegates to `header.height`. |
| `epoch()` | `(&self) -> u64` | Delegates to `header.epoch`. |
| `compute_spends_root()` | `(&self) -> Bytes32` | Compute Merkle root over `sha256(spend_bundle)` for each bundle. Returns `EMPTY_ROOT` if no bundles. |
| `compute_additions_root()` | `(&self) -> Bytes32` | Compute additions Merkle root grouped by puzzle_hash (Section 3.4). Chia parity: [`block_body_validation.py:158-175`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L158). |
| `compute_removals_root()` | `(&self) -> Bytes32` | Compute removals Merkle root over removed coin IDs (Section 3.5). Chia parity: [`block_body_validation.py:185`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L185). |
| `compute_filter_hash()` | `(&self) -> Bytes32` | Compute BIP158 filter hash (Section 3.6). Chia parity: [`block_creation.py:199-210`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L199). |
| `compute_slash_proposals_root()` | `(payloads: &[Vec<u8>]) -> Bytes32` | Static. Compute Merkle root over `sha256(payload)` per proposal. Returns `EMPTY_ROOT` if empty. |
| `slash_proposal_leaf_hash()` | `(payload: &[u8]) -> Bytes32` | Static. SHA-256 of one slash proposal payload. |
| `all_additions()` | `(&self) -> Vec<Coin>` | Extract all coins created by all SpendBundles (full Coin objects for Merkle root computation). |
| `all_addition_ids()` | `(&self) -> Vec<CoinId>` | Extract all coin IDs created by all SpendBundles. |
| `all_removals()` | `(&self) -> Vec<CoinId>` | Extract all coin IDs spent by all SpendBundles. |
| `has_duplicate_outputs()` | `(&self) -> Option<Bytes32>` | Returns the first duplicate coin ID in additions, or None. Chia parity: [`block_body_validation.py` Check 13](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L396). |
| `has_double_spends()` | `(&self) -> Option<Bytes32>` | Returns the first duplicate coin ID in removals, or None. Chia parity: [`block_body_validation.py` Check 14](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L400). |
| `validate_structure()` | `(&self) -> Result<(), BlockError>` | Full structural validation (see Section 5.2). |
| `compute_size()` | `(&self) -> usize` | Compute serialized size in bytes. |
| `to_bytes()` | `(&self) -> Vec<u8>` | Serialize via bincode. |
| `from_bytes()` | `(&[u8]) -> Result<Self, BlockError>` | Deserialize via bincode. |

### 2.4 AttestedBlock

A block that has been attested by validators. This is the form of a block after it enters the attestation process.

```rust
pub struct AttestedBlock {
    pub block: L2Block,                    // The original block
    pub signer_bitmap: SignerBitmap,       // Which validators signed
    pub aggregate_signature: Signature,    // Aggregated BLS signature
    pub receipts: ReceiptList,             // Execution receipts
    pub status: BlockStatus,               // Current lifecycle status
}
```

**Derived methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(block, validator_count, receipts) -> Self` | Create with empty signer bitmap and `Pending` status. Initial aggregate signature is the proposer's signature. |
| `signing_percentage()` | `(&self) -> u64` | Percentage of validators who have signed (0-100). |
| `has_soft_finality()` | `(&self, threshold_pct) -> bool` | Whether the signing percentage meets the threshold. |
| `hash()` | `(&self) -> Bytes32` | Delegates to `block.hash()`. |

### 2.5 BlockStatus

Lifecycle status of a block in the chain.

```rust
pub enum BlockStatus {
    Pending,        // Pending validation
    Validated,      // Validated, not yet finalized
    SoftFinalized,  // >67% validator stake signed
    HardFinalized,  // L1 checkpoint confirmed
    Orphaned,       // Not in the canonical chain
    Rejected,       // Rejected as invalid
}
```

**Derived methods:**

| Method | Returns | Description |
|--------|---------|-------------|
| `is_finalized()` | `bool` | `true` for `SoftFinalized` or `HardFinalized`. |
| `is_canonical()` | `bool` | `true` for everything except `Orphaned` and `Rejected`. |

### 2.6 Checkpoint

An epoch summary submitted to L1 for hard finality.

```rust
pub struct Checkpoint {
    pub epoch: u64,              // Epoch number
    pub state_root: Bytes32,     // Final state root after all blocks in epoch
    pub block_root: Bytes32,     // Merkle root of all block hashes in epoch
    pub block_count: u32,        // Number of blocks in epoch
    pub tx_count: u64,           // Total transactions processed
    pub total_fees: u64,         // Total fees collected
    pub prev_checkpoint: Bytes32,// Hash of the previous checkpoint
    pub withdrawals_root: Bytes32, // Withdrawals Merkle root
    pub withdrawal_count: u32,   // Number of withdrawals
}
```

**Derived methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(epoch, state_root, block_root, block_count, tx_count, total_fees, prev_checkpoint, withdrawals_root, withdrawal_count) -> Self` | Construct a checkpoint. |
| `hash()` | `(&self) -> Bytes32` | Compute checkpoint hash (see Section 3.2). |
| `compute_score()` | `(&self, stake_percentage) -> u64` | `stake_percentage * block_count`. Used in checkpoint competition to rank submissions. |
| `to_bytes()` | `(&self) -> Vec<u8>` | Serialize via bincode. |
| `from_bytes()` | `(&[u8]) -> Result<Self, CheckpointError>` | Deserialize via bincode. |

### 2.7 CheckpointSubmission

A checkpoint signed by a set of validators and ready for L1 submission.

```rust
pub struct CheckpointSubmission {
    pub checkpoint: Checkpoint,              // The checkpoint data
    pub signer_bitmap: SignerBitmap,         // Which validators signed
    pub aggregate_signature: Signature,      // Aggregated BLS signature
    pub aggregate_pubkey: PublicKey,          // Aggregated BLS public key
    pub score: u64,                          // Computed competition score
    pub submitter: u32,                      // Submitter validator index
    pub submission_height: Option<u32>,       // L1 height when submitted (if submitted)
    pub submission_coin: Option<Bytes32>,     // L1 coin ID of submission (if submitted)
}
```

**Derived methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(checkpoint, signer_bitmap, aggregate_signature, aggregate_pubkey, score, submitter) -> Self` | Construct with no L1 submission recorded. |
| `hash()` | `(&self) -> Bytes32` | Delegates to `checkpoint.hash()`. |
| `epoch()` | `(&self) -> u64` | Delegates to `checkpoint.epoch`. |
| `signing_percentage()` | `(&self) -> u64` | Percentage of validators who signed. |
| `meets_threshold()` | `(&self, threshold_pct) -> bool` | Whether signing percentage meets threshold. |
| `record_submission()` | `(&mut self, height, coin_id)` | Record L1 submission details. |
| `is_submitted()` | `(&self) -> bool` | Whether L1 submission has been recorded. |
| `to_bytes()` | `(&self) -> Vec<u8>` | Serialize via bincode. |
| `from_bytes()` | `(&[u8]) -> Result<Self, CheckpointError>` | Deserialize via bincode. |

### 2.8 CheckpointStatus

Lifecycle status of a checkpoint.

```rust
pub enum CheckpointStatus {
    Pending,                                            // Not yet started
    Collecting,                                         // Accepting submissions
    WinnerSelected { winner_hash: Bytes32, winner_score: u64 }, // Winner determined
    Finalized { winner_hash: Bytes32, l1_height: u32 }, // Confirmed on L1
    Failed,                                             // Epoch ended without valid checkpoint
}
```

### 2.9 Receipt and ReceiptList

Execution receipts for SpendBundles in a block.

```rust
pub enum ReceiptStatus {
    Success = 0,
    InsufficientBalance = 1,
    InvalidNonce = 2,
    InvalidSignature = 3,
    AccountNotFound = 4,
    Failed = 255,
}

pub struct Receipt {
    pub tx_id: Bytes32,            // Transaction (SpendBundle) ID
    pub block_height: u64,         // Block height
    pub tx_index: u32,             // Index within block
    pub status: ReceiptStatus,     // Execution outcome
    pub fee_charged: u64,          // Fee actually charged
    pub post_state_root: Bytes32,  // State root after this transaction
    pub cumulative_fees: u64,      // Cumulative fees up to this point
}

pub struct ReceiptList {
    pub receipts: Vec<Receipt>,    // Ordered receipts
    pub root: Bytes32,             // Merkle root of receipt hashes
}
```

**ReceiptList methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `() -> Self` | Empty receipt list with `EMPTY_ROOT`. |
| `from_receipts()` | `(Vec<Receipt>) -> Self` | Construct from receipts, computing Merkle root. |
| `push()` | `(&mut self, Receipt)` | Append a receipt (root becomes stale until `finalize()`). |
| `finalize()` | `(&mut self)` | Recompute Merkle root. |
| `get()` | `(&self, index) -> Option<&Receipt>` | Get receipt by index. |
| `get_by_tx_id()` | `(&self, Bytes32) -> Option<&Receipt>` | Get receipt by transaction ID. |
| `len()` | `(&self) -> usize` | Number of receipts. |
| `success_count()` | `(&self) -> usize` | Number of successful receipts. |
| `failure_count()` | `(&self) -> usize` | Number of failed receipts. |
| `total_fees()` | `(&self) -> u64` | Sum of all fees charged. |

### 2.10 SignerBitmap

A compact bitmap tracking which validators have signed a block or checkpoint.

```rust
pub struct SignerBitmap {
    bits: Vec<u8>,               // Bit array
    validator_count: u32,        // Number of validators tracked
}
```

**Constants:**

| Constant | Value | Description |
|----------|-------|-------------|
| `MAX_VALIDATORS` | `65,536` | Maximum supported validator count. |

**Methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(validator_count) -> Self` | Create empty bitmap (no signers). |
| `from_bytes()` | `(bytes, validator_count) -> Result<Self, SignerBitmapError>` | Restore from raw bytes. |
| `has_signed()` | `(&self, index) -> bool` | Check if validator at index has signed. |
| `set_signed()` | `(&mut self, index) -> Result<(), SignerBitmapError>` | Mark validator as signed. |
| `signer_count()` | `(&self) -> u32` | Number of signers. |
| `signing_percentage()` | `(&self) -> u64` | Signing percentage (0-100). |
| `has_threshold()` | `(&self, threshold_pct) -> bool` | Whether signing percentage meets threshold. |
| `merge()` | `(&mut self, other: &SignerBitmap)` | Bitwise OR merge of two bitmaps. |
| `signer_indices()` | `(&self) -> Vec<u32>` | Ordered list of signer validator indices. |
| `as_bytes()` | `(&self) -> &[u8]` | Raw byte access. |

### 2.11 Constants

```rust
/// SHA-256 of empty input. Canonical Merkle root when a tree has no leaves.
/// Hex: 0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
pub const EMPTY_ROOT: Bytes32 =
    Bytes32::from_hex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");

/// 32 zero-bytes. Used as the hash input for `None` optional fields and as the
/// default for `extension_data`.
/// Hex: 0x0000000000000000000000000000000000000000000000000000000000000000
pub const ZERO_HASH: Bytes32 =
    Bytes32::from_hex("0000000000000000000000000000000000000000000000000000000000000000");

/// Maximum serialized block size in bytes (10 MB).
pub const MAX_BLOCK_SIZE: usize = 10_000_000;

/// Maximum total CLVM cost per block (550 billion cost units).
/// Derived from: L1_MAX_BLOCK_COST_CLVM (11B) * MAX_BOUNDARY_SPEND_EQUIVALENTS (50).
pub const MAX_COST_PER_BLOCK: Cost = 550_000_000_000;

/// Maximum number of slash proposals per block.
pub const MAX_SLASH_PROPOSALS_PER_BLOCK: usize = 64;

/// Maximum size of a single slash proposal payload in bytes.
pub const MAX_SLASH_PROPOSAL_PAYLOAD_BYTES: usize = 65_536;

/// DFSP activation height. Defaults to u64::MAX (disabled).
/// Overridable via DIG_DFSP_ACTIVATION_HEIGHT environment variable.
pub const DFSP_ACTIVATION_HEIGHT: u64 = u64::MAX;

/// Maximum seconds a block timestamp can be ahead of the validator's wall clock.
/// Adopted from Chia's 5-minute future bound
/// ([`block_header_validation.py` Check 26a](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py),
/// `TIMESTAMP_TOO_FAR_IN_FUTURE`).
pub const MAX_FUTURE_TIMESTAMP_SECONDS: u64 = 300;

/// Tagged Merkle hashing prefixes, adopted from Chia
/// ([`merkle_utils.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/wallet/util/merkle_utils.py)).
/// Leaf prefix prevents second-preimage attacks on Merkle proofs.
pub const HASH_LEAF_PREFIX: u8 = 0x01;
/// Internal node prefix for domain separation in Merkle trees.
pub const HASH_TREE_PREFIX: u8 = 0x02;
```

## 3. Hashing

### 3.1 Block Header Hash

The block header hash is computed as `SHA-256` over a fixed-order concatenation of all header fields. This hash is the **block's identity** — it appears in `parent_hash` of child blocks, in attestation messages, and in checkpoint `block_root` Merkle trees.

**Hash input order** (each field encoded as specified):

| # | Field | Encoding | Bytes |
|---|-------|----------|-------|
| 1 | `version` | `u16` LE | 2 |
| 2 | `height` | `u64` LE | 8 |
| 3 | `epoch` | `u64` LE | 8 |
| 4 | `parent_hash` | raw bytes | 32 |
| 5 | `state_root` | raw bytes | 32 |
| 6 | `spends_root` | raw bytes | 32 |
| 7 | `additions_root` | raw bytes | 32 |
| 8 | `removals_root` | raw bytes | 32 |
| 9 | `receipts_root` | raw bytes | 32 |
| 10 | `l1_height` | `u32` LE | 4 |
| 11 | `l1_hash` | raw bytes | 32 |
| 12 | `timestamp` | `u64` LE | 8 |
| 13 | `proposer_index` | `u32` LE | 4 |
| 14 | `spend_bundle_count` | `u32` LE | 4 |
| 15 | `total_cost` | `u64` LE | 8 |
| 16 | `total_fees` | `u64` LE | 8 |
| 17 | `additions_count` | `u32` LE | 4 |
| 18 | `removals_count` | `u32` LE | 4 |
| 19 | `block_size` | `u32` LE | 4 |
| 20 | `filter_hash` | raw bytes | 32 |
| 21 | `extension_data` | raw bytes | 32 |
| 22 | `l1_collateral_coin_id` | raw bytes or `ZERO_HASH` if None | 32 |
| 23 | `l1_reserve_coin_id` | raw bytes or `ZERO_HASH` if None | 32 |
| 24 | `l1_prev_epoch_finalizer_coin_id` | raw bytes or `ZERO_HASH` if None | 32 |
| 25 | `l1_curr_epoch_finalizer_coin_id` | raw bytes or `ZERO_HASH` if None | 32 |
| 26 | `l1_network_coin_id` | raw bytes or `ZERO_HASH` if None | 32 |
| 27 | `slash_proposal_count` | `u32` LE | 4 |
| 28 | `slash_proposals_root` | raw bytes | 32 |
| 29 | `collateral_registry_root` | raw bytes | 32 |
| 30 | `cid_state_root` | raw bytes | 32 |
| 31 | `node_registry_root` | raw bytes | 32 |
| 32 | `namespace_update_root` | raw bytes | 32 |
| 33 | `dfsp_finalize_commitment_root` | raw bytes | 32 |

**Total hash input:** 626 bytes (fixed size).

```
block_hash = SHA256(version LE || height LE || epoch LE || parent_hash || ... || dfsp_finalize_commitment_root)
```

### 3.2 Checkpoint Hash

The checkpoint hash is computed as `SHA-256` over a fixed-order concatenation of all checkpoint fields.

| # | Field | Encoding | Bytes |
|---|-------|----------|-------|
| 1 | `epoch` | `u64` LE | 8 |
| 2 | `state_root` | raw bytes | 32 |
| 3 | `block_root` | raw bytes | 32 |
| 4 | `block_count` | `u32` LE | 4 |
| 5 | `tx_count` | `u64` LE | 8 |
| 6 | `total_fees` | `u64` LE | 8 |
| 7 | `prev_checkpoint` | raw bytes | 32 |
| 8 | `withdrawals_root` | raw bytes | 32 |
| 9 | `withdrawal_count` | `u32` LE | 4 |

**Total hash input:** 160 bytes (fixed size).

```
checkpoint_hash = SHA256(epoch LE || state_root || block_root || ... || withdrawal_count LE)
```

### 3.3 Merkle Root Computation

Several header fields are Merkle roots computed over ordered leaf sets:

| Root field | Leaves | Computed by | Chia crate |
|-----------|--------|-------------|------------|
| `spends_root` | SpendBundle hashes in block order | `chia-sdk-types::MerkleTree` | chia-sdk-types |
| `additions_root` | Coin additions grouped by puzzle_hash | `chia-consensus::merkle_set::compute_merkle_set_root()` | chia-consensus |
| `removals_root` | Removed coin IDs in block order | `chia-consensus::merkle_set::compute_merkle_set_root()` | chia-consensus |
| `receipts_root` | Receipt hashes in block order | `chia-sdk-types::MerkleTree` | chia-sdk-types |
| `slash_proposals_root` | Slash proposal payloads in block order | `chia-sdk-types::MerkleTree` | chia-sdk-types |

**Merkle set roots** (`additions_root`, `removals_root`) use `chia-consensus::merkle_set::compute_merkle_set_root()` directly. This is the same Rust function used by Chia L1 for block body validation, guaranteeing bit-for-bit parity. The Merkle set uses **tagged hashing** with domain separation ([`merkle_utils.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/wallet/util/merkle_utils.py)):

```rust
// These constants are built into chia-consensus::merkle_set — not redefined by dig-block.
const HASH_LEAF_PREFIX: u8 = 0x01;   // Leaf: SHA-256(0x01 || data)
const HASH_TREE_PREFIX: u8 = 0x02;   // Node: SHA-256(0x02 || left || right)
```

This prevents second-preimage attacks where a valid Merkle proof for a leaf could be reinterpreted as an internal node or vice versa. When the leaf set is empty, the root is `EMPTY_ROOT`.

**Other Merkle roots** (`spends_root`, `receipts_root`, `slash_proposals_root`) use `chia-sdk-types::MerkleTree` for binary tree construction with the same tagged hashing scheme. `MerkleTree` also provides `MerkleProof` generation for light client inclusion/exclusion proofs.

All SHA-256 operations use `chia-sha2::Sha256` for hash compatibility with the Chia ecosystem.

### 3.4 Additions Root Construction

The `additions_root` is computed by calling `chia-consensus::merkle_set::compute_merkle_set_root()` over a Merkle set built from coin additions, matching Chia's block body validation identically ([`block_body_validation.py:158-187`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L158)). Additions are grouped by `puzzle_hash` for efficient wallet lookups:

```rust
use chia_consensus::merkle_set::compute_merkle_set_root;

fn compute_additions_root(additions: &[(Coin, Bytes32)]) -> Bytes32 {
    // 1. Group coin additions by puzzle_hash
    let mut by_puzzle: HashMap<Bytes32, Vec<Bytes32>> = HashMap::new();
    for (coin, coin_name) in additions {
        by_puzzle.entry(coin.puzzle_hash).or_default().push(*coin_name);
    }

    // 2. Build Merkle set items: [puzzle_hash, hash_coin_ids(coin_ids)] pairs
    let mut items: Vec<Bytes32> = Vec::new();
    for (puzzle_hash, coin_ids) in &by_puzzle {
        items.push(*puzzle_hash);
        items.push(hash_coin_ids(coin_ids));  // chia-consensus function
    }

    // 3. Delegate to chia-consensus for the actual Merkle set root
    Bytes32::from(compute_merkle_set_root(&items))
}
```

This grouping enables a light client to prove that a specific `puzzle_hash` has (or doesn't have) coins in a block using a single Merkle proof, without enumerating all additions.

### 3.5 Removals Root Construction

The `removals_root` is computed by calling `chia-consensus::merkle_set::compute_merkle_set_root()` over all removed coin IDs, matching Chia's removals root construction identically ([`block_body_validation.py:185`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L185)):

```rust
use chia_consensus::merkle_set::compute_merkle_set_root;

let removals_root = Bytes32::from(compute_merkle_set_root(&all_removed_coin_ids));
```

### 3.6 Filter Hash Construction

The `filter_hash` is the SHA-256 hash of a BIP158 compact filter, adopted from Chia's transaction filter ([`block_creation.py:199-210`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L199), [`block_header_validation.py` Check 25](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py)). The filter enables light clients to quickly determine if a block might contain relevant transactions:

```
Filter input items:
  1. puzzle_hash of each coin addition (created coins)
  2. coin_id of each coin removal (spent coins)

filter = BIP158_encode(items)
filter_hash = SHA-256(filter)
```

The BIP158 encoding produces a compact probabilistic filter (Golomb-Rice coded set) that supports efficient membership testing with a tunable false-positive rate. A light client can test whether its watched puzzle hashes or coin IDs appear in the filter without downloading the full block body.

## 4. Error Types

### 4.1 BlockError

```rust
pub enum BlockError {
    /// Block payload or metadata is malformed.
    InvalidData(String),

    /// Block version does not match expected version for its height.
    InvalidVersion { expected: u16, actual: u16 },

    /// Serialized block size exceeds MAX_BLOCK_SIZE.
    TooLarge { size: usize, max: usize },

    /// Block execution cost exceeds MAX_COST_PER_BLOCK.
    CostExceeded { cost: Cost, max: Cost },

    /// Header spend_bundle_count does not match actual SpendBundle count in body.
    SpendBundleCountMismatch { header: u32, actual: usize },

    /// Recomputed spends Merkle root does not match header spends_root.
    InvalidSpendsRoot { expected: Bytes32, computed: Bytes32 },

    /// Recomputed receipts Merkle root does not match header receipts_root.
    InvalidReceiptsRoot { expected: Bytes32, computed: Bytes32 },

    /// Parent hash does not match expected value.
    /// (Used by chain-level validation, not structural validation.)
    InvalidParent { expected: Bytes32, got: Bytes32 },

    /// Proposer BLS signature is invalid.
    InvalidProposerSignature,

    /// Block with given hash was not found.
    NotFound(Bytes32),

    /// State root does not match expected value.
    InvalidStateRoot { expected: Bytes32, computed: Bytes32 },

    /// Header slash_proposal_count does not match actual payload count in body.
    SlashProposalCountMismatch { header: u32, actual: usize },

    /// Recomputed slash proposals Merkle root does not match header slash_proposals_root.
    InvalidSlashProposalsRoot { expected: Bytes32, computed: Bytes32 },

    /// A single slash proposal payload exceeds MAX_SLASH_PROPOSAL_PAYLOAD_BYTES.
    SlashProposalPayloadTooLarge { actual: usize, max: usize },

    /// Number of slash proposals exceeds MAX_SLASH_PROPOSALS_PER_BLOCK.
    TooManySlashProposals { actual: usize, max: usize },

    /// Recomputed additions Merkle root does not match header additions_root.
    /// Chia parity: [`block_body_validation.py` Check 11](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L158), `BAD_ADDITION_ROOT`.
    InvalidAdditionsRoot { expected: Bytes32, computed: Bytes32 },

    /// Recomputed removals Merkle root does not match header removals_root.
    /// Chia parity: [`block_body_validation.py` Check 11](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L185), `BAD_REMOVAL_ROOT`.
    InvalidRemovalsRoot { expected: Bytes32, computed: Bytes32 },

    /// Recomputed filter hash does not match header filter_hash.
    /// Chia parity: [`block_header_validation.py` Check 25](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py), `INVALID_TRANSACTIONS_FILTER_HASH`.
    InvalidFilterHash { expected: Bytes32, computed: Bytes32 },

    /// A coin ID appears more than once in the block's additions.
    /// Chia parity: [`block_body_validation.py` Check 13](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L396), `DUPLICATE_OUTPUT`.
    DuplicateOutput { coin_id: Bytes32 },

    /// A coin ID appears more than once in the block's removals.
    /// Chia parity: [`block_body_validation.py` Check 14](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L400), `DOUBLE_SPEND`.
    DoubleSpendInBlock { coin_id: Bytes32 },

    /// Header additions_count does not match computed additions from body.
    AdditionsCountMismatch { header: u32, actual: usize },

    /// Header removals_count does not match computed removals from body.
    RemovalsCountMismatch { header: u32, actual: usize },

    /// Block timestamp is too far in the future.
    /// Chia parity: [`block_header_validation.py` Check 26a](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py), `TIMESTAMP_TOO_FAR_IN_FUTURE`.
    TimestampTooFarInFuture { timestamp: u64, max_allowed: u64 },
}
```

### 4.2 CheckpointError

```rust
pub enum CheckpointError {
    /// Serialized checkpoint bytes could not be decoded.
    InvalidData(String),

    /// Requested checkpoint epoch does not exist.
    NotFound(u64),

    /// Checkpoint fields failed semantic validation.
    Invalid(String),

    /// Submitted checkpoint did not beat the current best score.
    ScoreNotHigher { current: u64, submitted: u64 },

    /// Epoch mismatch between submitted and expected checkpoint.
    EpochMismatch { expected: u64, got: u64 },

    /// Checkpoint has already been finalized.
    AlreadyFinalized,

    /// Competition has not been started.
    NotStarted,
}
```

### 4.3 SignerBitmapError

```rust
pub enum SignerBitmapError {
    /// Validator index exceeds bitmap size.
    IndexOutOfBounds { index: u32, max: u32 },

    /// Validator count exceeds MAX_VALIDATORS.
    TooManyValidators(usize),

    /// Byte slice length does not match expected bitmap size.
    InvalidLength { expected: usize, got: usize },

    /// Validator count mismatch when merging or comparing bitmaps.
    ValidatorCountMismatch { expected: u32, got: u32 },
}
```

### 4.4 ReceiptError

```rust
pub enum ReceiptError {
    /// Serialized receipt data could not be decoded.
    InvalidData(String),

    /// Receipt with given transaction ID was not found.
    NotFound(Bytes32),
}
```

## 5. Structural Validation (Tier 1)

Structural validation checks that a block is internally consistent without any chain context. These checks can be performed on a block received from any source (network, disk, test fixture) before attempting execution or state-level validation. This is Tier 1 of the validation pipeline described in Section 7.

### 5.1 Header Validation (`L2BlockHeader::validate`)

```
┌──────────────────────────────────────────┐
│           Header Validation              │
├──────────────────────────────────────────┤
│ 1. Check version matches height          │
│ 2. Check DFSP roots pre-activation       │
│ 3. Check total_cost <= MAX_COST          │
│ 4. Check block_size <= MAX_SIZE          │
│ 5. Check timestamp not too far in future │
└──────────────────────────────────────────┘
```

**Step 1 — Version check:**
- Compute `expected_version` from `height` and `DFSP_ACTIVATION_HEIGHT`.
- If `height >= DFSP_ACTIVATION_HEIGHT` then expected = `VERSION_V2` (2), else `VERSION_V1` (1).
- If `DFSP_ACTIVATION_HEIGHT == u64::MAX` (disabled), expected is always `VERSION_V1`.
- Reject with `BlockError::InvalidVersion` if `version != expected_version`.

**Step 2 — DFSP root pre-activation check:**
- If `height < DFSP_ACTIVATION_HEIGHT`, all five DFSP roots (`collateral_registry_root`, `cid_state_root`, `node_registry_root`, `namespace_update_root`, `dfsp_finalize_commitment_root`) must equal `EMPTY_ROOT`.
- Reject with `BlockError::InvalidData` if any DFSP root is non-empty before activation.

**Step 3 — Cost check:**
- Reject with `BlockError::CostExceeded` if `total_cost > MAX_COST_PER_BLOCK`.

**Step 4 — Size check:**
- Reject with `BlockError::TooLarge` if `block_size > MAX_BLOCK_SIZE`.

**Step 5 — Timestamp future bound:** (Adopted from Chia [`block_header_validation.py` Check 26a](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py), `TIMESTAMP_TOO_FAR_IN_FUTURE`)
- Reject with `BlockError::TimestampTooFarInFuture` if `timestamp > now() + MAX_FUTURE_TIMESTAMP_SECONDS`.
- `MAX_FUTURE_TIMESTAMP_SECONDS` defaults to 300 (5 minutes), matching Chia's threshold.
- This is a structural check that prevents clock-skew attacks without requiring chain context.

### 5.2 Full Block Validation (`L2Block::validate_structure`)

```
┌──────────────────────────────────────────────────────────┐
│              Block Structure Validation                   │
├──────────────────────────────────────────────────────────┤
│  1. Validate header (Section 5.1)                        │
│  2. Check spend_bundle_count matches body                │
│  3. Recompute spends_root, compare to header             │
│  4. Check additions_count matches computed additions     │
│  5. Check removals_count matches computed removals       │
│  6. Check no duplicate outputs (Chia Check 13)           │
│  7. Check no within-block double spends (Chia Check 14)  │
│  8. Recompute additions_root, compare to header          │
│  9. Recompute removals_root, compare to header           │
│ 10. Recompute filter_hash, compare to header             │
│ 11. Check slash proposal count limit                     │
│ 12. Check each slash proposal size limit                 │
│ 13. Check slash_proposal_count matches body              │
│ 14. Recompute slash_proposals_root, compare              │
│ 15. Compute serialized size, check limit                 │
└──────────────────────────────────────────────────────────┘
```

**Step 1** — Delegate to `header.validate()`.

**Step 2** — `header.spend_bundle_count` must equal `spend_bundles.len()`. Reject with `SpendBundleCountMismatch` otherwise.

**Step 3** — Recompute the spends Merkle root from the actual SpendBundles. Reject with `InvalidSpendsRoot` if it differs from `header.spends_root`.

**Step 4** — Count all coin additions across all SpendBundles. Reject with `AdditionsCountMismatch` if `header.additions_count` does not match.

**Step 5** — Count all coin removals across all SpendBundles. Reject with `RemovalsCountMismatch` if `header.removals_count` does not match.

**Step 6 — Duplicate output detection:** (Adopted from Chia [`block_body_validation.py` Check 13](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L396), `DUPLICATE_OUTPUT`)
- Collect all coin IDs from `all_additions()`. If any coin ID appears more than once, reject with `DuplicateOutput`. This catches invalid block construction where the same coin would be created twice.

**Step 7 — Within-block double-spend detection:** (Adopted from Chia [`block_body_validation.py` Check 14](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L400), `DOUBLE_SPEND`)
- Collect all coin IDs from `all_removals()`. If any coin ID appears more than once, reject with `DoubleSpendInBlock`. This catches invalid block construction where the same coin would be spent twice.

**Step 8 — Additions root check:** (Adopted from Chia [`block_body_validation.py` Check 11](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L158), `BAD_ADDITION_ROOT`)
- Recompute `additions_root` from all coin additions using the puzzle_hash-grouped Merkle set construction (Section 3.4). Reject with `InvalidAdditionsRoot` if it differs from `header.additions_root`.

**Step 9 — Removals root check:** (Adopted from Chia [`block_body_validation.py` Check 11](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L185), `BAD_REMOVAL_ROOT`)
- Recompute `removals_root` from all removed coin IDs using the Merkle set construction (Section 3.5). Reject with `InvalidRemovalsRoot` if it differs from `header.removals_root`.

**Step 10 — Filter hash check:** (Adopted from Chia [`block_header_validation.py` Check 25](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_header_validation.py), `INVALID_TRANSACTIONS_FILTER_HASH`)
- Recompute the BIP158 filter from addition puzzle hashes and removal coin IDs (Section 3.6). Hash it. Reject with `InvalidFilterHash` if it differs from `header.filter_hash`.

**Step 11** — `slash_proposal_payloads.len()` must not exceed `MAX_SLASH_PROPOSALS_PER_BLOCK`. Reject with `TooManySlashProposals` otherwise.

**Step 12** — Each individual payload in `slash_proposal_payloads` must not exceed `MAX_SLASH_PROPOSAL_PAYLOAD_BYTES`. Reject with `SlashProposalPayloadTooLarge` on the first violation.

**Step 13** — `header.slash_proposal_count` must equal `slash_proposal_payloads.len()`. Reject with `SlashProposalCountMismatch` otherwise.

**Step 14** — Recompute the slash proposals Merkle root from the actual payloads. Reject with `InvalidSlashProposalsRoot` if it differs from `header.slash_proposals_root`.

**Step 15** — Compute the full serialized size of the block. Reject with `TooLarge` if it exceeds `MAX_BLOCK_SIZE`.

## 6. Block Production

Block production is the process of assembling a valid `L2Block` from pending SpendBundles. The `BlockBuilder` is the crate's primary production API — it accumulates transactions, enforces limits during construction, computes all derived header fields, and emits a signed block that is structurally valid by design.

### 6.1 BlockBuilder

```rust
pub struct BlockBuilder {
    // ── Caller-provided context ──
    height: u64,
    epoch: u64,
    parent_hash: Bytes32,
    l1_height: u32,
    l1_hash: Bytes32,
    proposer_index: u32,

    // ── Accumulated body ──
    spend_bundles: Vec<SpendBundle>,
    slash_proposal_payloads: Vec<Vec<u8>>,

    // ── Running totals (updated on each add) ──
    total_cost: Cost,
    total_fees: u64,
    additions: Vec<Coin>,
    removals: Vec<CoinId>,
}
```

### 6.2 Builder Lifecycle

```
 ┌──────────────────────────────────────────────┐
 │              BlockBuilder Lifecycle           │
 ├──────────────────────────────────────────────┤
 │                                              │
 │  1. new(height, epoch, parent_hash,          │
 │         l1_height, l1_hash, proposer_index)  │
 │              │                               │
 │              ▼                               │
 │  2. add_spend_bundle(bundle, cost, fee)      │
 │     ├─ Check cost budget remaining           │
 │     ├─ Check block size budget remaining     │
 │     ├─ Accumulate additions/removals         │
 │     ├─ Update running cost/fee totals        │
 │     └─ Return Ok or Err(BlockFull)           │
 │         ... repeat for each bundle ...       │
 │              │                               │
 │  3. add_slash_proposal(payload)  [optional]  │
 │     ├─ Check count limit                     │
 │     └─ Check payload size limit              │
 │              │                               │
 │  4. set_l1_proofs(...)           [optional]  │
 │              │                               │
 │  5. set_dfsp_roots(...)          [optional]  │
 │              │                               │
 │  6. build(state_root, receipts_root,         │
 │           signer: &dyn BlockSigner)          │
 │     ├─ Compute spends_root                   │
 │     ├─ Compute additions_root                │
 │     ├─ Compute removals_root                 │
 │     ├─ Compute filter_hash                   │
 │     ├─ Compute slash_proposals_root          │
 │     ├─ Assemble L2BlockHeader                │
 │     ├─ Compute block_size                    │
 │     ├─ Sign header → proposer_signature      │
 │     └─ Return L2Block                        │
 │                                              │
 └──────────────────────────────────────────────┘
```

### 6.3 Builder Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(height, epoch, parent_hash, l1_height, l1_hash, proposer_index) -> Self` | Create an empty builder. |
| `add_spend_bundle()` | `(&mut self, bundle: SpendBundle, cost: Cost, fee: u64) -> Result<(), BuilderError>` | Add a SpendBundle. Rejects if adding it would exceed `MAX_COST_PER_BLOCK` or `MAX_BLOCK_SIZE`. Extracts additions/removals from the bundle and updates running totals. |
| `add_slash_proposal()` | `(&mut self, payload: Vec<u8>) -> Result<(), BuilderError>` | Add a slash proposal payload. Rejects if count exceeds `MAX_SLASH_PROPOSALS_PER_BLOCK` or payload exceeds `MAX_SLASH_PROPOSAL_PAYLOAD_BYTES`. |
| `set_l1_proofs()` | `(&mut self, collateral, reserve, prev_finalizer, curr_finalizer, network_coin)` | Set optional L1 proof anchor coin IDs. |
| `set_dfsp_roots()` | `(&mut self, collateral_registry, cid_state, node_registry, namespace_update, finalize_commitment)` | Set DFSP Sparse Merkle Tree roots (required post-activation). |
| `set_extension_data()` | `(&mut self, data: Bytes32)` | Set extension data field. |
| `remaining_cost()` | `(&self) -> Cost` | Cost budget remaining before `MAX_COST_PER_BLOCK`. |
| `spend_bundle_count()` | `(&self) -> usize` | Number of bundles added so far. |
| `build()` | `(self, state_root: Bytes32, receipts_root: Bytes32, signer: &dyn BlockSigner) -> Result<L2Block, BuilderError>` | Finalize and sign the block. See Section 6.4. |

### 6.4 Build Pipeline

The `build()` method computes all derived header fields from the accumulated body:

**Step 1 — Compute Merkle roots:**
```
spends_root           = MerkleTree(spend_bundles.map(|b| b.hash())).root()
additions_root        = compute_additions_root(additions)      // Section 3.4
removals_root         = compute_removals_root(removals)        // Section 3.5
slash_proposals_root  = compute_slash_proposals_root(payloads) // Section 3.3
```

**Step 2 — Compute filter:**
```
filter_hash = compute_filter_hash(additions, removals)         // Section 3.6
```

**Step 3 — Compute counts:**
```
spend_bundle_count = spend_bundles.len()
additions_count    = additions.len()
removals_count     = removals.len()
slash_proposal_count = slash_proposal_payloads.len()
```

**Step 4 — Assemble header:**
All computed fields plus caller-provided `state_root` and `receipts_root` are assembled into an `L2BlockHeader`. The `version` is auto-detected from `height`. The `timestamp` is set to the current wall-clock time.

**Step 5 — Compute block size:**
The header's `block_size` field is set to the serialized size of the complete block (header + body). This requires a two-pass approach: assemble the block with `block_size = 0`, serialize to measure, then update `block_size` and re-hash.

**Step 6 — Sign:**
The header hash is signed by the proposer via the `BlockSigner` trait:

```rust
pub trait BlockSigner {
    fn sign_block(&self, header_hash: &Bytes32) -> Result<Signature, SignerError>;
}
```

**Step 7 — Emit L2Block:**
Return the assembled `L2Block` with header, spend_bundles, slash_proposal_payloads, and proposer_signature.

### 6.5 BuilderError

```rust
pub enum BuilderError {
    /// Adding this SpendBundle would exceed MAX_COST_PER_BLOCK.
    CostBudgetExceeded { current: Cost, addition: Cost, max: Cost },

    /// Adding this SpendBundle would exceed MAX_BLOCK_SIZE.
    SizeBudgetExceeded { current: usize, addition: usize, max: usize },

    /// Slash proposal count would exceed MAX_SLASH_PROPOSALS_PER_BLOCK.
    TooManySlashProposals { max: usize },

    /// Slash proposal payload exceeds MAX_SLASH_PROPOSAL_PAYLOAD_BYTES.
    SlashProposalTooLarge { size: usize, max: usize },

    /// Block signing failed.
    SigningFailed(String),

    /// No SpendBundles were added (empty block body).
    EmptyBlock,

    /// DFSP roots are required at this height but were not set.
    MissingDfspRoots,
}
```

### 6.6 Checkpoint Production

Checkpoints are simpler to build since they summarize an epoch rather than assembling transactions:

```rust
pub struct CheckpointBuilder {
    epoch: u64,
    block_hashes: Vec<Bytes32>,    // All block hashes in this epoch
    state_root: Bytes32,           // Final state root after last block
    total_tx_count: u64,
    total_fees: u64,
    prev_checkpoint: Bytes32,
    withdrawals: Vec<Bytes32>,     // Withdrawal leaf hashes
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `(epoch, prev_checkpoint) -> Self` | Create builder for an epoch. |
| `add_block()` | `(&mut self, block_hash: Bytes32, tx_count: u64, fees: u64)` | Accumulate a block's contribution. |
| `set_state_root()` | `(&mut self, state_root: Bytes32)` | Set the final state root. |
| `add_withdrawal()` | `(&mut self, withdrawal_hash: Bytes32)` | Add a withdrawal leaf. |
| `build()` | `(self) -> Checkpoint` | Compute `block_root` and `withdrawals_root` Merkle trees, emit `Checkpoint`. |

## 7. Full Block Validation

Block validation is the process of verifying that a received `L2Block` is correct. Validation is organized into **three tiers**, each requiring progressively more external context. Callers can stop at any tier depending on their needs (e.g., a light client may only perform structural validation).

### 7.1 Validation Tiers

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Validation Tiers                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Tier 1: Structural Validation     (no external state)             │
│  ├─ Header field constraints                                        │
│  ├─ Merkle root recomputation                                       │
│  ├─ Size/cost limits                                                │
│  ├─ Duplicate output / double-spend detection                       │
│  └─ Count agreement (header vs body)                                │
│          │                                                          │
│          ▼                                                          │
│  Tier 2: Execution Validation      (requires clvmr::Allocator)     │
│  ├─ CLVM puzzle execution per CoinSpend                             │
│  ├─ Condition parsing and assertion checking                        │
│  ├─ Announcement creation/resolution                                │
│  ├─ BLS aggregate signature verification per SpendBundle            │
│  ├─ Coin conservation (no minting)                                  │
│  └─ Fee consistency                                                 │
│          │                                                          │
│          ▼                                                          │
│  Tier 3: State Validation          (requires CoinLookup)           │
│  ├─ Coin existence (removals must exist and be unspent)             │
│  ├─ Coin non-existence (additions must not already exist)           │
│  ├─ Puzzle hash verification (reveal matches coin record)           │
│  ├─ Height/time lock evaluation against chain state                 │
│  ├─ Proposer signature verification                                 │
│  └─ State root verification (apply additions/removals, compare)     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 7.2 Validation Context Traits

The crate defines traits for injecting external state. Callers provide implementations backed by their storage layer.

The crate defines **minimal** traits for injecting external state. CLVM execution is delegated to `dig-clvm`, and all Chia types are used directly.

```rust
// ── DIG crate for CLVM execution ──
use dig_clvm::{validate_spend_bundle, ValidationContext, ValidationConfig, SpendResult};

// ── Types reused from Chia crates (NOT redefined) ──
use chia_protocol::{Coin, CoinSpend, CoinState, Bytes32, SpendBundle, Program};
use chia_bls::{PublicKey, Signature, aggregate_verify};
use chia_consensus::merkle_set::compute_merkle_set_root;
use chia_consensus::spendbundle_conditions::OwnedSpendBundleConditions;
use chia_sdk_types::{Condition, MerkleTree};
use chia_sdk_signer::{RequiredSignature, AggSigConstants};
use chia_sha2::Sha256;
use clvm_utils::tree_hash;

/// Looks up coin state by ID. Injected into Tier 3 validation.
/// Uses chia-protocol's CoinState directly — no custom CoinRecord type.
pub trait CoinLookup {
    /// Look up a coin's state. Returns CoinState (from chia-protocol)
    /// which contains coin, created_height, and spent_height.
    fn get_coin_state(&self, coin_id: &Bytes32) -> Option<CoinState>;
    fn get_chain_height(&self) -> u64;
    fn get_chain_timestamp(&self) -> u64;
}

/// Signs a block header hash. Injected into block production.
pub trait BlockSigner {
    fn sign_block(&self, header_hash: &Bytes32) -> Result<Signature, SignerError>;
}
```

**CLVM execution via `dig-clvm`.** Tier 2 validation calls `dig_clvm::validate_spend_bundle()`, which wraps `chia-consensus::run_spendbundle()` with DIG-specific configuration (L2 cost limits, BLS cache, mempool mode flags). The `dig-clvm` crate is the single integration point for CLVM execution across the entire DIG ecosystem — `dig-mempool` already depends on it. There is no separate `ClvmRunner` trait because `dig-clvm` *is* the runner.

**Why no custom `CoinRecord`?** The `CoinState` type from `chia-protocol` already carries `coin: Coin`, `created_height: Option<u32>`, and `spent_height: Option<u32>`. This is sufficient for all validation lookups. Using it directly ensures wire-level compatibility with Chia peer protocol messages.

**Why no custom `Condition` enum?** The `Condition` enum from `chia-sdk-types` already covers all 43 CLVM condition opcodes. Using it directly means dig-block automatically supports new conditions added to the Chia ecosystem without code changes.

### 7.3 Tier 1: Structural Validation

Structural validation checks internal consistency with **no external state**. This is the `validate_structure()` method described in Section 5.2. It covers:

- Header field constraints (version, cost, size, timestamp, DFSP roots)
- Body-to-header agreement (spend_bundle_count, additions_count, removals_count, slash_proposal_count)
- Merkle root recomputation (spends_root, additions_root, removals_root, filter_hash, slash_proposals_root)
- Duplicate output detection and within-block double-spend detection
- Size limit enforcement

**API:** `L2Block::validate_structure(&self) -> Result<(), BlockError>`

**External state required:** None.

### 7.4 Tier 2: Execution Validation

Execution validation runs each SpendBundle through `dig-clvm`, which handles CLVM puzzle execution, condition parsing, BLS signature verification, and conservation checks. It does **not** require coin lookups (coin existence is deferred to Tier 3).

**API:** `L2Block::validate_execution(&self, clvm_config: &ValidationConfig, genesis_challenge: &Bytes32) -> Result<ExecutionResult, BlockError>`

The `ValidationConfig` (from `dig-clvm`) carries cost limits and validation flags. Internally, this creates a `clvmr::Allocator` and calls `dig_clvm::validate_spend_bundle()` per SpendBundle.

#### 7.4.1 Per-SpendBundle Execution Pipeline

For each `SpendBundle` in `spend_bundles`, in block order:

```
┌─────────────────────────────────────────────────────────┐
│         SpendBundle Execution Pipeline                   │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  For each CoinSpend in bundle:                           │
│  ├─ 1. Verify puzzle_hash(puzzle_reveal) == coin.puzzle_hash
│  ├─ 2. Validate bundle: dig_clvm::validate_spend_bundle()
│  ├─ 3. Parse conditions from CLVM output                │
│  └─ 4. Accumulate cost                                  │
│                                                          │
│  Across all CoinSpends in bundle:                        │
│  ├─ 5. Resolve announcements (create → assert pairs)    │
│  ├─ 6. Validate all assertion conditions                │
│  ├─ 7. Collect CREATE_COIN outputs                      │
│  ├─ 8. Collect signature requirements                   │
│  └─ 9. Verify BLS aggregate signature                   │
│                                                          │
│  After all CoinSpends:                                   │
│  ├─ 10. Verify coin conservation (no minting)           │
│  └─ 11. Verify fee matches (input - output)             │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

#### 7.4.2 Puzzle Hash Verification

For each `CoinSpend`, the tree hash of `puzzle_reveal` must equal `coin.puzzle_hash`. This uses `clvm-utils::tree_hash()` (the Chia standard `sha256tree` operation) to ensure the puzzle being executed is the one committed to when the coin was created.

```rust
use clvm_utils::tree_hash;

// tree_hash computes sha256tree(puzzle_reveal) — the standard Chia puzzle hash
let computed = tree_hash(allocator, puzzle_reveal_ptr);
if computed != coin_spend.coin.puzzle_hash {
    return Err(BlockError::PuzzleHashMismatch { ... });
}
```

Chia parity: [`block_body_validation.py` Check 20](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py), `WRONG_PUZZLE_HASH`.

#### 7.4.3 CLVM Execution

Each SpendBundle is validated by calling `dig_clvm::validate_spend_bundle()`, which internally calls `chia-consensus::run_spendbundle()` to execute all puzzles in the bundle:

```rust
use dig_clvm::{validate_spend_bundle, ValidationContext, ValidationConfig, SpendResult};

// Build validation context with chain state for this bundle
let context = ValidationContext {
    height: self.header.height as u32,
    timestamp: self.header.timestamp,
    constants: network_constants.clone(),
    coin_records: coin_records_for_bundle,
    ephemeral_coins: ephemeral_coins_from_earlier_bundles,
};

// validate_spend_bundle handles:
//   1. CLVM execution of all puzzles (via chia-consensus::run_spendbundle)
//   2. Condition parsing into OwnedSpendBundleConditions
//   3. BLS aggregate signature verification (with optional BlsCache)
//   4. Conservation check (total_input >= total_output)
//   5. Cost limit enforcement
let spend_result: SpendResult = validate_spend_bundle(
    &bundle, &context, &clvm_config, bls_cache.as_deref_mut()
)?;

// spend_result contains:
//   .fee: u64           — computed fee (input - output)
//   .conditions: OwnedSpendBundleConditions — all parsed conditions
//   .additions: Vec<Coin>   — coins created by CREATE_COIN
//   .removals: Vec<Bytes32> — coin IDs spent
```

`dig-clvm` delegates to `chia-consensus::spendbundle_conditions::run_spendbundle()` for CLVM execution, which uses `clvmr::Allocator` and `clvmr::ChiaDialect` internally. The `OwnedSpendBundleConditions` type (from `chia-consensus`) carries the complete parsed output: per-spend conditions, aggregate signature pairs, cost, fees, and all assertion data.

If execution fails (invalid puzzle, cost exceeded, runtime error, invalid signature), `dig-clvm` returns a `ValidationError` which dig-block maps to the appropriate `BlockError` variant.

Chia parity: Uses the same `chia-consensus` and `clvmr` runtime as Chia L1, wrapped by `dig-clvm` with DIG-specific cost limits (550B vs Chia's 11B).

#### 7.4.4 Condition Validation

Conditions are parsed from the CLVM output and validated in two passes:

**Pass 1 — Collect announcements and outputs:**

| Condition | Action |
|-----------|--------|
| `CREATE_COIN(puzzle_hash, amount)` | Add to outputs. Compute child coin ID = `sha256(parent_coin_id \|\| puzzle_hash \|\| amount)`. |
| `CREATE_COIN_ANNOUNCEMENT(message)` | Compute `sha256(coin_id \|\| message)`, add to coin announcement set. |
| `CREATE_PUZZLE_ANNOUNCEMENT(message)` | Compute `sha256(puzzle_hash \|\| message)`, add to puzzle announcement set. |
| `RESERVE_FEE(amount)` | Accumulate minimum fee requirement. |

**Pass 2 — Validate assertions:**

| Condition | Validation |
|-----------|-----------|
| `ASSERT_COIN_ANNOUNCEMENT(hash)` | Must exist in coin announcement set. |
| `ASSERT_PUZZLE_ANNOUNCEMENT(hash)` | Must exist in puzzle announcement set. |
| `ASSERT_CONCURRENT_SPEND(coin_id)` | Coin must be spent in the same SpendBundle. |
| `ASSERT_CONCURRENT_PUZZLE(puzzle_hash)` | Puzzle hash must be spent in the same SpendBundle. |
| `ASSERT_MY_COIN_ID(id)` | Must equal the CoinSpend's coin ID. |
| `ASSERT_MY_PARENT_ID(id)` | Must equal the CoinSpend's coin parent_coin_info. |
| `ASSERT_MY_PUZZLEHASH(hash)` | Must equal the CoinSpend's coin puzzle_hash. |
| `ASSERT_MY_AMOUNT(amount)` | Must equal the CoinSpend's coin amount. |
| `ASSERT_EPHEMERAL` | Coin must be created in the same block (checked in Tier 3). |

**Height/time assertions** (`ASSERT_HEIGHT_ABSOLUTE`, `ASSERT_HEIGHT_RELATIVE`, `ASSERT_SECONDS_ABSOLUTE`, `ASSERT_SECONDS_RELATIVE`, and their `BEFORE_*` counterparts) are **collected** during Tier 2 but **evaluated** in Tier 3, since they require chain context.

All conditions are represented by the `chia-sdk-types::Condition` enum — dig-block does **not** define its own condition types. Condition opcodes and semantics are inherited directly from the Chia crate, which matches [`condition_opcodes.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/types/condition_opcodes.py). Validation logic matches [`block_body_validation.py` Checks 8-22](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py).

#### 7.4.5 Signature Verification

BLS aggregate signature verification is handled **inside** `dig_clvm::validate_spend_bundle()` — dig-block does not perform signature verification separately. When `ValidationConfig` does not include the `DONT_VALIDATE_SIGNATURE` flag, `dig-clvm` calls `chia-consensus::validate_clvm_and_signature()` which:

1. Extracts all AGG_SIG requirements (AGG_SIG_ME, AGG_SIG_UNSAFE, AGG_SIG_PARENT, AGG_SIG_PUZZLE, AGG_SIG_AMOUNT, and compound variants) from the parsed conditions.
2. Constructs the correct message bytes for each variant using `chia-consensus::make_aggsig_final_message()`.
3. Calls `chia-bls::aggregate_verify()` over all `(public_key, message)` pairs against `bundle.aggregated_signature`.

If verification fails, `dig-clvm` returns `ValidationError::SignatureFailed`, which dig-block maps to `BlockError::SignatureFailed`.

`dig-clvm` also supports an optional `BlsCache` for amortizing pairing computations across bundles within a block, which is important for block validation performance.

Chia parity: Uses the same `chia-consensus` signature validation path as Chia L1. AGG_SIG semantics match [`block_body_validation.py` Check 22](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py).

#### 7.4.6 Conservation and Fee Verification

**Per-bundle conservation** is checked inside `dig_clvm::validate_spend_bundle()` — it rejects any bundle where `total_input < total_output` with `ValidationError::ConservationViolation`. The per-bundle fee is returned in `SpendResult.fee`.

**Block-level aggregation** is performed by dig-block after all bundles pass:

```
computed_total_fees = sum of all SpendResult.fee values
computed_total_cost = sum of all SpendResult.conditions.cost values
```

- **Fee consistency:** `computed_total_fees == header.total_fees`. Reject with `BlockError::FeesMismatch` if violated.
- **Cost consistency:** `computed_total_cost == header.total_cost`. Reject with `BlockError::CostMismatch` if violated.
- **Reserve fee:** Each bundle's reserve fee is checked by `dig-clvm` internally.

Chia parity: Per-bundle conservation matches [`block_body_validation.py` Check 16](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L431), `MINTING_COIN`. Fee check matches Check 19, `INVALID_BLOCK_FEE_AMOUNT`. Cost check matches Check 9, `INVALID_BLOCK_COST`.

#### 7.4.7 ExecutionResult

Tier 2 returns an `ExecutionResult` that carries validated outputs for use in Tier 3:

```rust
pub struct ExecutionResult {
    /// All coins created across all SpendBundles, in block order.
    pub additions: Vec<Coin>,
    /// All coin IDs spent across all SpendBundles, in block order.
    pub removals: Vec<Bytes32>,
    /// Collected height/time assertions to evaluate in Tier 3.
    pub pending_assertions: Vec<PendingAssertion>,
    /// Total validated CLVM cost.
    pub total_cost: Cost,
    /// Total validated fees.
    pub total_fees: u64,
    /// Per-SpendBundle receipts.
    pub receipts: Vec<Receipt>,
}
```

### 7.5 Tier 3: State Validation

State validation checks the block against the current chain state. This requires a `CoinLookup` implementation and the `ExecutionResult` from Tier 2.

**API:** `L2Block::validate_state(&self, exec: &ExecutionResult, coins: &dyn CoinLookup, proposer_pubkey: &PublicKey) -> Result<Bytes32, BlockError>`

Returns the computed `state_root` on success. Coin lookups return `chia-protocol::CoinState` (the same type used in Chia's peer protocol for `register_for_coin_updates` responses).

#### 7.5.1 Coin Existence

For each removal (spent coin ID):

1. Look up the coin via `coins.get_coin_state(coin_id)`.
2. **Must exist:** Reject with `BlockError::CoinNotFound` if `None`, unless the coin is **ephemeral** (created in the same block — look for it in `exec.additions`).
3. **Must be unspent:** Reject with `BlockError::CoinAlreadySpent` if `coin_state.spent_height.is_some()`.

Chia parity: [`block_body_validation.py` Check 15](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py), `UNKNOWN_UNSPENT` and `DOUBLE_SPEND`.

#### 7.5.2 Puzzle Hash Cross-Check

For each removal, the puzzle hash from the coin state must match the puzzle hash in the CoinSpend:

```
coins.get_coin_state(coin_id).coin.puzzle_hash == coin_spend.coin.puzzle_hash
```

Reject with `BlockError::PuzzleHashMismatch` if they differ. The `CoinState.coin` field (from `chia-protocol`) carries the full `Coin` including `puzzle_hash`.

Chia parity: [`block_body_validation.py` Check 20](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py), `WRONG_PUZZLE_HASH`.

#### 7.5.3 Addition Non-Existence

For each addition (created coin ID), the coin must **not** already exist in the coin set (unless it's ephemeral and spent in the same block):

```
coins.get_coin_state(addition.coin_id()) must be None
```

Reject with `BlockError::CoinAlreadyExists` if found.

#### 7.5.4 Height/Time Lock Evaluation

Pending assertions from Tier 2 are now evaluated against chain state:

| Assertion | Evaluation |
|-----------|-----------|
| `ASSERT_HEIGHT_ABSOLUTE(h)` | `coins.get_chain_height() >= h` |
| `ASSERT_HEIGHT_RELATIVE(h)` | `coins.get_chain_height() >= coin_confirmed_height + h` |
| `ASSERT_SECONDS_ABSOLUTE(t)` | `coins.get_chain_timestamp() >= t` |
| `ASSERT_SECONDS_RELATIVE(t)` | `coins.get_chain_timestamp() >= coin_timestamp + t` |
| `ASSERT_BEFORE_HEIGHT_ABSOLUTE(h)` | `coins.get_chain_height() < h` |
| `ASSERT_BEFORE_HEIGHT_RELATIVE(h)` | `coins.get_chain_height() < coin_confirmed_height + h` |
| `ASSERT_BEFORE_SECONDS_ABSOLUTE(t)` | `coins.get_chain_timestamp() < t` |
| `ASSERT_BEFORE_SECONDS_RELATIVE(t)` | `coins.get_chain_timestamp() < coin_timestamp + t` |

Reject with `BlockError::AssertionFailed` if any assertion does not hold.

Chia parity: [`block_body_validation.py` Check 21](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py). Height/time assertion semantics match Chia's condition evaluation.

#### 7.5.5 Proposer Signature Verification

The proposer's BLS signature over the header hash is verified:

```
verify(proposer_pubkey, header.hash(), block.proposer_signature)
```

Reject with `BlockError::InvalidProposerSignature` if verification fails.

#### 7.5.6 State Root Verification

The final step applies all additions and removals to compute the expected state root:

```
1. Start with the parent block's state root (implied by CoinLookup state)
2. For each removal: mark coin as spent at this height
3. For each addition: insert coin as unspent at this height
4. Recompute Merkle root over the modified coin set
5. Compare computed_state_root with header.state_root
```

Reject with `BlockError::InvalidStateRoot` if they differ. Return the computed state root on success.

### 7.6 Full Validation Error Types (Additions)

The following error variants are added to `BlockError` for Tier 2 and Tier 3 validation:

```rust
// ── Tier 2: Execution errors ──

/// Puzzle reveal hash does not match the coin's puzzle_hash.
/// Chia parity: block_body_validation.py Check 20, WRONG_PUZZLE_HASH.
PuzzleHashMismatch { coin_id: Bytes32, expected: Bytes32, computed: Bytes32 },

/// CLVM execution failed for a CoinSpend.
ClvmExecutionFailed { coin_id: Bytes32, reason: String },

/// CLVM execution cost exceeded remaining budget.
ClvmCostExceeded { coin_id: Bytes32, cost: Cost, remaining: Cost },

/// An assertion condition was not satisfied.
AssertionFailed { condition: String, reason: String },

/// An announcement was asserted but never created.
AnnouncementNotFound { announcement_hash: Bytes32 },

/// BLS aggregate signature verification failed for a SpendBundle.
/// Chia parity: block_body_validation.py Check 22, BAD_AGGREGATE_SIGNATURE.
SignatureFailed { bundle_index: usize },

/// Total removed value < total added value (coin minting).
/// Chia parity: block_body_validation.py Check 16, MINTING_COIN.
CoinMinting { removed: u64, added: u64 },

/// Computed fees do not match header.total_fees.
/// Chia parity: block_body_validation.py Check 19, INVALID_BLOCK_FEE_AMOUNT.
FeesMismatch { header: u64, computed: u64 },

/// Fees do not meet the RESERVE_FEE condition sum.
/// Chia parity: block_body_validation.py Check 17, RESERVE_FEE_CONDITION_FAILED.
ReserveFeeFailed { required: u64, actual: u64 },

/// Computed total cost does not match header.total_cost.
/// Chia parity: block_body_validation.py Check 9, INVALID_BLOCK_COST.
CostMismatch { header: Cost, computed: Cost },

// ── Tier 3: State errors ──

/// Coin being spent does not exist in the coin set.
/// Chia parity: block_body_validation.py Check 15, UNKNOWN_UNSPENT.
CoinNotFound { coin_id: Bytes32 },

/// Coin being spent has already been spent.
/// Chia parity: block_body_validation.py Check 15, DOUBLE_SPEND.
CoinAlreadySpent { coin_id: Bytes32, spent_height: u64 },

/// Coin being created already exists in the coin set.
CoinAlreadyExists { coin_id: Bytes32 },
```

## 8. Serialization

### 8.1 Format

All block types use **bincode** for serialization. Bincode produces compact binary output with no schema overhead, suitable for both storage and network transmission.

| Type | Serialization | Notes |
|------|--------------|-------|
| `L2BlockHeader` | bincode | Fixed structure, deterministic output. |
| `L2Block` | bincode | Variable size due to SpendBundles and slash proposals. |
| `AttestedBlock` | bincode | Wraps L2Block + attestation data. |
| `Checkpoint` | bincode | Fixed structure. |
| `CheckpointSubmission` | bincode | Wraps Checkpoint + signing data. |

### 8.2 Conventions

- **`to_bytes()`** — infallible. Panics on serialization failure (should never happen with well-formed types).
- **`from_bytes()`** — fallible. Returns the appropriate error type (`BlockError::InvalidData` or `CheckpointError::InvalidData`) on deserialization failure.
- **Serde attributes**: Fields added in later protocol versions use `#[serde(default)]` or `#[serde(default = "...")]` to maintain backwards compatibility with older serialized data.

### 8.3 Genesis Block

The genesis block is constructed via `L2BlockHeader::genesis(network_id, l1_height, l1_hash)`:

- `version`: Auto-detected from height 0 (always `VERSION_V1` unless DFSP activates at height 0).
- `height`: 0.
- `epoch`: 0.
- `parent_hash`: Set to `network_id` (the network's unique identifier, not a real parent block hash).
- All Merkle roots (`state_root`, `spends_root`, `additions_root`, `removals_root`, `receipts_root`, `slash_proposals_root`, DFSP roots): `EMPTY_ROOT`.
- `filter_hash`: `EMPTY_ROOT` (`0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`) — no transactions to filter.
- `extension_data`: `ZERO_HASH` (`0x0000000000000000000000000000000000000000000000000000000000000000`).
- All counts and costs: 0.
- All L1 proof anchors: `None`.
- `timestamp`: Current wall-clock time (set at construction).

## 9. Block Lifecycle

This section shows how production (Section 6) and validation (Sections 5, 7) fit into the full lifecycle. Status transitions are enforced by the consensus layer, not this crate.

### 9.1 Regular Block Lifecycle

```
 Mempool provides SpendBundles
         │
         ▼
 ┌─────────────────────────┐
 │  BlockBuilder.build()    │  ← dig-block (Section 6)
 │  (computes all roots,    │
 │   signs header)          │
 └───────┬─────────────────┘
         │
         ▼
 ┌─────────────────────────┐
 │  Tier 1: Structural      │  ← dig-block (Section 5)
 │  validate_structure()    │
 └───────┬─────────────────┘
         │
         ▼
 ┌─────────────────────────┐
 │  Tier 2: Execution       │  ← dig-block (Section 7.4)
 │  validate_execution()    │     requires Allocator
 └───────┬─────────────────┘
         │
         ▼
 ┌─────────────────────────┐
 │  Tier 3: State           │  ← dig-block (Section 7.5)
 │  validate_state()        │     requires CoinLookup
 └───────┬─────────────────┘
         │
         ▼
 ┌─────────────────────────┐
 │  AttestedBlock           │  ← consensus layer
 │  Pending → Validated     │
 └───────┬─────────────────┘
         │ validators attest
         ▼
 ┌─────────────────────────┐
 │  SoftFinalized           │  ← >67% stake signed
 └───────┬─────────────────┘
         │ checkpoint confirmed on L1
         ▼
 ┌─────────────────────────┐
 │  HardFinalized           │  ← L1 checkpoint confirmed
 └─────────────────────────┘
```

### 9.2 Checkpoint Lifecycle

```
 Epoch ends
    │
    ▼
 ┌──────────────────────────┐
 │  CheckpointBuilder.build()│  ← dig-block (Section 6.6)
 └───────┬──────────────────┘
         │
         ▼
 ┌──────────────────────────┐
 │  CheckpointSubmission     │  ← consensus layer
 │  (signer_bitmap, score)   │     aggregate BLS signature
 └───────┬──────────────────┘
         │ submitted to L1
         ▼
 ┌──────────────────┐
 │  L1 Finalization  │  ← epoch finalizer puzzle
 └──────────────────┘
```

## 10. Public API Summary

### 10.1 Construction

| Function | Input | Output |
|----------|-------|--------|
| `L2BlockHeader::new()` | Core header fields | `L2BlockHeader` |
| `L2BlockHeader::new_with_collateral()` | Core fields + collateral | `L2BlockHeader` |
| `L2BlockHeader::new_with_l1_proofs()` | Core fields + all L1 proofs | `L2BlockHeader` |
| `L2BlockHeader::genesis()` | `network_id`, `l1_height`, `l1_hash` | `L2BlockHeader` |
| `L2Block::new()` | `header`, `spend_bundles`, `proposer_signature` | `L2Block` |
| `AttestedBlock::new()` | `block`, `validator_count`, `receipts` | `AttestedBlock` |
| `Checkpoint::new()` | All checkpoint fields | `Checkpoint` |
| `CheckpointSubmission::new()` | `checkpoint`, signing data, `score`, `submitter` | `CheckpointSubmission` |
| `SignerBitmap::new()` | `validator_count` | `SignerBitmap` |
| `ReceiptList::new()` | (none) | Empty `ReceiptList` |
| `ReceiptList::from_receipts()` | `Vec<Receipt>` | `ReceiptList` with computed root |

### 10.2 Hashing

| Function | Input | Output |
|----------|-------|--------|
| `L2BlockHeader::hash()` | `&self` | `Bytes32` |
| `L2Block::hash()` | `&self` | `Bytes32` (delegates to header) |
| `AttestedBlock::hash()` | `&self` | `Bytes32` (delegates to block) |
| `Checkpoint::hash()` | `&self` | `Bytes32` |
| `CheckpointSubmission::hash()` | `&self` | `Bytes32` (delegates to checkpoint) |

### 10.3 Validation

| Function | Input | Output | Tier | Chia crates used |
|----------|-------|--------|------|-----------------|
| `L2BlockHeader::validate()` | `&self` | `Result<(), BlockError>` | 1 | — |
| `L2Block::validate_structure()` | `&self` | `Result<(), BlockError>` | 1 | `chia-consensus::compute_merkle_set_root`, `chia-sdk-types::MerkleTree` |
| `L2Block::validate_execution()` | `&self, &ValidationConfig, &Bytes32` | `Result<ExecutionResult, BlockError>` | 2 | `dig-clvm::validate_spend_bundle` (wraps chia-consensus, chia-bls, clvmr) |
| `L2Block::validate_state()` | `&self, &ExecutionResult, &dyn CoinLookup, &PublicKey` | `Result<Bytes32, BlockError>` | 3 | `chia-bls::verify` |
| `L2Block::validate_full()` | `&self, &ValidationConfig, &dyn CoinLookup, &Bytes32, &PublicKey` | `Result<Bytes32, BlockError>` | 1+2+3 | All of the above |

### 10.4 Serialization

| Function | Input | Output |
|----------|-------|--------|
| `L2BlockHeader::to_bytes()` / `from_bytes()` | `&self` / `&[u8]` | `Vec<u8>` / `Result<Self, BlockError>` |
| `L2Block::to_bytes()` / `from_bytes()` | `&self` / `&[u8]` | `Vec<u8>` / `Result<Self, BlockError>` |
| `Checkpoint::to_bytes()` / `from_bytes()` | `&self` / `&[u8]` | `Vec<u8>` / `Result<Self, CheckpointError>` |
| `CheckpointSubmission::to_bytes()` / `from_bytes()` | `&self` / `&[u8]` | `Vec<u8>` / `Result<Self, CheckpointError>` |

### 10.5 Block Production

| Function | Input | Output |
|----------|-------|--------|
| `BlockBuilder::new()` | `height, epoch, parent_hash, l1_height, l1_hash, proposer_index` | `BlockBuilder` |
| `BlockBuilder::add_spend_bundle()` | `&mut self, SpendBundle, Cost, u64` | `Result<(), BuilderError>` |
| `BlockBuilder::add_slash_proposal()` | `&mut self, Vec<u8>` | `Result<(), BuilderError>` |
| `BlockBuilder::set_l1_proofs()` | `&mut self, ...Option<Bytes32>` | `()` |
| `BlockBuilder::set_dfsp_roots()` | `&mut self, ...Bytes32` | `()` |
| `BlockBuilder::remaining_cost()` | `&self` | `Cost` |
| `BlockBuilder::build()` | `self, Bytes32, Bytes32, &dyn BlockSigner` | `Result<L2Block, BuilderError>` |
| `CheckpointBuilder::new()` | `epoch, prev_checkpoint` | `CheckpointBuilder` |
| `CheckpointBuilder::add_block()` | `&mut self, Bytes32, u64, u64` | `()` |
| `CheckpointBuilder::build()` | `self` | `Checkpoint` |

### 10.6 Block Helpers

| Function | Input | Output | Chia Source |
|----------|-------|--------|-------------|
| `L2Block::compute_spends_root()` | `&self` | `Bytes32` | — |
| `L2Block::compute_additions_root()` | `&self` | `Bytes32` | [`block_body_validation.py:158-175`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L158) |
| `L2Block::compute_removals_root()` | `&self` | `Bytes32` | [`block_body_validation.py:185`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L185) |
| `L2Block::compute_filter_hash()` | `&self` | `Bytes32` | [`block_creation.py:199-210`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py#L199) |
| `L2Block::compute_slash_proposals_root()` | `&[Vec<u8>]` | `Bytes32` | — |
| `L2Block::slash_proposal_leaf_hash()` | `&[u8]` | `Bytes32` | — |
| `L2Block::all_additions()` | `&self` | `Vec<Coin>` | — |
| `L2Block::all_addition_ids()` | `&self` | `Vec<CoinId>` | — |
| `L2Block::all_removals()` | `&self` | `Vec<CoinId>` | — |
| `L2Block::has_duplicate_outputs()` | `&self` | `Option<Bytes32>` | [`block_body_validation.py` Check 13](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L396) |
| `L2Block::has_double_spends()` | `&self` | `Option<Bytes32>` | [`block_body_validation.py` Check 14](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L400) |
| `L2Block::compute_size()` | `&self` | `usize` | — |
| `L2Block::height()` | `&self` | `u64` | — |
| `L2Block::epoch()` | `&self` | `u64` | — |

### 10.7 Checkpoint Helpers

| Function | Input | Output |
|----------|-------|--------|
| `Checkpoint::compute_score()` | `&self, stake_percentage: u64` | `u64` |
| `CheckpointSubmission::signing_percentage()` | `&self` | `u64` |
| `CheckpointSubmission::meets_threshold()` | `&self, threshold_pct: u64` | `bool` |
| `CheckpointSubmission::record_submission()` | `&mut self, height: u32, coin_id: Bytes32` | `()` |
| `CheckpointSubmission::is_submitted()` | `&self` | `bool` |

### 10.8 Attestation Helpers

| Function | Input | Output |
|----------|-------|--------|
| `AttestedBlock::signing_percentage()` | `&self` | `u64` |
| `AttestedBlock::has_soft_finality()` | `&self, threshold_pct: u64` | `bool` |
| `BlockStatus::is_finalized()` | `&self` | `bool` |
| `BlockStatus::is_canonical()` | `&self` | `bool` |

## 11. Crate Boundary

### 11.1 What This Crate Owns

| Concern | Owned by `dig-block` | Chia crates used |
|---------|---------------------|-----------------|
| Block type definitions (L2BlockHeader, L2Block, AttestedBlock) | Yes | `chia-protocol` (Bytes32, SpendBundle, Coin) |
| Checkpoint type definitions (Checkpoint, CheckpointSubmission) | Yes | `chia-protocol` (Bytes32), `chia-bls` (Signature, PublicKey) |
| Block and checkpoint hashing algorithms | Yes | `chia-sha2` (Sha256) |
| Block production (BlockBuilder, CheckpointBuilder) | Yes | `chia-consensus` (compute_merkle_set_root), `chia-sdk-types` (MerkleTree) |
| Full validation pipeline (structural + execution + state) | Yes | `dig-clvm` (validate_spend_bundle), `chia-consensus` (compute_merkle_set_root), `chia-sdk-types` (Condition, MerkleTree), `clvm-utils` (tree_hash) |
| Validation context trait (CoinLookup) | Yes | `chia-protocol` (CoinState) as return type |
| Block signer trait (BlockSigner) | Yes | `chia-bls` (Signature) as return type |
| Serialization format (bincode + Streamable interop) | Yes | `chia-traits` (Streamable) for wire format |
| Block-level constants (MAX_BLOCK_SIZE, MAX_COST_PER_BLOCK, etc.) | Yes | `chia-sdk-types` (MAINNET_CONSTANTS) for reference values |
| Supporting types (BlockStatus, CheckpointStatus, Receipt, ReceiptList, SignerBitmap) | Yes | — |
| Error types (BlockError, CheckpointError, BuilderError, SignerBitmapError, ReceiptError) | Yes | — |

### 11.2 What This Crate Does NOT Own

| Concern | Owned by | Notes |
|---------|----------|-------|
| Storing blocks on disk | Chain store crate | — |
| Maintaining a chain of blocks (parent linkage, fork tracking) | Chain manager | — |
| Global CoinSet state (database, rollback, queries) | `dig-coinstore` | Provides `CoinLookup` impl returning `chia-protocol::CoinState` |
| CLVM execution engine | `dig-clvm` crate (DIG) | Wraps `chia-consensus::run_spendbundle()` with DIG-specific cost limits and BLS caching. `dig-mempool` uses the same crate. |
| Condition type definitions | `chia-sdk-types` crate (Chia) | `Condition` enum with 43 variants — used directly, not redefined |
| BLS signature verification | `chia-bls` / `chia-consensus` (via `dig-clvm`) | `aggregate_verify()` and `validate_clvm_and_signature()` — handled inside `dig-clvm` |
| Merkle set root computation | `chia-consensus` crate (Chia) | `compute_merkle_set_root()` — used directly for additions/removals roots |
| Transaction selection and fee prioritization | Mempool layer | Feeds SpendBundles to `BlockBuilder` |
| Validator set management and attestation pooling | Consensus layer | — |
| Checkpoint competition (managing multiple submissions) | Consensus layer | — |
| Network transmission of blocks | Networking layer | — |

### 11.3 Dependency Direction

```
dig-block  (this crate — types, building, validation)
    │
    │  ┌─── DIG ecosystem ─────────────────────────────────┐
    ├──► dig-clvm          (validate_spend_bundle, ValidationContext, ValidationConfig, SpendResult)
    │  └───────────────────────────────────────────────────┘
    │
    │  ┌─── Chia ecosystem (used directly for types and Merkle) ───┐
    ├──► chia-protocol    (Bytes32, SpendBundle, CoinSpend, Coin, CoinState, Program)
    ├──► chia-bls         (Signature, PublicKey, sign, verify)
    ├──► chia-consensus   (compute_merkle_set_root)
    ├──► chia-sdk-types   (Condition, MerkleTree, MerkleProof, MAINNET_CONSTANTS)
    ├──► chia-sdk-signer  (AggSigConstants — passed to dig-clvm)
    ├──► chia-sha2        (Sha256)
    ├──► chia-traits       (Streamable)
    ├──► clvm-utils        (tree_hash — puzzle hash verification)
    │  └──────────────────────────────────────────────────────────┘
    │
    ├──► bincode          (internal block serialization)
    ├──► serde            (derive Serialize/Deserialize)
    └──► thiserror        (error derivation)

    dig-clvm  (CLVM execution — transitive deps, NOT direct deps of dig-block)
        ├──► chia-consensus   (run_spendbundle, validate_clvm_and_signature, spendbundle_conditions)
        ├──► chia-bls         (aggregate_verify, BlsCache)
        ├──► clvmr            (Allocator, ChiaDialect, run_program)
        └──► clvm-traits       (FromClvm, ToClvm)

Trait implementations (provided by downstream crates):
    dig-coinstore  ──► implements CoinLookup (returns chia-protocol::CoinState)
    proposer       ──► implements BlockSigner (returns chia-bls::Signature)

Downstream consumers:
    dig-coinstore  ──► dig-block  (reads block additions/removals, provides CoinLookup)
    chain-manager  ──► dig-block  (calls validate_full(), stores blocks)
    consensus      ──► dig-block  (creates AttestedBlocks, CheckpointSubmissions)
    networking     ──► dig-block  (serializes/deserializes for wire)
    proposer       ──► dig-block  (calls BlockBuilder, provides BlockSigner)
    dig-mempool    ──► dig-clvm   (validates SpendBundles on admission — same engine)
```

## 12. Testing Strategy

### 12.1 Unit Tests

| Category | Tests |
|----------|-------|
| **Header construction** | `new()`, `new_with_collateral()`, `new_with_l1_proofs()`, `genesis()` produce valid headers. Auto-version detection at different heights. |
| **Header hashing** | Deterministic (same fields = same hash). Different fields = different hash. All fields participate in hash (toggle each field, verify hash changes). |
| **Header validation** | Version mismatch rejected. DFSP roots before activation rejected. Cost exceeded rejected. Size exceeded rejected. Timestamp too far in future rejected. |
| **Block construction** | `new()` produces valid block. Slash proposals default to empty. |
| **Block structural validation** | SpendBundle count mismatch rejected. Spends root mismatch rejected. Additions/removals root mismatch rejected. Filter hash mismatch rejected. Additions/removals count mismatch rejected. Duplicate output rejected. Double spend in block rejected. Slash proposal count/root/size violations rejected. Oversized block rejected. Valid block passes. |
| **Block helpers** | `compute_spends_root()` matches header. `compute_additions_root()` matches Chia's grouped-by-puzzle_hash Merkle set. `compute_removals_root()` matches Chia's removals Merkle set. `compute_filter_hash()` produces correct BIP158 filter hash. `all_additions()` and `all_removals()` extract correct coin IDs. `has_duplicate_outputs()` detects duplicate coins. `has_double_spends()` detects duplicate removals. `compute_size()` returns correct byte count. |
| **Tagged Merkle hashing** | Leaf hashes use `0x01` prefix. Internal node hashes use `0x02` prefix. Unprefixed data produces different hashes than prefixed. Matches Chia [`merkle_utils.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/wallet/util/merkle_utils.py) reference implementation. |
| **Checkpoint hashing** | Deterministic. Different epochs produce different hashes. |
| **Checkpoint score** | `compute_score(stake_pct)` = `stake_pct * block_count`. |
| **Checkpoint serialization** | Round-trip: `from_bytes(to_bytes()) == original`. |
| **SignerBitmap** | `set_signed()` / `has_signed()` correctness. `signer_count()` and `signing_percentage()` accuracy. `merge()` produces bitwise OR. Out-of-bounds index rejected. |
| **ReceiptList** | `from_receipts()` computes correct Merkle root. `push()` + `finalize()` matches `from_receipts()`. `get_by_tx_id()` finds correct receipt. `success_count()` / `failure_count()` / `total_fees()` accuracy. |
| **BlockStatus** | `is_finalized()` true for SoftFinalized and HardFinalized, false otherwise. `is_canonical()` false for Orphaned and Rejected. |
| **Serialization round-trips** | All types survive `to_bytes()` -> `from_bytes()` without data loss. |

### 12.2 Block Production Tests

| Category | Tests |
|----------|-------|
| **BlockBuilder basic** | Create builder, add SpendBundles, build block. Verify all derived header fields are correct (all roots, counts, cost, fees, filter_hash). Verify `validate_structure()` passes on the built block. |
| **BlockBuilder cost limit** | Add SpendBundles until cost budget is exhausted. Verify `add_spend_bundle()` returns `CostBudgetExceeded` on the bundle that would exceed the limit. Verify `remaining_cost()` is accurate throughout. |
| **BlockBuilder size limit** | Add SpendBundles until size budget is exhausted. Verify `SizeBudgetExceeded`. |
| **BlockBuilder slash proposals** | Add slash proposals up to `MAX_SLASH_PROPOSALS_PER_BLOCK`. Verify rejection on exceeding count. Verify rejection on oversized payload. Verify `slash_proposals_root` is correct in built block. |
| **BlockBuilder DFSP roots** | Build block at DFSP activation height without setting roots — verify `MissingDfspRoots`. Set roots, build again — verify success. |
| **BlockBuilder signing** | Provide mock `BlockSigner`, verify `proposer_signature` in built block matches `sign_block(header.hash())`. |
| **BlockBuilder empty** | Build with zero SpendBundles — verify the result has correct empty-state roots (`EMPTY_ROOT` for spends, additions, removals, filter). |
| **CheckpointBuilder** | Add blocks, set state root, build checkpoint. Verify `block_root` Merkle tree is correct. Verify `block_count`, `tx_count`, `total_fees` are accumulated correctly. |
| **Build-then-validate round-trip** | Build a block via `BlockBuilder`, then run `validate_structure()` on it. Must always pass — the builder guarantees structural validity by construction. |

### 12.3 Execution Validation Tests (Tier 2)

| Category | Tests |
|----------|-------|
| **Puzzle hash verification** | CoinSpend with wrong puzzle_hash rejected with `PuzzleHashMismatch`. Correct puzzle_hash passes. |
| **CLVM execution** | Use `dig-clvm::validate_spend_bundle()` with test puzzles. Verify `SpendResult` contains correct additions, removals, fees, and conditions. |
| **CLVM cost exceeded** | Mock runner returns cost exceeding remaining budget. Verify `ClvmCostExceeded`. |
| **Announcement resolution** | Bundle with `CREATE_COIN_ANNOUNCEMENT` + `ASSERT_COIN_ANNOUNCEMENT`. Verify pass when paired, fail when assertion has no matching creation. |
| **Concurrent spend assertion** | `ASSERT_CONCURRENT_SPEND` with coin in same bundle passes; coin in different bundle fails. |
| **Self-assertions** | `ASSERT_MY_COIN_ID`, `ASSERT_MY_PARENT_ID`, `ASSERT_MY_PUZZLEHASH`, `ASSERT_MY_AMOUNT` with correct and incorrect values. |
| **Signature verification** | Bundle with valid aggregate signature passes. Invalid signature rejected with `SignatureFailed`. |
| **Conservation** | Bundle where `total_input < total_output` rejected with `CoinMinting`. Equal or greater passes. |
| **Fee consistency** | Computed fees != header.total_fees rejected with `FeesMismatch`. |
| **Reserve fee** | RESERVE_FEE condition with `amount > computed_fees` rejected with `ReserveFeeFailed`. |
| **Cost consistency** | Computed total cost != header.total_cost rejected with `CostMismatch`. |

### 12.4 State Validation Tests (Tier 3)

| Category | Tests |
|----------|-------|
| **Coin existence** | Removal of non-existent coin rejected with `CoinNotFound`. Removal of existing unspent coin passes. |
| **Coin already spent** | Removal of already-spent coin rejected with `CoinAlreadySpent`. |
| **Ephemeral coins** | Coin created and spent in the same block passes (not found in CoinLookup but found in additions). |
| **Addition non-existence** | Creating a coin that already exists rejected with `CoinAlreadyExists`. |
| **Height locks** | `ASSERT_HEIGHT_ABSOLUTE(h)` passes when chain height >= h, fails when < h. Relative variant tested similarly. `ASSERT_BEFORE_HEIGHT_*` variants tested with inverted logic. |
| **Time locks** | `ASSERT_SECONDS_ABSOLUTE(t)` passes when chain timestamp >= t, fails when < t. Relative and before variants tested. |
| **Proposer signature** | Valid proposer signature passes. Wrong key or corrupted signature rejected with `InvalidProposerSignature`. |
| **State root** | Computed state root matching header passes. Mismatch rejected with `InvalidStateRoot`. |

### 12.5 Property Tests

| Property | Description |
|----------|-------------|
| **Hash uniqueness** | For randomly generated headers, no two distinct headers produce the same hash (probabilistic). |
| **Hash stability** | Serializing a header, deserializing it, and re-hashing produces the same hash as the original. |
| **Validation determinism** | `validate_structure()` returns the same result for the same block bytes, regardless of call count. |
| **Serialization round-trip** | For all block types: `from_bytes(to_bytes(x)) == x`. |
| **Builder produces valid blocks** | For any sequence of valid SpendBundles, `BlockBuilder.build()` followed by `validate_structure()` always passes. |
| **Validation tier ordering** | If Tier 1 fails, Tiers 2 and 3 are not reached. If Tier 2 fails, Tier 3 is not reached. `validate_full()` returns the first failure. |

### 12.6 Integration Tests

| Test | Description |
|------|-------------|
| **Genesis block** | Construct genesis, validate all three tiers, verify hash is deterministic for a given network ID. |
| **Multi-bundle block end-to-end** | Build a block via `BlockBuilder` with multiple SpendBundles. Run `validate_full()` with a real `Allocator` and mock `CoinLookup`. Verify all tiers pass and state root is correct. |
| **Additions root Chia parity** | Construct additions with multiple coins sharing the same puzzle_hash, verify the grouped Merkle set produces the same root as Chia's [`compute_merkle_set_root()`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py#L158). |
| **Duplicate output detection** | Construct a block where two SpendBundles create coins with the same ID. Verify `validate_structure()` rejects with `DuplicateOutput`. |
| **Double spend detection** | Construct a block where two SpendBundles spend the same coin. Verify `validate_structure()` rejects with `DoubleSpendInBlock`. |
| **BIP158 filter round-trip** | Construct a block, compute its filter, verify light client can test membership of known puzzle_hashes and get true; test non-existent puzzle_hashes and get false (with probabilistic allowance for false positives). |
| **Tagged Merkle proof** | Generate a Merkle proof for a leaf, verify it against the root. Verify that swapping leaf/node prefixes invalidates the proof. |
| **Slash proposal block** | Construct a block with slash proposals at the limit, verify root computation and validation passes. Exceed limits, verify rejection. |
| **Attested block** | Construct L2Block, wrap in AttestedBlock, simulate signing via SignerBitmap, verify `has_soft_finality()` triggers at threshold. |
| **Checkpoint round-trip** | Build Checkpoint via `CheckpointBuilder`, wrap in CheckpointSubmission, serialize, deserialize, verify all fields preserved. |
| **Version transition** | Construct blocks at `DFSP_ACTIVATION_HEIGHT - 1` (v1) and `DFSP_ACTIVATION_HEIGHT` (v2), verify version auto-detection and validation behavior across tiers. |
| **Ephemeral coin full validation** | Create a SpendBundle that produces a coin, and another SpendBundle in the same block that spends it. Run `validate_full()`. Verify the ephemeral coin is accepted without being in CoinLookup. |
