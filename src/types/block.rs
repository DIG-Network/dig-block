//! `L2Block` — full L2 block: header, transaction body (`SpendBundle`s), slash proposal payloads, proposer signature.
//!
//! **Requirements:**
//! - [BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md) — struct + `new` / `hash` / `height` / `epoch`
//! - [HSH-003](docs/requirements/domains/hashing/specs/HSH-003.md) — [`crate::compute_spends_root`] (spends Merkle root)
//! - [HSH-004](docs/requirements/domains/hashing/specs/HSH-004.md) — [`crate::compute_additions_root`] (additions Merkle set)
//! - [HSH-005](docs/requirements/domains/hashing/specs/HSH-005.md) — [`crate::compute_removals_root`] (removals Merkle set)
//! - [HSH-006](docs/requirements/domains/hashing/specs/HSH-006.md) — [`crate::compute_filter_hash`] (BIP-158; [`L2Block::compute_filter_hash`] keys SipHash with [`L2BlockHeader::parent_hash`] per SPEC §6.4)
//! - [BLK-004](docs/requirements/domains/block_types/specs/BLK-004.md) — Merkle roots, BIP158 `filter_hash` preimage,
//!   additions/removals collectors, duplicate / double-spend probes, serialized size
//! - [SVL-005](docs/requirements/domains/structural_validation/specs/SVL-005.md) — header/body **count agreement**
//!   ([`L2Block::validate_structure`]; SPEC §5.2 steps 2, 4, 5, 13) before expensive Merkle checks ([SVL-006](docs/requirements/domains/structural_validation/specs/SVL-006.md))
//! - [SER-002](docs/requirements/domains/serialization/specs/SER-002.md) — [`Self::to_bytes`] / [`Self::from_bytes`] (bincode + [`BlockError::InvalidData`](crate::BlockError::InvalidData) on decode)
//! - [SPEC §2.3](docs/resources/SPEC.md), [SPEC §3.3–§3.6](docs/resources/SPEC.md) — body commitments + filter
//!
//! ## Usage
//!
//! Build a block by assembling an [`L2BlockHeader`] (commitments, roots, counts) and the body fields.
//! **Canonical identity** is [`L2Block::hash`] → [`L2BlockHeader::hash`] only; spend bundles and slash
//! bytes are committed via Merkle roots **in the header**, not mixed into this hash (SPEC §2.3 / BLK-003 notes).
//!
//! ## Rationale
//!
//! - **`SpendBundle`** comes from **`chia-protocol`** so CLVM spends match L1/Chia tooling ([BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md)).
//! - **`Signature`** is the **`chia-bls`** type re-exported as [`crate::primitives::Signature`] ([BLK-006](docs/requirements/domains/block_types/specs/BLK-006.md)) so callers import one `dig_block` surface.
//! - **`slash_proposal_payloads`** are `Vec<Vec<u8>>` for opaque slash evidence (encoding evolves independently).

use chia_protocol::{Coin, SpendBundle};
use chia_streamable_macro::Streamable;
use serde::{Deserialize, Serialize};

use super::header::L2BlockHeader;
use crate::error::BlockError;
use crate::merkle_util::{empty_on_additions_err, merkle_tree_root, slash_leaf_hash};
use crate::primitives::{Bytes32, Signature};
use crate::{MAX_BLOCK_SIZE, MAX_SLASH_PROPOSALS_PER_BLOCK, MAX_SLASH_PROPOSAL_PAYLOAD_BYTES};

/// Complete L2 block: header plus body (spend bundles, slash payloads) and proposer attestation.
///
/// See [BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md) and [`SPEC §2.3`](docs/resources/SPEC.md).
/// **Chia [`Streamable`] (wire):** see [`L2BlockHeader`] — gossip uses this encoding; persistence uses bincode + zstd in dig-blockstore.
#[derive(Debug, Clone, Serialize, Deserialize, Streamable)]
pub struct L2Block {
    /// Block header (identity hash, Merkle roots, metadata).
    pub header: L2BlockHeader,
    /// Spend bundles included in this block (`chia-protocol`).
    pub spend_bundles: Vec<SpendBundle>,
    /// Raw slash proposal payloads (count should align with header slash fields when validated).
    pub slash_proposal_payloads: Vec<Vec<u8>>,
    /// BLS signature over the block from the proposer ([`crate::primitives::Signature`] / `chia-bls`).
    pub proposer_signature: Signature,
}

impl L2Block {
    /// Construct a block from all body fields and the header ([BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md) `new()`).
    ///
    /// **Note:** Callers must keep `header` fields (e.g. `spend_bundle_count`, Merkle roots) consistent with
    /// `spend_bundles` / `slash_proposal_payloads`; structural validation is separate (ERR-* / VAL-* requirements).
    pub fn new(
        header: L2BlockHeader,
        spend_bundles: Vec<SpendBundle>,
        slash_proposal_payloads: Vec<Vec<u8>>,
        proposer_signature: Signature,
    ) -> Self {
        Self {
            header,
            spend_bundles,
            slash_proposal_payloads,
            proposer_signature,
        }
    }

    /// Canonical block identity: SHA-256 over the header preimage only ([`L2BlockHeader::hash`], HSH-001 / SPEC §3.1).
    ///
    /// **Delegation:** identical to `self.header.hash()` — required by BLK-003 so light clients and
    /// signers can treat the header hash as the block id without serializing the body.
    #[inline]
    pub fn hash(&self) -> Bytes32 {
        self.header.hash()
    }

    /// Serialize this block (header + body) to **bincode** bytes ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md), SPEC §8.2).
    ///
    /// **Infallible:** Same contract as [`L2BlockHeader::to_bytes`] — well-formed structs serialize; failures are `expect` panics.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("L2Block serialization should never fail")
    }

    /// Deserialize a block from **bincode** bytes ([SER-002](docs/requirements/domains/serialization/specs/SER-002.md)).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BlockError> {
        bincode::deserialize(bytes).map_err(|e| BlockError::InvalidData(e.to_string()))
    }

    /// Block height from the header ([`L2BlockHeader::height`]).
    #[inline]
    pub fn height(&self) -> u64 {
        self.header.height
    }

    /// Epoch from the header ([`L2BlockHeader::epoch`]).
    #[inline]
    pub fn epoch(&self) -> u64 {
        self.header.epoch
    }

    // --- BLK-004: Merkle roots (SPEC §3.3–§3.5) ---

    /// Merkle root over spend-bundle leaf digests in **block order**; empty body → [`crate::EMPTY_ROOT`].
    ///
    /// **Delegation:** [`crate::compute_spends_root`] ([HSH-003](docs/requirements/domains/hashing/specs/HSH-003.md)) —
    /// each leaf is SHA-256 of serialized [`SpendBundle`] bytes; [`chia_sdk_types::MerkleTree`] applies tagged hashing
    /// (HSH-007, SPEC §3.3 `spends_root` row).
    #[must_use]
    pub fn compute_spends_root(&self) -> Bytes32 {
        crate::compute_spends_root(&self.spend_bundles)
    }

    /// Additions Merkle root over [`Self::all_additions`] ([HSH-004](docs/requirements/domains/hashing/specs/HSH-004.md)).
    ///
    /// **Delegation:** [`crate::compute_additions_root`] — `puzzle_hash` groups, `[ph, hash_coin_ids(ids)]` pairs in
    /// first-seen order ([`indexmap::IndexMap`] inside that function), then [`merkle_set_root`] /
    /// [`chia_consensus::merkle_set::compute_merkle_set_root`] ([SPEC §3.4](docs/resources/SPEC.md)).
    #[must_use]
    pub fn compute_additions_root(&self) -> Bytes32 {
        let additions = self.all_additions();
        crate::compute_additions_root(&additions)
    }

    /// Removals Merkle set over all spent coin IDs ([HSH-005](docs/requirements/domains/hashing/specs/HSH-005.md)).
    ///
    /// **Body order:** IDs come from [`Self::all_removals`] (spend-bundle then coin-spend order). **Root:** delegates to
    /// [`crate::compute_removals_root`], which uses [`chia_consensus::merkle_set::compute_merkle_set_root`] — the same
    /// multiset of IDs yields the same root regardless of slice order ([SPEC §3.5](docs/resources/SPEC.md)).
    #[must_use]
    pub fn compute_removals_root(&self) -> Bytes32 {
        let ids = self.all_removals();
        crate::compute_removals_root(&ids)
    }

    /// BIP-158 compact filter hash ([HSH-006](docs/requirements/domains/hashing/specs/HSH-006.md), SPEC §3.6).
    ///
    /// **Delegation:** [`crate::compute_filter_hash`] with body-derived [`Self::all_additions`] /
    /// [`Self::all_removals`] slices.
    ///
    /// **BIP158 key (`block_identity` argument):** [`Self::header`]'s [`L2BlockHeader::parent_hash`] — stable while the
    /// filter field is being filled and matches SPEC §6.4’s `filter_hash = compute_filter_hash(additions, removals)` build
    /// step (no self-referential [`Self::hash`] dependency). SipHash keys are the first 16 bytes of that parent digest
    /// ([`crate::merkle_util::bip158_filter_encoded`]).
    #[must_use]
    pub fn compute_filter_hash(&self) -> Bytes32 {
        let additions = self.all_additions();
        let removals = self.all_removals();
        crate::compute_filter_hash(self.header.parent_hash, &additions, &removals)
    }

    /// Binary Merkle root over slash payload digests (`sha256` each), in payload order.
    #[must_use]
    pub fn compute_slash_proposals_root(&self) -> Bytes32 {
        Self::slash_proposals_root_from(&self.slash_proposal_payloads)
    }

    /// [`Self::compute_slash_proposals_root`] for an explicit payload list (tests, pre-serialized batches).
    #[must_use]
    pub fn slash_proposals_root_from(payloads: &[Vec<u8>]) -> Bytes32 {
        if payloads.is_empty() {
            return merkle_tree_root(&[]);
        }
        let leaves: Vec<Bytes32> = payloads.iter().map(|p| slash_leaf_hash(p)).collect();
        merkle_tree_root(&leaves)
    }

    /// Single slash payload leaf digest (building block for [`Self::compute_slash_proposals_root`]).
    #[must_use]
    pub fn slash_proposal_leaf_hash(payload: &[u8]) -> Bytes32 {
        slash_leaf_hash(payload)
    }

    // --- BLK-004: collections & integrity ---

    /// All `CREATE_COIN` outputs from every spend bundle (CLVM-simulated per [`SpendBundle::additions`]).
    #[must_use]
    pub fn all_additions(&self) -> Vec<Coin> {
        let mut out = Vec::new();
        for sb in &self.spend_bundles {
            out.extend(empty_on_additions_err(sb.additions()));
        }
        out
    }

    /// Coin IDs of every addition in body order (same walk as [`Self::all_additions`]).
    #[must_use]
    pub fn all_addition_ids(&self) -> Vec<Bytes32> {
        self.all_additions()
            .into_iter()
            .map(|c| c.coin_id())
            .collect()
    }

    /// Spent coin IDs (`CoinSpend.coin`) in bundle / spend order.
    #[must_use]
    pub fn all_removals(&self) -> Vec<Bytes32> {
        self.spend_bundles
            .iter()
            .flat_map(|sb| sb.coin_spends.iter().map(|cs| cs.coin.coin_id()))
            .collect()
    }

    /// First duplicate output coin ID in addition set, else `None` (SPEC / Chia duplicate-output check).
    #[must_use]
    pub fn has_duplicate_outputs(&self) -> Option<Bytes32> {
        first_duplicate_addition_coin_id(&self.all_additions())
    }

    /// First coin ID spent twice as a removal, else `None`.
    #[must_use]
    pub fn has_double_spends(&self) -> Option<Bytes32> {
        let mut seen = std::collections::HashSet::<Bytes32>::new();
        self.all_removals().into_iter().find(|&id| !seen.insert(id))
    }

    /// Full `bincode` body size (header + spends + slash payloads + signature), per SPEC serialization rules.
    #[must_use]
    pub fn compute_size(&self) -> usize {
        bincode::serialize(self).map(|b| b.len()).unwrap_or(0)
    }

    /// Tier 1 **structural** validation: cheap consistency checks that need no chain state ([SPEC §5.2](docs/resources/SPEC.md)).
    ///
    /// **SVL-005** ([spec](docs/requirements/domains/structural_validation/specs/SVL-005.md)): header counters
    /// `spend_bundle_count`, `additions_count`, `removals_count`, and `slash_proposal_count` MUST match the body
    /// (`spend_bundles`, [`Self::all_additions`], total [`CoinSpend`] rows, `slash_proposal_payloads`).
    ///
    /// **SVL-006** ([spec](docs/requirements/domains/structural_validation/specs/SVL-006.md)): after counts, enforces
    /// Merkle commitments and integrity in **SPEC §5.2** order: `spends_root` → duplicate outputs (Chia check 13) →
    /// double spends (check 14) → `additions_root` / `removals_root` → BIP158 `filter_hash` → slash count and per-payload
    /// byte caps → `slash_proposals_root` → full-block bincode size vs [`crate::MAX_BLOCK_SIZE`]. All hashing reuses
    /// [`Self::compute_spends_root`], [`Self::compute_additions_root`], [`Self::compute_removals_root`],
    /// [`Self::compute_filter_hash`], [`Self::compute_slash_proposals_root`] so validation stays aligned with HSH-003–006
    /// and BLK-004.
    ///
    /// **Rationale:** Count checks stay first (cheap, fail-fast); Merkle and filter work only after cardinality is sane;
    /// serialized-size is last so malicious oversized bodies still pay for earlier checks where applicable.
    /// [`crate::validation::structural`](crate::validation::structural) indexes the SVL matrix.
    pub fn validate_structure(&self) -> Result<(), BlockError> {
        let actual_spend_bundles = u32_len(self.spend_bundles.len());
        if self.header.spend_bundle_count != actual_spend_bundles {
            return Err(BlockError::SpendBundleCountMismatch {
                header: self.header.spend_bundle_count,
                actual: actual_spend_bundles,
            });
        }

        let computed_additions = u32_len(self.all_additions().len());
        if self.header.additions_count != computed_additions {
            return Err(BlockError::AdditionsCountMismatch {
                header: self.header.additions_count,
                actual: computed_additions,
            });
        }

        let computed_removals: usize = self
            .spend_bundles
            .iter()
            .map(|sb| sb.coin_spends.len())
            .sum();
        let computed_removals = u32_len(computed_removals);
        if self.header.removals_count != computed_removals {
            return Err(BlockError::RemovalsCountMismatch {
                header: self.header.removals_count,
                actual: computed_removals,
            });
        }

        let actual_slash = u32_len(self.slash_proposal_payloads.len());
        if self.header.slash_proposal_count != actual_slash {
            return Err(BlockError::SlashProposalCountMismatch {
                header: self.header.slash_proposal_count,
                actual: actual_slash,
            });
        }

        // --- SVL-006: Merkle roots + integrity (SPEC §5.2 steps 3, 6–15) ---
        // Step 3 — spends_root (HSH-003)
        let computed_spends_root = self.compute_spends_root();
        if self.header.spends_root != computed_spends_root {
            return Err(BlockError::InvalidSpendsRoot {
                expected: self.header.spends_root,
                computed: computed_spends_root,
            });
        }

        // Steps 6–7 — duplicate outputs / double spends (Chia checks 13–14; BLK-004 probes)
        if let Some(coin_id) = self.has_duplicate_outputs() {
            return Err(BlockError::DuplicateOutput { coin_id });
        }
        if let Some(coin_id) = self.has_double_spends() {
            return Err(BlockError::DoubleSpendInBlock { coin_id });
        }

        // Steps 8–9 — additions / removals Merkle sets (HSH-004 / HSH-005)
        let computed_additions_root = self.compute_additions_root();
        if self.header.additions_root != computed_additions_root {
            return Err(BlockError::InvalidAdditionsRoot);
        }
        let computed_removals_root = self.compute_removals_root();
        if self.header.removals_root != computed_removals_root {
            return Err(BlockError::InvalidRemovalsRoot);
        }

        // Step 10 — BIP158 filter (HSH-006)
        let computed_filter_hash = self.compute_filter_hash();
        if self.header.filter_hash != computed_filter_hash {
            return Err(BlockError::InvalidFilterHash);
        }

        // Steps 11–12 — slash proposal policy ([`MAX_SLASH_PROPOSALS_PER_BLOCK`], [`MAX_SLASH_PROPOSAL_PAYLOAD_BYTES`])
        if self.slash_proposal_payloads.len() > MAX_SLASH_PROPOSALS_PER_BLOCK as usize {
            return Err(BlockError::TooManySlashProposals);
        }
        let max_payload = MAX_SLASH_PROPOSAL_PAYLOAD_BYTES as usize;
        for payload in &self.slash_proposal_payloads {
            if payload.len() > max_payload {
                return Err(BlockError::SlashProposalPayloadTooLarge);
            }
        }

        // Step 14 — slash proposals Merkle root (BLK-004 / header field)
        let computed_slash_root = self.compute_slash_proposals_root();
        if self.header.slash_proposals_root != computed_slash_root {
            return Err(BlockError::InvalidSlashProposalsRoot);
        }

        // Step 15 — actual serialized block size (bincode), independent of header `block_size` (SVL-003 caps declared field)
        let serialized_size = self.compute_size();
        if serialized_size > MAX_BLOCK_SIZE as usize {
            let size_u32 = u32::try_from(serialized_size).unwrap_or(u32::MAX);
            return Err(BlockError::TooLarge {
                size: size_u32,
                max: MAX_BLOCK_SIZE,
            });
        }

        Ok(())
    }

    /// Tier 2 — **execution validation** entry point ([EXE-001](docs/requirements/domains/execution_validation/specs/EXE-001.md), [SPEC §7.4](docs/resources/SPEC.md)).
    ///
    /// Processes each [`SpendBundle`] in **block order** (ephemeral-coin semantics depend on this)
    /// and returns an aggregated [`crate::ExecutionResult`] that carries into Tier 3
    /// ([STV-001](docs/requirements/domains/state_validation/specs/STV-001.md)).
    ///
    /// ## Scope of this method (EXE-001 alone)
    ///
    /// **Implemented:**
    /// - API surface matching NORMATIVE: `&self`, `&ValidationConfig` (from **dig-clvm**, see
    ///   [`docs/prompt/start.md`](docs/prompt/start.md) Hard Requirement 2), `&Bytes32`
    ///   (`genesis_challenge` for `AGG_SIG_ME` domain separation under EXE-005).
    /// - Block-order traversal of [`Self::spend_bundles`].
    /// - Block-level **fee consistency** ([EXE-006](docs/requirements/domains/execution_validation/specs/EXE-006.md))
    ///   — `computed_total_fees == header.total_fees`, else
    ///   [`BlockError::FeesMismatch`].
    /// - Block-level **cost consistency** ([EXE-007](docs/requirements/domains/execution_validation/specs/EXE-007.md))
    ///   — `computed_total_cost == header.total_cost`, else
    ///   [`BlockError::CostMismatch`].
    /// - Emits a fully populated (potentially empty) [`crate::ExecutionResult`]
    ///   ([EXE-008](docs/requirements/domains/execution_validation/specs/EXE-008.md)).
    ///
    /// **Deferred to later requirements (documented here for trace):**
    /// - **EXE-002** — `tree_hash(puzzle_reveal) == coin.puzzle_hash` per [`CoinSpend`]
    ///   ([`clvm_utils::tree_hash`]).
    /// - **EXE-003** — [`dig_clvm::validate_spend_bundle`] per bundle (CLVM execution);
    ///   note it requires a [`dig_clvm::ValidationContext`] with per-coin `CoinRecord`s, which
    ///   today lives in Tier 3 ([`crate::CoinLookup`]). Wiring this in is EXE-003's job.
    /// - **EXE-004 / EXE-009** — two-pass condition collection + [`crate::PendingAssertion`]
    ///   population.
    /// - **EXE-005** — BLS aggregate signature verification (inside `dig-clvm`).
    ///
    /// For this requirement alone, the method only needs to be callable with the NORMATIVE
    /// signature and must return the empty-block identity when there is no body. That matches the
    /// EXE-001 test plan's `empty_block` case; non-empty behavior is validated once EXE-003 lands.
    ///
    /// ## Error mapping
    ///
    /// | Trigger | Variant | Requirement |
    /// |---|---|---|
    /// | `computed_total_fees != header.total_fees` | [`BlockError::FeesMismatch`] | EXE-006 |
    /// | `computed_total_cost != header.total_cost` | [`BlockError::CostMismatch`] | EXE-007 |
    ///
    /// ## Chia parity
    ///
    /// The method sits at the same layer as `chia-blockchain`'s `pre_validate_blocks` + body-level
    /// checks ([`block_body_validation.py` Check 9 (`INVALID_BLOCK_COST`) + Check 19
    /// (`INVALID_BLOCK_FEE_AMOUNT`)](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py)).
    pub fn validate_execution(
        &self,
        clvm_config: &dig_clvm::ValidationConfig,
        genesis_challenge: &Bytes32,
    ) -> Result<crate::ExecutionResult, BlockError> {
        // NOTE: `clvm_config` and `genesis_challenge` are accepted per NORMATIVE EXE-001 but are
        // not consumed until EXE-003 (CLVM execution) / EXE-005 (BLS `AGG_SIG_ME` under genesis
        // domain). Silencing unused-variable lints here without suppressing the symbols so they
        // remain visible to the API surface.
        let _ = clvm_config;
        let _ = genesis_challenge;

        // Intentionally `mut`: EXE-003 will push additions/removals/receipts into `result` as
        // each bundle's `SpendResult` is produced. Kept mutable so the EXE-003 diff is minimal.
        #[allow(unused_mut)]
        let mut result = crate::ExecutionResult::default();

        // Process bundles in block order (ephemeral-coin semantics; EXE-001 NORMATIVE).
        //
        // EXE-002: For every CoinSpend, tree_hash(puzzle_reveal) MUST equal coin.puzzle_hash
        // before CLVM execution. This is cheap (pure SHA-256 over serialized CLVM bytes) and
        // fails fast on tampered puzzle reveals, so it runs first per Chia parity with Check 20.
        // See [`crate::verify_coin_spend_puzzle_hash`].
        //
        // EXE-003 (dig-clvm invocation) remains deferred — CLVM execution requires a
        // [`dig_clvm::ValidationContext`] seeded with coin_records from Tier 3 ([`crate::CoinLookup`]).
        // Will land alongside the Tier-2/Tier-3 bridge.
        for bundle in &self.spend_bundles {
            for coin_spend in &bundle.coin_spends {
                crate::verify_coin_spend_puzzle_hash(coin_spend)?;
            }
            // EXE-003 / EXE-004 / EXE-005 / EXE-009 deferred.
        }

        // EXE-006 — block-level fee consistency.
        if result.total_fees != self.header.total_fees {
            return Err(BlockError::FeesMismatch {
                header: self.header.total_fees,
                computed: result.total_fees,
            });
        }

        // EXE-007 — block-level cost consistency.
        if result.total_cost != self.header.total_cost {
            return Err(BlockError::CostMismatch {
                header: self.header.total_cost,
                computed: result.total_cost,
            });
        }

        Ok(result)
    }

    /// Tier-2 execution with an explicit [`dig_clvm::ValidationContext`]
    /// ([EXE-003](docs/requirements/domains/execution_validation/specs/EXE-003.md), [SPEC §7.4.3](docs/resources/SPEC.md)).
    ///
    /// ## Why this signature (not EXE-001's tight one)
    ///
    /// `dig_clvm::validate_spend_bundle` requires a [`dig_clvm::ValidationContext`] populated with
    /// per-coin [`chia_sdk_coinset::CoinRecord`]s before it can run CLVM (structural coin-exists
    /// check precedes execution). That state lives in Tier 3 ([`crate::CoinLookup`]). This method
    /// is the integration entry point callers use when they have a context ready; the
    /// NORMATIVE-pinned [`Self::validate_execution`] remains a thin wrapper over an **empty**
    /// context and is only sound for empty bodies. When the Tier-2/Tier-3 bridge lands
    /// (`validate_full`), the wrapper will build context from a provided `CoinLookup`.
    ///
    /// ## Pipeline
    ///
    /// 1. For each [`chia_protocol::SpendBundle`] in block order:
    ///    1. For each [`chia_protocol::CoinSpend`]: [`crate::verify_coin_spend_puzzle_hash`]
    ///       (EXE-002).
    ///    2. [`dig_clvm::validate_spend_bundle`] (EXE-003) — CLVM + conditions + BLS
    ///       aggregate verify + per-bundle conservation.
    ///    3. Fold `SpendResult` into the running [`crate::ExecutionResult`].
    /// 2. After all bundles:
    ///    1. EXE-006 fee consistency (`computed_total_fees == header.total_fees`).
    ///    2. EXE-007 cost consistency (`computed_total_cost == header.total_cost`).
    ///
    /// ## Error mapping
    ///
    /// All [`dig_clvm::ValidationError`] variants pass through [`crate::map_clvm_validation_error`]
    /// (EXE-003 mapping table) so callers see only [`BlockError`].
    ///
    /// ## Rationale vs delegating directly
    ///
    /// Keeping the CLVM call gated by a puzzle-hash pre-check (EXE-002) preserves fail-fast on
    /// tampered reveals without paying CLVM cost. Every other check runs inside dig-clvm.
    pub fn validate_execution_with_context(
        &self,
        clvm_config: &dig_clvm::ValidationConfig,
        genesis_challenge: &Bytes32,
        context: &dig_clvm::ValidationContext,
    ) -> Result<crate::ExecutionResult, BlockError> {
        // `genesis_challenge` is part of the AGG_SIG_ME domain; dig-clvm currently reads this
        // from `context.constants`, so the parameter is documentary here until EXE-005 uses it
        // to override per-call.
        let _ = genesis_challenge;

        let mut result = crate::ExecutionResult::default();

        for (idx, bundle) in self.spend_bundles.iter().enumerate() {
            // EXE-002: puzzle-hash pre-check (fail-fast before CLVM cost).
            for coin_spend in &bundle.coin_spends {
                crate::verify_coin_spend_puzzle_hash(coin_spend)?;
            }

            // EXE-003: delegate to dig-clvm for full CLVM + conditions + BLS + conservation.
            let spend_result = dig_clvm::validate_spend_bundle(bundle, context, clvm_config, None)
                .map_err(|e| {
                    // Rewrap bundle_index on signature failures; all other variants ignore it.
                    let mapped = crate::map_clvm_validation_error(e);
                    if let BlockError::SignatureFailed { .. } = mapped {
                        BlockError::SignatureFailed {
                            bundle_index: idx as u32,
                        }
                    } else {
                        mapped
                    }
                })?;

            // Aggregate per-bundle outputs into the block-level ExecutionResult (EXE-008 shape).
            result.additions.extend(spend_result.additions);
            result
                .removals
                .extend(spend_result.removals.iter().map(|c| c.coin_id()));
            result.total_cost = result
                .total_cost
                .saturating_add(spend_result.conditions.cost);
            result.total_fees = result.total_fees.saturating_add(spend_result.fee);

            // EXE-004: collect height / time pending assertions from this bundle's parsed
            // conditions (block-level absolutes + per-spend relatives). Tier-3 (STV-005)
            // evaluates them against chain context.
            result.pending_assertions.extend(
                crate::collect_pending_assertions_from_conditions(&spend_result.conditions),
            );

            // EXE-004 Pass 2 (announcement / concurrent-spend / self-assertions) + EXE-005
            // (BLS) run inside `dig_clvm::validate_spend_bundle` → `run_spendbundle`; rejection
            // surfaces via the mapped `ValidationError::Clvm` / `SignatureFailed` paths above.
            //
            // Per-bundle Receipt construction lives outside this commit; Tier-2 callers that
            // need receipts build them from the SpendResult + bundle metadata (RCP-002).
        }

        // EXE-006 — block-level fee consistency.
        if result.total_fees != self.header.total_fees {
            return Err(BlockError::FeesMismatch {
                header: self.header.total_fees,
                computed: result.total_fees,
            });
        }

        // EXE-007 — block-level cost consistency.
        if result.total_cost != self.header.total_cost {
            return Err(BlockError::CostMismatch {
                header: self.header.total_cost,
                computed: result.total_cost,
            });
        }

        Ok(result)
    }

    /// Tier 3 — **state validation** entry point ([STV-001](docs/requirements/domains/state_validation/specs/STV-001.md), [SPEC §7.5](docs/resources/SPEC.md)).
    ///
    /// Consumes the [`crate::ExecutionResult`] produced by Tier 2, cross-references it against
    /// the caller's [`crate::CoinLookup`] view of the coin set, verifies the proposer signature,
    /// and returns the computed state-trie root for commitment.
    ///
    /// ## Sub-checks (each a follow-on STV-* requirement)
    ///
    /// | Step | Requirement | Purpose |
    /// |---|---|---|
    /// | 1 | [STV-002](docs/requirements/domains/state_validation/specs/STV-002.md) | Every `exec.removals` coin exists and is unspent (or is ephemeral — present in `exec.additions`). |
    /// | 2 | [STV-003](docs/requirements/domains/state_validation/specs/STV-003.md) | `CoinState.coin.puzzle_hash` cross-check vs the spent coin's `puzzle_hash`. |
    /// | 3 | [STV-004](docs/requirements/domains/state_validation/specs/STV-004.md) | Every `exec.additions` coin is not already in the coin set (ephemeral exception). |
    /// | 4 | [STV-005](docs/requirements/domains/state_validation/specs/STV-005.md) | Evaluate each [`crate::PendingAssertion`] from Tier 2 against chain context. |
    /// | 5 | [STV-006](docs/requirements/domains/state_validation/specs/STV-006.md) | `chia_bls::verify(proposer_pubkey, header.hash(), proposer_signature)`. |
    /// | 6 | [STV-007](docs/requirements/domains/state_validation/specs/STV-007.md) | Apply additions / removals, recompute state root, compare to `header.state_root`, return it. |
    ///
    /// ## Scope of this commit (STV-001 only)
    ///
    /// Dispatcher with placeholder sub-check bodies. On empty inputs (zero additions / removals /
    /// pending assertions) every sub-check is a no-op and the method returns
    /// `self.header.state_root` directly — the boundary case needed for `validate_full` to
    /// finish a genesis-style empty block end-to-end. STV-002..007 will harden each step
    /// without changing this outer signature.
    ///
    /// ## Return value
    ///
    /// `Bytes32` — the computed state-trie root. For successful validation this equals
    /// `self.header.state_root`; callers use it as the committed parent-state value for the next
    /// block. This is why the return is not `()`.
    pub fn validate_state(
        &self,
        exec: &crate::ExecutionResult,
        coins: &dyn crate::CoinLookup,
        proposer_pubkey: &crate::primitives::PublicKey,
    ) -> Result<Bytes32, BlockError> {
        // STV-002 — coin existence. (Stub: on empty removals, no-op.)
        self.check_coin_existence_stub(exec, coins)?;
        // STV-003 — puzzle hash cross-check. (Stub.)
        self.check_puzzle_hashes_stub(exec, coins)?;
        // STV-004 — addition non-existence. (Stub.)
        self.check_addition_uniqueness_stub(exec, coins)?;
        // STV-005 — height/time lock evaluation. (Stub: on empty pending_assertions, no-op.)
        self.evaluate_pending_assertions_stub(exec, coins)?;
        // STV-006 — proposer signature. (Stub.)
        self.verify_proposer_signature_stub(proposer_pubkey)?;
        // STV-007 — state root verification + computation.
        self.compute_and_verify_state_root_stub(exec, coins)
    }

    /// Convenience wrapper: Tier 1 → Tier 2 → Tier 3 ([STV-001](docs/requirements/domains/state_validation/specs/STV-001.md)).
    ///
    /// Short-circuits on the first failing tier. On success returns the computed state root
    /// (same semantics as [`Self::validate_state`]). Each tier can still be called independently
    /// for partial validation or tests.
    pub fn validate_full(
        &self,
        clvm_config: &dig_clvm::ValidationConfig,
        genesis_challenge: &Bytes32,
        coins: &dyn crate::CoinLookup,
        proposer_pubkey: &crate::primitives::PublicKey,
    ) -> Result<Bytes32, BlockError> {
        // Tier 1 — structural validation.
        self.validate_structure()?;
        // Tier 2 — execution validation.
        let exec = self.validate_execution(clvm_config, genesis_challenge)?;
        // Tier 3 — state validation + compute state root.
        self.validate_state(&exec, coins, proposer_pubkey)
    }

    // --- STV-002..007 stub sub-checks (STV-001 dispatcher shape) ---

    /// STV-002 coin existence — stub. For empty `exec.removals` this is a no-op. Tightens in STV-002.
    fn check_coin_existence_stub(
        &self,
        _exec: &crate::ExecutionResult,
        _coins: &dyn crate::CoinLookup,
    ) -> Result<(), BlockError> {
        Ok(())
    }

    /// STV-003 puzzle-hash cross-check — stub. No-op on empty.
    fn check_puzzle_hashes_stub(
        &self,
        _exec: &crate::ExecutionResult,
        _coins: &dyn crate::CoinLookup,
    ) -> Result<(), BlockError> {
        Ok(())
    }

    /// STV-004 addition uniqueness — stub. No-op on empty additions.
    fn check_addition_uniqueness_stub(
        &self,
        _exec: &crate::ExecutionResult,
        _coins: &dyn crate::CoinLookup,
    ) -> Result<(), BlockError> {
        Ok(())
    }

    /// STV-005 height/time lock evaluation — stub. No-op on empty pending_assertions.
    fn evaluate_pending_assertions_stub(
        &self,
        _exec: &crate::ExecutionResult,
        _coins: &dyn crate::CoinLookup,
    ) -> Result<(), BlockError> {
        Ok(())
    }

    /// STV-006 proposer signature — stub. No-op until STV-006 wires `chia_bls::verify`.
    fn verify_proposer_signature_stub(
        &self,
        _pubkey: &crate::primitives::PublicKey,
    ) -> Result<(), BlockError> {
        Ok(())
    }

    /// STV-007 state-root recomputation — stub. Returns `header.state_root` directly so the
    /// outer method has a `Bytes32` to return. STV-007 will recompute from `exec` + `coins` and
    /// compare.
    fn compute_and_verify_state_root_stub(
        &self,
        _exec: &crate::ExecutionResult,
        _coins: &dyn crate::CoinLookup,
    ) -> Result<Bytes32, BlockError> {
        Ok(self.header.state_root)
    }
}

/// Convert slice lengths to `u32` for header/count fields; saturates at `u32::MAX` if the platform `usize` exceeds it.
#[inline]
fn u32_len(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// First repeated [`Coin::coin_id`] in a slice of additions (shared by [`L2Block::has_duplicate_outputs`]).
#[must_use]
fn first_duplicate_addition_coin_id(coins: &[Coin]) -> Option<Bytes32> {
    let mut seen = std::collections::HashSet::<Bytes32>::new();
    for c in coins {
        let id = c.coin_id();
        if !seen.insert(id) {
            return Some(id);
        }
    }
    None
}

/// Exposed for [`tests/test_l2_block_helpers.rs`] (BLK-004) only — not protocol surface.
#[doc(hidden)]
#[must_use]
pub fn __blk004_first_duplicate_addition_coin_id(coins: &[Coin]) -> Option<Bytes32> {
    first_duplicate_addition_coin_id(coins)
}
