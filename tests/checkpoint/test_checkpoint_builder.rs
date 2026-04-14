//! CKP-006: [`dig_block::CheckpointBuilder`] — incremental epoch summary construction per NORMATIVE.
//!
//! **Normative:** `docs/requirements/domains/checkpoint/NORMATIVE.md` (CKP-006)  
//! **Spec + test plan:** `docs/requirements/domains/checkpoint/specs/CKP-006.md`  
//! **Implementation:** `src/builder/checkpoint_builder.rs`
//!
//! ## How these tests prove CKP-006
//!
//! - **Accumulation:** `add_block` / `add_withdrawal` drive `block_count`, `withdrawal_count`, `tx_count`, and
//!   `total_fees` in the built [`Checkpoint`] ([CKP-006 acceptance](docs/requirements/domains/checkpoint/specs/CKP-006.md#acceptance-criteria)).
//! - **Merkle roots:** `block_root` and `withdrawals_root` must match [`chia_sdk_types::MerkleTree`] over the same
//!   leaf order as BLK-004 spends/slash roots ([`merkle_tree_root`](docs/requirements/domains/block_types/specs/BLK-004.md) —
//!   [HSH-007](docs/requirements/domains/hashing/specs/HSH-007.md) tagging via `MerkleTree`). Empty lists → [`EMPTY_ROOT`]
//!   ([BLK-005](docs/requirements/domains/block_types/specs/BLK-005.md)).
//! - **Linkage:** `epoch` and `prev_checkpoint` propagate from [`CheckpointBuilder::new`]; [`CheckpointBuilder::set_state_root`]
//!   overwrites the builder’s state commitment before [`CheckpointBuilder::build`].

use chia_sdk_types::MerkleTree;
use dig_block::{Bytes32, CheckpointBuilder, EMPTY_ROOT};

fn byte_tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// **Test plan:** “Build empty checkpoint” — no blocks/withdrawals, roots sentinel ([CKP-006 § Test Plan](docs/requirements/domains/checkpoint/specs/CKP-006.md#test-plan)).
#[test]
fn ckp006_build_empty_checkpoint() {
    let prev = byte_tag(0x01);
    let cp = CheckpointBuilder::new(9, prev).build();
    assert_eq!(cp.epoch, 9);
    assert_eq!(cp.prev_checkpoint, prev);
    assert_eq!(cp.block_count, 0);
    assert_eq!(cp.withdrawal_count, 0);
    assert_eq!(cp.block_root, EMPTY_ROOT);
    assert_eq!(cp.withdrawals_root, EMPTY_ROOT);
    assert_eq!(cp.tx_count, 0);
    assert_eq!(cp.total_fees, 0);
    assert_eq!(cp.state_root, Bytes32::default());
}

/// **Test plan:** “Build with single block”.
#[test]
fn ckp006_build_single_block() {
    let h = byte_tag(0xab);
    let mut b = CheckpointBuilder::new(1, byte_tag(0));
    b.add_block(h, 3, 100);
    let cp = b.build();
    assert_eq!(cp.block_count, 1);
    assert_eq!(cp.tx_count, 3);
    assert_eq!(cp.total_fees, 100);
    assert_eq!(cp.block_root, MerkleTree::new(&[h]).root());
}

/// **Test plan:** “Build with multiple blocks” + “Fee aggregation” + “Tx count aggregation”.
#[test]
fn ckp006_build_three_blocks_aggregates_tx_and_fees() {
    let h1 = byte_tag(0x10);
    let h2 = byte_tag(0x20);
    let h3 = byte_tag(0x30);
    let mut b = CheckpointBuilder::new(2, byte_tag(0xff));
    b.add_block(h1, 5, 10);
    b.add_block(h2, 10, 20);
    b.add_block(h3, 15, 30);
    let cp = b.build();
    assert_eq!(cp.block_count, 3);
    assert_eq!(cp.tx_count, 30);
    assert_eq!(cp.total_fees, 60);
    let expected_root = MerkleTree::new(&[h1, h2, h3]).root();
    assert_eq!(cp.block_root, expected_root);
}

/// **Test plan:** “Block root Merkle computation” (two leaves).
#[test]
fn ckp006_block_root_matches_merkle_tree() {
    let a = byte_tag(0x01);
    let c = byte_tag(0x02);
    let mut b = CheckpointBuilder::new(0, EMPTY_ROOT);
    b.add_block(a, 0, 0);
    b.add_block(c, 0, 0);
    let cp = b.build();
    assert_eq!(cp.block_root, MerkleTree::new(&[a, c]).root());
}

/// **Test plan:** “Withdrawals root Merkle computation” + empty blocks list still allowed.
#[test]
fn ckp006_withdrawals_root_matches_merkle_tree() {
    let w1 = byte_tag(0xaa);
    let w2 = byte_tag(0xbb);
    let mut b = CheckpointBuilder::new(0, EMPTY_ROOT);
    b.add_withdrawal(w1);
    b.add_withdrawal(w2);
    let cp = b.build();
    assert_eq!(cp.withdrawal_count, 2);
    assert_eq!(cp.withdrawals_root, MerkleTree::new(&[w1, w2]).root());
    assert_eq!(cp.block_root, EMPTY_ROOT);
}

/// **Test plan:** “Set state root”.
#[test]
fn ckp006_set_state_root_persists() {
    let sr = byte_tag(0x77);
    let mut b = CheckpointBuilder::new(3, byte_tag(0x11));
    b.set_state_root(sr);
    let cp = b.build();
    assert_eq!(cp.state_root, sr);
}

/// **Test plan:** “Prev checkpoint linkage” + “Epoch propagation” (covered in empty test; explicit here).
#[test]
fn ckp006_epoch_and_prev_checkpoint_from_new() {
    let epoch = 123_456u64;
    let prev = byte_tag(0xcd);
    let cp = CheckpointBuilder::new(epoch, prev).build();
    assert_eq!(cp.epoch, epoch);
    assert_eq!(cp.prev_checkpoint, prev);
}
