//! SER-002: `to_bytes()` / `from_bytes()` wire helpers ([NORMATIVE § SER-002](docs/requirements/domains/serialization/NORMATIVE.md#ser-002-to_bytes-and-from_bytes-conventions),
//! [spec](docs/requirements/domains/serialization/specs/SER-002.md), SPEC §8.2).
//!
//! ## Authoritative surface
//!
//! - **Block-shaped types** ([`dig_block::L2BlockHeader`], [`dig_block::L2Block`], [`dig_block::AttestedBlock`]): infallible
//!   [`to_bytes`](L2BlockHeader::to_bytes) returning [`Vec<u8>`]; fallible [`from_bytes`](L2BlockHeader::from_bytes) returning
//!   [`Result<_, BlockError>`](dig_block::BlockError) with decode failures mapped to [`BlockError::InvalidData`].
//! - **Checkpoint-shaped types** ([`dig_block::Checkpoint`], [`dig_block::CheckpointSubmission`]): same split with
//!   [`CheckpointError::InvalidData`](dig_block::CheckpointError::InvalidData) on decode.
//!
//! ## How this proves SER-002
//!
//! | Spec test plan row | What we assert | Why it satisfies NORMATIVE |
//! |--------------------|----------------|------------------------------|
//! | `to_bytes_succeeds` | Valid values produce **non-empty** `Vec<u8>` for header/block (body present) | Confirms API is infallible `Vec` return, not `Result` |
//! | `from_bytes_valid` | `from_bytes(&to_bytes(x))` recovers `x` (equality or stable byte round-trip) | Proves bincode codec matches serde schema |
//! | `from_bytes_truncated` | First *n* bytes of a valid payload ⇒ `Err(InvalidData)` | Tier-1 decode failure path |
//! | `from_bytes_empty` | `&[]` ⇒ `InvalidData` | Explicit empty-input obligation |
//! | `from_bytes_garbage` | Random short bytes ⇒ `InvalidData` | Corruption / type mismatch path |
//! | `checkpoint_from_bytes_invalid` | Same patterns on [`Checkpoint`] / [`CheckpointSubmission`] with [`CheckpointError`] | Checkpoint error domain separation |
//!
//! **Flat test path:** `tests/test_ser_002_to_from_bytes.rs` (STR-002 / `tests-layout.mdc`). **Related:** SER-001
//! (`tests/test_ser_001_bincode_all_types.rs`) proves raw serde round-trip; SER-002 adds the **public** infallible/fallible
//! API and typed error mapping required for storage / gossip adapters.

mod common;

use dig_block::{
    AttestedBlock, BlockError, Bytes32, Checkpoint, CheckpointError, CheckpointSubmission, Cost,
    L2Block, L2BlockHeader, PublicKey, ReceiptList, Signature, SignerBitmap,
};

use common::{test_header, test_spend_bundle};

// --- Shared assertion helpers (keep each `#[test]` readable; pattern-match proves *which* error variant fires). ---

fn assert_header_invalid_data(res: Result<L2BlockHeader, BlockError>, context: &str) {
    match res {
        Err(BlockError::InvalidData(msg)) => {
            assert!(
                !msg.is_empty(),
                "{context}: InvalidData should carry bincode error text"
            );
        }
        other => panic!("{context}: expected BlockError::InvalidData, got {other:?}"),
    }
}

fn assert_l2block_invalid_data(res: Result<L2Block, BlockError>, context: &str) {
    match res {
        Err(BlockError::InvalidData(msg)) => {
            assert!(
                !msg.is_empty(),
                "{context}: InvalidData should carry bincode error text"
            );
        }
        other => panic!("{context}: expected BlockError::InvalidData, got {other:?}"),
    }
}

fn assert_checkpoint_invalid_data(res: Result<Checkpoint, CheckpointError>, context: &str) {
    match res {
        Err(CheckpointError::InvalidData(msg)) => {
            assert!(
                !msg.is_empty(),
                "{context}: InvalidData should carry bincode error text"
            );
        }
        other => panic!("{context}: expected CheckpointError::InvalidData, got {other:?}"),
    }
}

fn assert_checkpoint_submission_invalid_data(
    res: Result<CheckpointSubmission, CheckpointError>,
    context: &str,
) {
    match res {
        Err(CheckpointError::InvalidData(msg)) => {
            assert!(
                !msg.is_empty(),
                "{context}: InvalidData should carry bincode error text"
            );
        }
        other => panic!("{context}: expected CheckpointError::InvalidData, got {other:?}"),
    }
}

fn sample_header_rich() -> L2BlockHeader {
    let t = |b: u8| Bytes32::new([b; 32]);
    L2BlockHeader::new(
        9,
        4,
        t(0x10),
        t(0x11),
        t(0x12),
        t(0x13),
        t(0x14),
        t(0x15),
        200,
        t(0x16),
        3,
        2,
        50 as Cost,
        123,
        1,
        1,
        500,
        t(0x17),
    )
}

/// **Test plan:** `to_bytes_succeeds` — header path.
#[test]
fn ser002_header_to_bytes_returns_vec() {
    let h = sample_header_rich();
    let bytes = h.to_bytes();
    assert!(
        !bytes.is_empty(),
        "SER-002: a populated header must produce non-empty bincode bytes"
    );
}

/// **Test plan:** `from_bytes_valid` — header round-trip.
#[test]
fn ser002_header_from_bytes_roundtrip_ok() {
    let h = sample_header_rich();
    let bytes = h.to_bytes();
    let back = L2BlockHeader::from_bytes(&bytes).expect("valid header bytes must decode");
    assert_eq!(h, back);
}

/// **Test plan:** `from_bytes_truncated` / `from_bytes_empty` / `from_bytes_garbage` — header negative cases.
#[test]
fn ser002_header_from_bytes_rejects_truncated_empty_and_garbage() {
    let h = sample_header_rich();
    let full = h.to_bytes();
    assert_header_invalid_data(
        L2BlockHeader::from_bytes(&full[..full.len().saturating_sub(3)]),
        "truncated header",
    );
    assert_header_invalid_data(L2BlockHeader::from_bytes(&[]), "empty header");
    assert_header_invalid_data(
        L2BlockHeader::from_bytes(&[0xff, 0xfe, 0xfd, 0xfc]),
        "garbage header",
    );
}

/// **Test plan:** `to_bytes_succeeds` / `from_bytes_valid` — full block (includes [`chia_protocol::SpendBundle`]).
#[test]
fn ser002_block_to_bytes_and_roundtrip_by_hash() {
    let h = test_header();
    let b = L2Block::new(
        h,
        vec![test_spend_bundle()],
        vec![vec![0x01, 0x02]],
        Signature::default(),
    );
    let bytes = b.to_bytes();
    assert!(
        !bytes.is_empty(),
        "SER-002: block with one spend bundle must serialize to non-empty bincode"
    );
    let back = L2Block::from_bytes(&bytes).expect("valid block bytes");
    assert_eq!(
        b.hash(),
        back.hash(),
        "round-trip must preserve canonical block id"
    );
}

#[test]
fn ser002_block_from_bytes_invalid() {
    assert_l2block_invalid_data(L2Block::from_bytes(&[]), "empty block");
}

/// **Test plan:** `from_bytes_valid` — attested wrapper (no `PartialEq` on type: use hash + byte stability).
#[test]
fn ser002_attested_block_roundtrip_stable() {
    let inner = L2Block::new(
        sample_header_rich(),
        vec![test_spend_bundle()],
        Vec::new(),
        Signature::default(),
    );
    let a = AttestedBlock::new(inner, 8, ReceiptList::default());
    let bytes = a.to_bytes();
    let back = AttestedBlock::from_bytes(&bytes).expect("valid attested block");
    assert_eq!(a.hash(), back.hash());
    assert_eq!(back.to_bytes(), bytes);
}

/// **Test plan:** `checkpoint_from_bytes_invalid` — positive + negative for [`Checkpoint`].
#[test]
fn ser002_checkpoint_to_bytes_roundtrip_and_invalid() {
    let mut c = Checkpoint::new();
    c.epoch = 7;
    c.block_count = 3;
    let bytes = c.to_bytes();
    let back = Checkpoint::from_bytes(&bytes).expect("valid checkpoint");
    assert_eq!(c, back);

    assert_checkpoint_invalid_data(Checkpoint::from_bytes(&[]), "empty checkpoint");
    assert_checkpoint_invalid_data(
        Checkpoint::from_bytes(&bytes[..bytes.len().saturating_sub(2)]),
        "truncated checkpoint",
    );
}

/// **Test plan:** `checkpoint_from_bytes_invalid` — [`CheckpointSubmission`] uses same error type on decode.
#[test]
fn ser002_checkpoint_submission_roundtrip_and_invalid() {
    let ckpt = Checkpoint::new();
    let sub = CheckpointSubmission::new(
        ckpt,
        SignerBitmap::new(4),
        Signature::default(),
        PublicKey::default(),
        42,
        0,
    );
    let bytes = sub.to_bytes();
    let back = CheckpointSubmission::from_bytes(&bytes).expect("valid submission");
    assert_eq!(back.to_bytes(), bytes);
    assert_eq!(back.epoch(), sub.epoch());

    assert_checkpoint_submission_invalid_data(
        CheckpointSubmission::from_bytes(&[]),
        "empty submission",
    );
}
