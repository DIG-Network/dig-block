//! Incremental construction of [`crate::Checkpoint`] for an L2 epoch ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md)).
//!
//! ## Requirements trace
//!
//! - **[CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md)** — [`CheckpointBuilder`]: `new`, `add_block`,
//!   `set_state_root`, `add_withdrawal`, `build`.
//! - **[NORMATIVE § CKP-006](docs/requirements/domains/checkpoint/NORMATIVE.md)** — builder obligations for epoch summaries.
//! - **[SPEC §6.6](docs/resources/SPEC.md)** — checkpoint builder in block production.
//! - **Merkle roots:** [`CheckpointBuilder::build`] uses the crate-internal `merkle_tree_root` helper — same tagged binary Merkle tree as
//!   [`crate::L2Block::compute_spends_root`](crate::L2Block::compute_spends_root) /
//!   [`crate::L2Block::compute_slash_proposals_root`](crate::L2Block::compute_slash_proposals_root)
//!   ([BLK-004](docs/requirements/domains/block_types/specs/BLK-004.md), [HSH-007](docs/requirements/domains/hashing/specs/HSH-007.md)
//!   via `chia_sdk_types::MerkleTree`). **Empty** leaf lists → [`crate::EMPTY_ROOT`]
//!   ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md) implementation notes).
//! - **[HSH-002](docs/requirements/domains/hashing/specs/HSH-002.md)** — [`crate::Checkpoint::hash`] applies once the
//!   built [`Checkpoint`] is complete.
//!
//! ## Rationale
//!
//! - **Consuming `build`:** [`CheckpointBuilder::build`] takes `self` by value so a finalized [`crate::Checkpoint`] cannot be
//!   accidentally extended ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md) implementation notes).
//! - **Ordered leaves:** Block hashes and withdrawal hashes are Merkle leaves in **append order** — matches
//!   [`CheckpointBuilder::add_block`] / [`CheckpointBuilder::add_withdrawal`] call order during the epoch.
//! - **State root:** Defaults to [`Bytes32::default`] until [`CheckpointBuilder::set_state_root`] runs; production code should set the
//!   post-epoch trie root before signing.

use crate::merkle_util::merkle_tree_root;
use crate::primitives::Bytes32;
use crate::types::checkpoint::Checkpoint;

/// Accumulates per-epoch block hashes, fees, tx counts, and withdrawal commitments.
///
/// [`CheckpointBuilder::build`] materializes a [`Checkpoint`] with Merkle `block_root` / `withdrawals_root` ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md)).
pub struct CheckpointBuilder {
    epoch: u64,
    prev_checkpoint: Bytes32,
    state_root: Bytes32,
    block_hashes: Vec<Bytes32>,
    tx_count: u64,
    total_fees: u64,
    withdrawal_hashes: Vec<Bytes32>,
}

impl CheckpointBuilder {
    /// Start an epoch summary builder: link to `prev_checkpoint`, initialize empty accumulators ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md)).
    #[must_use]
    pub fn new(epoch: u64, prev_checkpoint: Bytes32) -> Self {
        Self {
            epoch,
            prev_checkpoint,
            state_root: Bytes32::default(),
            block_hashes: Vec::new(),
            tx_count: 0,
            total_fees: 0,
            withdrawal_hashes: Vec::new(),
        }
    }

    /// Append one L2 block’s identity hash and roll up its `tx_count` and `fees` ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md)).
    pub fn add_block(&mut self, block_hash: Bytes32, tx_count: u64, fees: u64) {
        self.block_hashes.push(block_hash);
        self.tx_count += tx_count;
        self.total_fees += fees;
    }

    /// Set the finalized L2 state root for this epoch ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md)).
    pub fn set_state_root(&mut self, state_root: Bytes32) {
        self.state_root = state_root;
    }

    /// Append one withdrawal record hash (leaf material for `withdrawals_root`) ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md)).
    pub fn add_withdrawal(&mut self, withdrawal_hash: Bytes32) {
        self.withdrawal_hashes.push(withdrawal_hash);
    }

    /// Finalize: compute Merkle roots over accumulated hashes and return an owned [`Checkpoint`] ([CKP-006](docs/requirements/domains/checkpoint/specs/CKP-006.md)).
    ///
    /// **`block_count` / `withdrawal_count`:** Derived from vector lengths, cast to `u32` per [`Checkpoint`] layout (CKP-001).
    /// Protocol epochs must stay within `u32` item counts; if lengths exceed `u32::MAX`, the cast truncates — callers must bound work.
    #[must_use]
    pub fn build(self) -> Checkpoint {
        let block_root = merkle_tree_root(&self.block_hashes);
        let withdrawals_root = merkle_tree_root(&self.withdrawal_hashes);
        Checkpoint {
            epoch: self.epoch,
            state_root: self.state_root,
            block_root,
            block_count: self.block_hashes.len() as u32,
            tx_count: self.tx_count,
            total_fees: self.total_fees,
            prev_checkpoint: self.prev_checkpoint,
            withdrawals_root,
            withdrawal_count: self.withdrawal_hashes.len() as u32,
        }
    }
}
