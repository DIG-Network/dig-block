//! CKP-001: [`dig_block::Checkpoint`] — nine-field epoch summary per NORMATIVE § CKP-001.
//!
//! **Normative:** `docs/requirements/domains/checkpoint/NORMATIVE.md` (CKP-001)  
//! **Spec + test plan:** `docs/requirements/domains/checkpoint/specs/CKP-001.md`  
//! **Implementation:** `src/types/checkpoint.rs`
//!
//! ## How these tests prove CKP-001
//!
//! - **`new()` defaults:** [CKP-001 acceptance](docs/requirements/domains/checkpoint/specs/CKP-001.md#acceptance-criteria) requires epoch `0`, zero counts, and [`Bytes32::default`] for roots — exercised in [`ckp001_new_sets_defaults`].
//! - **Read/write surface:** All fields are `pub`; mutating each field and reading it back proves the requirement that values persist ([CKP-001 § Test Plan — Field assignment](docs/requirements/domains/checkpoint/specs/CKP-001.md#test-plan)).
//! - **Nine-field shape:** [`read_all_nine_fields`] returns a 9-tuple of every field; if a field is removed or renamed, this file fails to compile ([CKP-001 § Test Plan — Struct size](docs/requirements/domains/checkpoint/specs/CKP-001.md#test-plan)).
//! - **Bincode / serde:** [`Checkpoint`] MUST remain serde-compatible for SER-001; a round-trip guards against accidental non-serializable field types ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md) — forward alignment, not a substitute for future SER-002 `to_bytes` tests).

use dig_block::{Bytes32, Checkpoint};

fn byte_tag(b: u8) -> Bytes32 {
    Bytes32::new([b; 32])
}

/// Compile-time proof that [`Checkpoint`] exposes exactly the nine fields named in CKP-001.
#[must_use]
fn read_all_nine_fields(
    c: &Checkpoint,
) -> (u64, Bytes32, Bytes32, u32, u64, u64, Bytes32, Bytes32, u32) {
    (
        c.epoch,
        c.state_root,
        c.block_root,
        c.block_count,
        c.tx_count,
        c.total_fees,
        c.prev_checkpoint,
        c.withdrawals_root,
        c.withdrawal_count,
    )
}

/// **Test plan:** “Default construction” — [`Checkpoint::new`] yields zeros / default hashes.
#[test]
fn ckp001_new_sets_defaults() {
    let c = Checkpoint::new();
    assert_eq!(c.epoch, 0);
    assert_eq!(c.state_root, Bytes32::default());
    assert_eq!(c.block_root, Bytes32::default());
    assert_eq!(c.block_count, 0);
    assert_eq!(c.tx_count, 0);
    assert_eq!(c.total_fees, 0);
    assert_eq!(c.prev_checkpoint, Bytes32::default());
    assert_eq!(c.withdrawals_root, Bytes32::default());
    assert_eq!(c.withdrawal_count, 0);
}

/// **Test plan:** “Field assignment” — each field accepts a distinct value and reads back identically.
#[test]
fn ckp001_each_field_roundtrip_after_mutation() {
    let mut c = Checkpoint::new();
    c.epoch = 7;
    c.state_root = byte_tag(0x10);
    c.block_root = byte_tag(0x11);
    c.block_count = 100;
    c.tx_count = 9_999;
    c.total_fees = 1_234_567;
    c.prev_checkpoint = byte_tag(0x12);
    c.withdrawals_root = byte_tag(0x13);
    c.withdrawal_count = 3;

    let tuple = read_all_nine_fields(&c);
    assert_eq!(tuple.0, 7);
    assert_eq!(tuple.1, byte_tag(0x10));
    assert_eq!(tuple.2, byte_tag(0x11));
    assert_eq!(tuple.3, 100);
    assert_eq!(tuple.4, 9_999);
    assert_eq!(tuple.5, 1_234_567);
    assert_eq!(tuple.6, byte_tag(0x12));
    assert_eq!(tuple.7, byte_tag(0x13));
    assert_eq!(tuple.8, 3);
}

/// **Test plan:** “Struct size” / nine fields — tuple extraction touches every field; arity is fixed at nine.
#[test]
fn ckp001_nine_fields_tuple_extraction() {
    let c = Checkpoint::new();
    let (
        epoch,
        state_root,
        block_root,
        block_count,
        tx_count,
        total_fees,
        prev_checkpoint,
        withdrawals_root,
        withdrawal_count,
    ) = read_all_nine_fields(&c);
    assert_eq!(epoch, 0);
    assert_eq!(state_root, Bytes32::default());
    assert_eq!(block_root, Bytes32::default());
    assert_eq!(block_count, 0);
    assert_eq!(tx_count, 0);
    assert_eq!(total_fees, 0);
    assert_eq!(prev_checkpoint, Bytes32::default());
    assert_eq!(withdrawals_root, Bytes32::default());
    assert_eq!(withdrawal_count, 0);
}

/// Serde/bincode integrity — ensures the wire shape stays deserializable (SER-001 alignment).
#[test]
fn ckp001_bincode_roundtrip() {
    let mut c = Checkpoint::new();
    c.epoch = 42;
    c.state_root = byte_tag(0xaa);
    c.block_root = byte_tag(0xbb);
    c.block_count = 5;
    c.tx_count = 50;
    c.total_fees = 500;
    c.prev_checkpoint = byte_tag(0xcc);
    c.withdrawals_root = byte_tag(0xdd);
    c.withdrawal_count = 2;

    let bytes = bincode::serialize(&c).expect("bincode serialize");
    let back: Checkpoint = bincode::deserialize(&bytes).expect("bincode deserialize");
    assert_eq!(back, c);
}
