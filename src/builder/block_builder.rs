//! [`BlockBuilder`] — incremental construction of signed [`crate::L2Block`] instances.
//!
//! ## Requirements trace
//!
//! - **[BLD-001](docs/requirements/domains/block_production/specs/BLD-001.md)** — struct fields and `new()` constructor.
//! - **[BLD-002](docs/requirements/domains/block_production/specs/BLD-002.md)** — `add_spend_bundle()` with cost/size budget enforcement, `remaining_cost()`, `spend_bundle_count()`.
//! - **[BLD-003](docs/requirements/domains/block_production/specs/BLD-003.md)** — `add_slash_proposal()` with count/size limits.
//! - **[BLD-004](docs/requirements/domains/block_production/specs/BLD-004.md)** — `set_l1_proofs()`, `set_dfsp_roots()`, `set_extension_data()`.
//! - **[BLD-005](docs/requirements/domains/block_production/specs/BLD-005.md)** — `build()` pipeline: compute all derived fields, sign header.
//! - **[BLD-006](docs/requirements/domains/block_production/specs/BLD-006.md)** — [`crate::BlockSigner`] integration in `build()`.
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
//! **BLD-001** (struct + `new`) and **BLD-002** (`add_spend_bundle`, `remaining_cost`, `spend_bundle_count`) are
//! implemented. **`add_slash_proposal` / `set_*` / `build`** follow in BLD-003 — BLD-007.

use bincode;
use chia_protocol::{Coin, SpendBundle};

use crate::error::BuilderError;
use crate::merkle_util::empty_on_additions_err;
use crate::primitives::{Bytes32, Cost, Signature};
use crate::types::block::L2Block;
use crate::types::header::L2BlockHeader;
use crate::{EMPTY_ROOT, MAX_BLOCK_SIZE, MAX_COST_PER_BLOCK};

/// Incremental accumulator for a single L2 block body and header metadata ([SPEC §6.1–6.2](docs/resources/SPEC.md),
/// [BLD-001](docs/requirements/domains/block_production/specs/BLD-001.md)).
///
/// **Usage:** Construct with [`Self::new`], add spend bundles and optional slash payloads via future BLD-002/003 APIs,
/// then call `build(...)` (BLD-005) to obtain a signed [`crate::L2Block`]. The struct exposes **public fields** so
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
    /// Flattened [`Coin`] outputs extracted from spends (BLD-002 will maintain this on each add).
    pub additions: Vec<Coin>,
    /// Spent coin IDs (same bytes as `coin.coin_id()` / NORMATIVE `CoinId`).
    pub removals: Vec<Bytes32>,
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
        }
    }

    /// Serialized [`L2Block`] byte length if the body were `spend_bundles` plus this builder’s slash payloads.
    ///
    /// **Rationale (BLD-002):** [`crate::L2Block::validate_structure`] and SVL-003 compare **full** `bincode(L2Block)`
    /// against [`MAX_BLOCK_SIZE`](crate::MAX_BLOCK_SIZE). The builder does not yet have a final header (BLD-005), so we
    /// synthesize a probe [`L2BlockHeader`] with **fixed-shape** fields only: identity scalars copied from
    /// [`Self::new`], Merkle counters set to zero, roots set to [`EMPTY_ROOT`], and a default [`Signature`]. The
    /// header’s bincode footprint is independent of those placeholder root values (same struct layout as production),
    /// while the variable portion (`Vec<SpendBundle>`, `Vec<Vec<u8>>` slash payloads) matches what this builder will
    /// eventually serialize — so the estimate tracks real growth in body bytes.
    ///
    /// **Related:** [`L2Block::compute_size`](crate::L2Block::compute_size) (BLK-004) uses the same `bincode` schema.
    fn serialized_l2_block_probe_len(&self, spend_bundles: &[SpendBundle]) -> usize {
        let header = L2BlockHeader::new(
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
}
