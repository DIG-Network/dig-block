//! [`BlockBuilder`] — incremental construction of signed [`crate::L2Block`] instances.
//!
//! ## Requirements trace
//!
//! - **[BLD-001](docs/requirements/domains/block_production/specs/BLD-001.md)** — struct fields and `new()` constructor.
//! - **[BLD-002](docs/requirements/domains/block_production/specs/BLD-002.md)** — `add_spend_bundle()` with cost/size budget enforcement, `remaining_cost()`, `spend_bundle_count()`.
//! - **[BLD-003](docs/requirements/domains/block_production/specs/BLD-003.md)** — `add_slash_proposal()` with count/size limits.
//! - **[BLD-004](docs/requirements/domains/block_production/specs/BLD-004.md)** — `set_l1_proofs()`, `set_dfsp_roots()`, `set_extension_data()`.
//! - **[BLD-005](docs/requirements/domains/block_production/specs/BLD-005.md)** — `build()` pipeline: compute all derived fields, sign header.
//! - **[BLD-006](docs/requirements/domains/block_production/specs/BLD-006.md)** — [`crate::BlockSigner`] integration in `build()` (see `tests/test_bld_006_block_signer_integration.rs`).
//! - **[BLD-007](docs/requirements/domains/block_production/specs/BLD-007.md)** — structural validity guarantee: output always passes `validate_structure()`.
//! - **[NORMATIVE](docs/requirements/domains/block_production/NORMATIVE.md)** — full block production domain.
//! - **[SPEC §6](docs/resources/SPEC.md)** — block production lifecycle.
//!
//! ## Build pipeline overview (BLD-005)
//!
//! ```text
//! 1. compute spends_root        ← MerkleTree(sha256(bundle)) per HSH-003
//! 2. compute additions_root     ← compute_merkle_set_root(grouped by puzzle_hash) per HSH-004
//! 3. compute removals_root      ← compute_merkle_set_root(coin IDs) per HSH-005
//! 4. compute filter_hash        ← SHA-256(BIP158 compact filter) per HSH-006
//! 5. compute slash_proposals_root ← MerkleTree(sha256(payload)) per BLK-004
//! 6. count all items            ← spend_bundle_count, additions_count, removals_count, slash_proposal_count
//! 7. auto-detect version        ← protocol_version_for_height(height) per BLK-007
//! 8. set timestamp              ← current wall-clock time
//! 9. compute block_size         ← two-pass: assemble with size=0, measure, update
//! 10. sign header               ← signer.sign_block(&header_hash) per BLD-006
//! ```
//!
//! ## Design decisions
//!
//! - **Consuming `build(self)`:** Takes ownership so the builder cannot be reused after producing a block,
//!   preventing accidental double-build or stale state.
//! - **Budget enforcement on add, not build:** `add_spend_bundle()` rejects bundles that would exceed
//!   `MAX_COST_PER_BLOCK` or `MAX_BLOCK_SIZE` _before_ mutating state (BLD-002). This means a rejected
//!   bundle leaves the builder unchanged — callers can try a smaller bundle.
//! - **State root and receipts_root are parameters to `build()`:** The builder doesn't maintain coin state,
//!   so the caller (proposer layer) must compute these externally and pass them in.
//!
//! ## Status
//!
//! **BLD-001**–**BLD-006** are implemented (`new`, body accumulation, optional header setters, [`Self::build`] /
//! [`Self::build_with_dfsp_activation`], [`crate::BlockSigner`] hook).
//! **BLD-007** (every `build` output structurally valid) is partially evidenced by
//! `tests/test_bld_005_build_pipeline.rs` calling [`crate::L2Block::validate_structure`] on successful builds; a full
//! BLD-007 requirement pass remains for explicit negative cases and documentation tightening.

use std::time::{SystemTime, UNIX_EPOCH};

use bincode;
use chia_protocol::{Coin, SpendBundle};

use crate::error::BuilderError;
use crate::merkle_util::empty_on_additions_err;
use crate::primitives::{Bytes32, Cost, Signature};
use crate::traits::BlockSigner;
use crate::types::block::L2Block;
use crate::types::header::L2BlockHeader;
use crate::{
    compute_additions_root, compute_filter_hash, compute_removals_root, compute_spends_root,
    DFSP_ACTIVATION_HEIGHT, EMPTY_ROOT, MAX_BLOCK_SIZE, MAX_COST_PER_BLOCK,
    MAX_SLASH_PROPOSALS_PER_BLOCK, MAX_SLASH_PROPOSAL_PAYLOAD_BYTES, VERSION_V2, ZERO_HASH,
};

/// Incremental accumulator for a single L2 block body and header metadata ([SPEC §6.1–6.2](docs/resources/SPEC.md),
/// [BLD-001](docs/requirements/domains/block_production/specs/BLD-001.md)).
///
/// **Usage:** Construct with [`Self::new`], optionally configure L1 proof anchors / DFSP roots / [`L2BlockHeader::extension_data`]
/// via [`Self::set_l1_proofs`], [`Self::set_dfsp_roots`], [`Self::set_extension_data`] (BLD-004), add spend bundles via
/// [`Self::add_spend_bundle`] (BLD-002) and slash payloads via [`Self::add_slash_proposal`] (BLD-003), then call
/// `build(...)` (BLD-005) to obtain a signed [`crate::L2Block`].
/// The struct exposes **public fields** so
/// advanced callers or tests can inspect partial state without accessor boilerplate; treat them as read-mostly except
/// through official builder methods once those exist.
///
/// **Rationale:** Public fields match the BLD-001 specification prose and mirror the SPEC §6.1 layout (caller context,
/// accumulated body, running totals). [`Coin`] and [`SpendBundle`] stay on **`chia-protocol`** types per project rules
/// ([`docs/prompt/start.md`](docs/prompt/start.md) — Chia ecosystem first).
///
/// **Removals type:** NORMATIVE names this `Vec<CoinId>`; `chia-protocol` does not export a separate `CoinId` newtype in
/// the versions we pin — coin IDs are the same 32-byte values as [`Bytes32`] (see BLK-004 / [`crate::L2Block::all_removals`]).
pub struct BlockBuilder {
    /// Block height this builder is assembling (immutable for the lifetime of the builder).
    pub height: u64,
    /// Epoch index ([`crate::L2BlockHeader::epoch`] semantics).
    pub epoch: u64,
    /// Parent L2 block header hash (chain link).
    pub parent_hash: Bytes32,
    /// Anchoring L1 block height for light-client / bridge logic.
    pub l1_height: u32,
    /// Anchoring L1 block hash.
    pub l1_hash: Bytes32,
    /// Proposer slot index in the validator set for this block.
    pub proposer_index: u32,
    /// Spend bundles accumulated in insertion order (body).
    pub spend_bundles: Vec<SpendBundle>,
    /// Raw slash-proposal payloads (opaque bytes per protocol).
    pub slash_proposal_payloads: Vec<Vec<u8>>,
    /// Running sum of CLVM costs for bundles accepted so far ([`Cost`] / BLK-006).
    pub total_cost: Cost,
    /// Running sum of fees from accepted bundles.
    pub total_fees: u64,
    /// Flattened [`Coin`] outputs extracted from spends ([`Self::add_spend_bundle`] / BLD-002).
    pub additions: Vec<Coin>,
    /// Spent coin IDs (same bytes as `coin.coin_id()` / NORMATIVE `CoinId`).
    pub removals: Vec<Bytes32>,

    // --- BLD-004: optional / deferred header fields (copied into [`L2BlockHeader`] in BLD-005) ---
    /// L1 collateral proof anchor ([`L2BlockHeader::l1_collateral_coin_id`]); `None` until [`Self::set_l1_proofs`].
    pub l1_collateral_coin_id: Option<Bytes32>,
    /// Network validator collateral set anchor ([`L2BlockHeader::l1_reserve_coin_id`]).
    pub l1_reserve_coin_id: Option<Bytes32>,
    /// Previous-epoch finalizer proof coin ([`L2BlockHeader::l1_prev_epoch_finalizer_coin_id`]).
    pub l1_prev_epoch_finalizer_coin_id: Option<Bytes32>,
    /// Current-epoch finalizer proof coin ([`L2BlockHeader::l1_curr_epoch_finalizer_coin_id`]).
    pub l1_curr_epoch_finalizer_coin_id: Option<Bytes32>,
    /// Network singleton proof coin ([`L2BlockHeader::l1_network_coin_id`]).
    pub l1_network_coin_id: Option<Bytes32>,

    /// DFSP collateral registry root ([`L2BlockHeader::collateral_registry_root`]); defaults [`EMPTY_ROOT`] per SVL-002.
    pub collateral_registry_root: Bytes32,
    /// DFSP CID state root ([`L2BlockHeader::cid_state_root`]).
    pub cid_state_root: Bytes32,
    /// DFSP node registry root ([`L2BlockHeader::node_registry_root`]).
    pub node_registry_root: Bytes32,
    /// Namespace update delta root ([`L2BlockHeader::namespace_update_root`]).
    pub namespace_update_root: Bytes32,
    /// DFSP finalize commitment root ([`L2BlockHeader::dfsp_finalize_commitment_root`]).
    pub dfsp_finalize_commitment_root: Bytes32,

    /// Header extension slot ([`L2BlockHeader::extension_data`]); default [`ZERO_HASH`] matches [`L2BlockHeader::new`].
    pub extension_data: Bytes32,
}

impl BlockBuilder {
    /// Create an empty builder anchored at the given chain / L1 context ([BLD-001](docs/requirements/domains/block_production/specs/BLD-001.md)).
    ///
    /// **Contract:** All accumulation fields start empty or zero; identity arguments are copied into the struct so the
    /// caller may reuse their locals afterward without aliasing the builder’s internal state.
    #[must_use]
    pub fn new(
        height: u64,
        epoch: u64,
        parent_hash: Bytes32,
        l1_height: u32,
        l1_hash: Bytes32,
        proposer_index: u32,
    ) -> Self {
        Self {
            height,
            epoch,
            parent_hash,
            l1_height,
            l1_hash,
            proposer_index,
            spend_bundles: Vec::new(),
            slash_proposal_payloads: Vec::new(),
            total_cost: 0,
            total_fees: 0,
            additions: Vec::new(),
            removals: Vec::new(),
            l1_collateral_coin_id: None,
            l1_reserve_coin_id: None,
            l1_prev_epoch_finalizer_coin_id: None,
            l1_curr_epoch_finalizer_coin_id: None,
            l1_network_coin_id: None,
            collateral_registry_root: EMPTY_ROOT,
            cid_state_root: EMPTY_ROOT,
            node_registry_root: EMPTY_ROOT,
            namespace_update_root: EMPTY_ROOT,
            dfsp_finalize_commitment_root: EMPTY_ROOT,
            extension_data: ZERO_HASH,
        }
    }

    /// Build a probe [`L2BlockHeader`] sharing this builder’s identity, L1 anchor, **optional L1 proofs**, **DFSP roots**,
    /// and **extension** fields ([BLD-004](docs/requirements/domains/block_production/specs/BLD-004.md)).
    ///
    /// **Rationale:** `bincode` encodes `Option<Bytes32>` with a discriminant — `Some(_)` rows are larger than `None`.
    /// After BLD-004, [`Self::serialized_l2_block_probe_len`] must fold these fields in so [`Self::add_spend_bundle`]
    /// (BLD-002) cannot underestimate wire size when proofs are present.
    fn probe_header_stub(&self) -> L2BlockHeader {
        let mut h = L2BlockHeader::new(
            self.height,
            self.epoch,
            self.parent_hash,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            EMPTY_ROOT,
            self.l1_height,
            self.l1_hash,
            self.proposer_index,
            0,
            0,
            0,
            0,
            0,
            0,
            EMPTY_ROOT,
        );
        h.l1_collateral_coin_id = self.l1_collateral_coin_id;
        h.l1_reserve_coin_id = self.l1_reserve_coin_id;
        h.l1_prev_epoch_finalizer_coin_id = self.l1_prev_epoch_finalizer_coin_id;
        h.l1_curr_epoch_finalizer_coin_id = self.l1_curr_epoch_finalizer_coin_id;
        h.l1_network_coin_id = self.l1_network_coin_id;
        h.collateral_registry_root = self.collateral_registry_root;
        h.cid_state_root = self.cid_state_root;
        h.node_registry_root = self.node_registry_root;
        h.namespace_update_root = self.namespace_update_root;
        h.dfsp_finalize_commitment_root = self.dfsp_finalize_commitment_root;
        h.extension_data = self.extension_data;
        h
    }

    /// Serialized [`L2Block`] byte length if the body were `spend_bundles` plus this builder’s slash payloads.
    ///
    /// **Rationale (BLD-002):** [`crate::L2Block::validate_structure`] and SVL-003 compare **full** `bincode(L2Block)`
    /// against [`MAX_BLOCK_SIZE`](crate::MAX_BLOCK_SIZE). The builder does not yet have a final header (BLD-005), so we
    /// synthesize a probe header via [`Self::probe_header_stub`] (counts/ Merkle fields still placeholders), while the
    /// variable body (`Vec<SpendBundle>`, `Vec<Vec<u8>>` slash payloads) matches this builder — so the estimate tracks
    /// growth in spend + slash bytes **and** optional header encodings from BLD-004.
    ///
    /// **Related:** [`L2Block::compute_size`](crate::L2Block::compute_size) (BLK-004) uses the same `bincode` schema.
    fn serialized_l2_block_probe_len(&self, spend_bundles: &[SpendBundle]) -> usize {
        let header = self.probe_header_stub();
        let block = L2Block::new(
            header,
            spend_bundles.to_vec(),
            self.slash_proposal_payloads.clone(),
            Signature::default(),
        );
        bincode::serialize(&block)
            .map(|bytes| bytes.len())
            .unwrap_or(usize::MAX)
    }

    /// Append a [`SpendBundle`] after validating CLVM cost and serialized block-size budgets ([BLD-002](docs/requirements/domains/block_production/specs/BLD-002.md)).
    ///
    /// **Parameters:** `cost` / `fee` are **caller-supplied** aggregates for this bundle (typically from
    /// `dig_clvm::validate_spend_bundle` / execution preview). The builder trusts these numbers for budgeting only;
    /// [`L2BlockHeader::total_cost`] consistency is enforced later at execution tier (EXE-007) and in `build()` (BLD-005).
    ///
    /// **Budget order:** cost is checked first (cheap, no cloning). Size uses a bincode probe that temporarily
    /// [`clone`]s the candidate bundle — if the probe fails, `bundle` is still owned by the caller on `Err`.
    ///
    /// **Mutation contract:** On `Err`, **no** field of `self` changes. On `Ok`, additions/removals/totals/spend_bundles
    /// advance together so partial state is impossible.
    ///
    /// **Additions / removals:** Mirrors [`crate::L2Block::all_additions`] / [`crate::L2Block::all_removals`] —
    /// [`SpendBundle::additions`] (CLVM-simulated `CREATE_COIN`s) plus one removal [`Bytes32`] per [`CoinSpend`].
    pub fn add_spend_bundle(
        &mut self,
        bundle: SpendBundle,
        cost: Cost,
        fee: u64,
    ) -> Result<(), BuilderError> {
        let next_cost = self.total_cost.saturating_add(cost);
        if next_cost > MAX_COST_PER_BLOCK {
            return Err(BuilderError::CostBudgetExceeded {
                current: self.total_cost,
                addition: cost,
                max: MAX_COST_PER_BLOCK,
            });
        }

        let base_bytes = self.serialized_l2_block_probe_len(&self.spend_bundles);
        let mut probe_bundles = self.spend_bundles.clone();
        probe_bundles.push(bundle.clone());
        let with_bytes = self.serialized_l2_block_probe_len(&probe_bundles);

        if with_bytes > MAX_BLOCK_SIZE as usize {
            return Err(BuilderError::SizeBudgetExceeded {
                current: u32::try_from(base_bytes).unwrap_or(u32::MAX),
                addition: u32::try_from(with_bytes.saturating_sub(base_bytes)).unwrap_or(u32::MAX),
                max: MAX_BLOCK_SIZE,
            });
        }

        self.additions
            .extend(empty_on_additions_err(bundle.additions()));
        for cs in &bundle.coin_spends {
            self.removals.push(cs.coin.coin_id());
        }
        self.total_cost += cost;
        self.total_fees += fee;
        self.spend_bundles.push(bundle);
        Ok(())
    }

    /// Remaining CLVM cost budget before hitting [`MAX_COST_PER_BLOCK`](crate::MAX_COST_PER_BLOCK) ([BLD-002](docs/requirements/domains/block_production/specs/BLD-002.md)).
    ///
    /// **Usage:** Proposers can gate bundle selection without duplicating protocol constants. Saturates at zero if
    /// `total_cost` ever overshoots (should not happen if only [`Self::add_spend_bundle`] mutates cost).
    #[must_use]
    pub fn remaining_cost(&self) -> Cost {
        MAX_COST_PER_BLOCK.saturating_sub(self.total_cost)
    }

    /// Number of spend bundles accepted so far (same as `spend_bundles.len()`).
    #[must_use]
    pub fn spend_bundle_count(&self) -> usize {
        self.spend_bundles.len()
    }

    /// Append an opaque slash-proposal payload after enforcing protocol caps ([BLD-003](docs/requirements/domains/block_production/specs/BLD-003.md)).
    ///
    /// **Count:** Rejects when [`Self::slash_proposal_payloads`] already holds [`MAX_SLASH_PROPOSALS_PER_BLOCK`](crate::MAX_SLASH_PROPOSALS_PER_BLOCK)
    /// rows — the guard uses `>=` so the **next** push would exceed the cap ([`BuilderError::TooManySlashProposals`]).
    ///
    /// **Size:** Rejects when `payload.len() > `[`MAX_SLASH_PROPOSAL_PAYLOAD_BYTES`](crate::MAX_SLASH_PROPOSAL_PAYLOAD_BYTES)
    /// ([`BuilderError::SlashProposalTooLarge`]). A payload whose length **equals** the limit is accepted (strict `>`).
    ///
    /// **Check order (spec):** Count is validated **before** size so that a builder already at the count cap surfaces
    /// [`BuilderError::TooManySlashProposals`] even if the candidate payload is also oversized — callers get a stable
    /// primary failure mode ([BLD-003 implementation notes](docs/requirements/domains/block_production/specs/BLD-003.md#implementation-notes)).
    ///
    /// **Mutation contract:** On `Err`, `slash_proposal_payloads` and all other fields are unchanged. On `Ok`, only
    /// `slash_proposal_payloads` grows; spend-bundle state is untouched (slash bytes still participate in the BLD-002
    /// `bincode(L2Block)` size probe on later [`Self::add_spend_bundle`] calls).
    pub fn add_slash_proposal(&mut self, payload: Vec<u8>) -> Result<(), BuilderError> {
        if self.slash_proposal_payloads.len() >= MAX_SLASH_PROPOSALS_PER_BLOCK as usize {
            return Err(BuilderError::TooManySlashProposals {
                max: MAX_SLASH_PROPOSALS_PER_BLOCK,
            });
        }
        let len = payload.len();
        if len > MAX_SLASH_PROPOSAL_PAYLOAD_BYTES as usize {
            return Err(BuilderError::SlashProposalTooLarge {
                size: u32::try_from(len).unwrap_or(u32::MAX),
                max: MAX_SLASH_PROPOSAL_PAYLOAD_BYTES,
            });
        }
        self.slash_proposal_payloads.push(payload);
        Ok(())
    }

    /// Set all five L1 proof anchor coin IDs at once ([BLD-004](docs/requirements/domains/block_production/specs/BLD-004.md)).
    ///
    /// **Semantics:** Each argument becomes `Some(hash)` on the builder, matching [`L2BlockHeader`]’s optional L1 proof
    /// fields ([`L2BlockHeader::l1_collateral_coin_id`] … [`L2BlockHeader::l1_network_coin_id`]). Callers omitting L1
    /// proofs should leave these as `None` (default from [`Self::new`]).
    ///
    /// **Overwrite:** Later calls replace the entire quintuple — there is no partial merge.
    pub fn set_l1_proofs(
        &mut self,
        collateral: Bytes32,
        reserve: Bytes32,
        prev_finalizer: Bytes32,
        curr_finalizer: Bytes32,
        network_coin: Bytes32,
    ) {
        self.l1_collateral_coin_id = Some(collateral);
        self.l1_reserve_coin_id = Some(reserve);
        self.l1_prev_epoch_finalizer_coin_id = Some(prev_finalizer);
        self.l1_curr_epoch_finalizer_coin_id = Some(curr_finalizer);
        self.l1_network_coin_id = Some(network_coin);
    }

    /// Set all five DFSP data-layer Merkle roots ([BLD-004](docs/requirements/domains/block_production/specs/BLD-004.md)).
    ///
    /// **Semantics:** Mirrors [`L2BlockHeader`]’s DFSP root block ([`L2BlockHeader::collateral_registry_root`] …
    /// [`L2BlockHeader::dfsp_finalize_commitment_root`]). Pre-activation callers typically keep [`EMPTY_ROOT`] values
    /// (SVL-002); post-activation, pass real roots before `build()` (BLD-005 validates against version).
    pub fn set_dfsp_roots(
        &mut self,
        collateral_registry_root: Bytes32,
        cid_state_root: Bytes32,
        node_registry_root: Bytes32,
        namespace_update_root: Bytes32,
        dfsp_finalize_commitment_root: Bytes32,
    ) {
        self.collateral_registry_root = collateral_registry_root;
        self.cid_state_root = cid_state_root;
        self.node_registry_root = node_registry_root;
        self.namespace_update_root = namespace_update_root;
        self.dfsp_finalize_commitment_root = dfsp_finalize_commitment_root;
    }

    /// Set the header extension hash ([BLD-004](docs/requirements/domains/block_production/specs/BLD-004.md)).
    ///
    /// **Semantics:** Stored as [`L2BlockHeader::extension_data`]; [`Self::new`] initializes to [`ZERO_HASH`] like
    /// [`L2BlockHeader::new`].
    pub fn set_extension_data(&mut self, extension_data: Bytes32) {
        self.extension_data = extension_data;
    }

    /// Finalize this builder into a signed [`L2Block`] ([BLD-005](docs/requirements/domains/block_production/specs/BLD-005.md)).
    ///
    /// **Parameters:** `state_root` / `receipts_root` come from the execution + receipt pipeline outside this crate
    /// (see module-level **Rationale**). `signer` produces the BLS attestation over [`L2BlockHeader::hash`] (HSH-001).
    ///
    /// **Pipeline:** Computes Merkle roots and counts using the same public functions as [`L2Block::validate_structure`]
    /// (`compute_spends_root`, `compute_additions_root`, `compute_removals_root`, [`L2Block::slash_proposals_root_from`],
    /// [`compute_filter_hash`]), sets wall-clock [`L2BlockHeader::timestamp`], runs the two-pass `block_size` fill
    /// (assemble with `block_size == 0`, measure [`L2Block::compute_size`], write), then signs.
    ///
    /// **Errors:** [`BuilderError::EmptyBlock`] if no spend bundles ([ERR-004](docs/requirements/domains/error_types/specs/ERR-004.md));
    /// [`BuilderError::MissingDfspRoots`] when [`VERSION_V2`](crate::VERSION_V2) applies but all DFSP roots are still
    /// [`EMPTY_ROOT`]; [`BuilderError::SigningFailed`] wraps [`crate::traits::SignerError`].
    ///
    /// **DFSP activation:** Uses [`crate::DFSP_ACTIVATION_HEIGHT`]. For tests or fork simulation with a different
    /// activation height, call [`Self::build_with_dfsp_activation`] instead.
    pub fn build(
        self,
        state_root: Bytes32,
        receipts_root: Bytes32,
        signer: &dyn BlockSigner,
    ) -> Result<L2Block, BuilderError> {
        self.build_with_dfsp_activation(state_root, receipts_root, signer, DFSP_ACTIVATION_HEIGHT)
    }

    /// Like [`Self::build`], but supplies an explicit `dfsp_activation_height` for BLK-007 / SVL-001 version selection and
    /// the BLD-005 DFSP-root precondition ([`BuilderError::MissingDfspRoots`]).
    ///
    /// **Rationale:** Crate tests keep [`DFSP_ACTIVATION_HEIGHT`] at `u64::MAX` (DFSP off) so normal `build()` always
    /// selects [`crate::VERSION_V1`]. Passing a finite `dfsp_activation_height` **≤** [`Self::height`] forces V2 in
    /// integration tests without recompiling constants.
    pub fn build_with_dfsp_activation(
        self,
        state_root: Bytes32,
        receipts_root: Bytes32,
        signer: &dyn BlockSigner,
        dfsp_activation_height: u64,
    ) -> Result<L2Block, BuilderError> {
        if self.spend_bundles.is_empty() {
            return Err(BuilderError::EmptyBlock);
        }

        let spends_root = compute_spends_root(&self.spend_bundles);
        let additions_root = compute_additions_root(&self.additions);
        let removals_root = compute_removals_root(&self.removals);
        let slash_proposals_root =
            L2Block::slash_proposals_root_from(&self.slash_proposal_payloads);
        let filter_hash = compute_filter_hash(self.parent_hash, &self.additions, &self.removals);

        let version = L2BlockHeader::protocol_version_for_height_with_activation(
            self.height,
            dfsp_activation_height,
        );
        if version == VERSION_V2 {
            let dfsp = [
                self.collateral_registry_root,
                self.cid_state_root,
                self.node_registry_root,
                self.namespace_update_root,
                self.dfsp_finalize_commitment_root,
            ];
            if dfsp.iter().all(|r| *r == EMPTY_ROOT) {
                return Err(BuilderError::MissingDfspRoots);
            }
        }

        let spend_bundle_count = usize_to_u32_count(self.spend_bundles.len());
        let additions_count = usize_to_u32_count(self.additions.len());
        let removals_rows: usize = self
            .spend_bundles
            .iter()
            .map(|sb| sb.coin_spends.len())
            .sum();
        let removals_count = usize_to_u32_count(removals_rows);
        let slash_proposal_count = usize_to_u32_count(self.slash_proposal_payloads.len());

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut header = L2BlockHeader::new(
            self.height,
            self.epoch,
            self.parent_hash,
            state_root,
            spends_root,
            additions_root,
            removals_root,
            receipts_root,
            self.l1_height,
            self.l1_hash,
            self.proposer_index,
            spend_bundle_count,
            self.total_cost,
            self.total_fees,
            additions_count,
            removals_count,
            0,
            filter_hash,
        );
        header.version = version;
        header.timestamp = timestamp;
        header.slash_proposal_count = slash_proposal_count;
        header.slash_proposals_root = slash_proposals_root;
        header.extension_data = self.extension_data;
        header.l1_collateral_coin_id = self.l1_collateral_coin_id;
        header.l1_reserve_coin_id = self.l1_reserve_coin_id;
        header.l1_prev_epoch_finalizer_coin_id = self.l1_prev_epoch_finalizer_coin_id;
        header.l1_curr_epoch_finalizer_coin_id = self.l1_curr_epoch_finalizer_coin_id;
        header.l1_network_coin_id = self.l1_network_coin_id;
        header.collateral_registry_root = self.collateral_registry_root;
        header.cid_state_root = self.cid_state_root;
        header.node_registry_root = self.node_registry_root;
        header.namespace_update_root = self.namespace_update_root;
        header.dfsp_finalize_commitment_root = self.dfsp_finalize_commitment_root;

        let spend_bundles = self.spend_bundles;
        let slash_proposal_payloads = self.slash_proposal_payloads;

        let mut block = L2Block::new(
            header,
            spend_bundles,
            slash_proposal_payloads,
            Signature::default(),
        );

        // Two-pass `block_size` (BLD-005): measure with placeholder zero, then store the full `bincode(L2Block)` length.
        let measured = block.compute_size();
        block.header.block_size = usize_to_u32_count(measured);

        // BLD-006: sign the **final** header digest (includes two-pass `block_size`; HSH-001 preimage omits BLS sig —
        // see BLK-003 `L2Block::proposer_signature`). ERR-004 stores `SignerError` as `String` via `Display` so
        // [`crate::BuilderError`] stays `Clone` without wrapping a nested `thiserror` source type.
        let header_hash = block.header.hash();
        let sig = signer
            .sign_block(&header_hash)
            .map_err(|e| BuilderError::SigningFailed(e.to_string()))?;
        block.proposer_signature = sig;

        Ok(block)
    }
}

/// Header/body count fields are `u32` on wire; saturate when `usize` exceeds `u32::MAX` (same helper pattern as BLK-004).
#[inline]
fn usize_to_u32_count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
