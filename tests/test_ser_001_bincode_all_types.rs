//! SER-001: Bincode serialization for all wire-bearing DIG types ([SPEC §8.1](docs/resources/SPEC.md),
//! [NORMATIVE — SER-001](docs/requirements/domains/serialization/NORMATIVE.md#ser-001-bincode-serialization-for-all-types)).
//!
//! **Authoritative spec:** `docs/requirements/domains/serialization/specs/SER-001.md`. **Flat test path:**
//! `tests/test_ser_001_bincode_all_types.rs` (STR-002 — not `tests/serialization/test_bincode.rs`).
//!
//! ## How these tests prove SER-001
//!
//! - Each case calls [`bincode::serialize`] then [`bincode::deserialize`]. Where [`PartialEq`] is available,
//!   [`assert_bincode_roundtrip`] asserts **value equality** with the original — full serde + semantic round-trip.
//!   Types embedding [`chia_protocol::SpendBundle`] use [`assert_bincode_roundtrip_stable_encoding`] instead (see below).
//!   Together this proves every listed type uses [`serde::Serialize`] + [`serde::Deserialize`] with the crate’s
//!   [`bincode`] 1.x default options ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)).
//! - **No JSON for consensus:** [`ser001_bincode_smaller_than_json_for_header`] shows bincode is materially smaller than
//!   `serde_json` for the same [`dig_block::L2BlockHeader`] — supporting the “bincode exclusively for wire/storage” rule
//!   (JSON only for ad-hoc debugging per spec notes).
//! - **`PendingAssertion` / [`dig_block::AssertionKind`]:** Exercises [`dig_block::PendingAssertion::from_condition`]
//!   on a real [`chia_sdk_types::Condition`] variant, then bincode round-trip — ties SER-001 to EXE-009’s serde surface.
//! - **Types without [`PartialEq`] (e.g. [`L2Block`]):** [`assert_bincode_roundtrip_stable_encoding`] proves
//!   `serialize → deserialize → serialize` yields **identical bytes**; [`SpendBundle`](chia_protocol::SpendBundle) from
//!   upstream does not implement equality, so byte-stable round-trip is the practical SER-001 proof for those structs.
//!
//! **Tooling:** Repomix packs for `src/`, `tests/`, `docs/requirements/domains/serialization`. `npx gitnexus impact ExecutionResult`
//! after expanding execution types → **LOW** (tests + crate root re-exports). **SocratiCode:** not available in this session.

mod common;

use bincode::{deserialize, serialize};
use chia_protocol::{Coin, CoinSpend, Program};
use chia_sdk_types::conditions::AssertHeightAbsolute;
use chia_sdk_types::Condition;
use serde::{de::DeserializeOwned, Serialize};

use dig_block::{
    AssertionKind, AttestedBlock, BlockStatus, Bytes32, Checkpoint, CheckpointStatus,
    CheckpointSubmission, Cost, ExecutionResult, L2Block, L2BlockHeader, PendingAssertion, Receipt,
    ReceiptList, ReceiptStatus, Signature, SignerBitmap,
};

use common::{test_header, test_spend_bundle};

/// **Contract:** `bincode::serialize` must succeed and `deserialize` must recover `PartialEq` equality.
fn assert_bincode_roundtrip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let bytes = serialize(value).expect("SER-001: bincode::serialize must succeed for wire types");
    // Note: bincode may produce an empty `Vec<u8>` for structs whose serde shape is empty (e.g. [`ExecutionResult`]
    // placeholder until EXE-008); emptiness is still a valid encoding as long as deserialize + equality holds.
    let back: T = deserialize(&bytes).expect("SER-001: bincode::deserialize must succeed");
    assert_eq!(*value, back);
}

/// **Contract (no `PartialEq`):** first serialization bytes must equal bytes after deserialize + re-serialize.
///
/// **Rationale:** [`dig_block::L2Block`] embeds [`chia_protocol::SpendBundle`], which intentionally omits `PartialEq`
/// in `chia-protocol` 0.26; equality on full blocks is still unnecessary for SER-001 — we only need proof that bincode
/// is a **lossless codec** for the type’s serde schema ([SER-001](docs/requirements/domains/serialization/specs/SER-001.md)).
fn assert_bincode_roundtrip_stable_encoding<T>(value: &T)
where
    T: Serialize + DeserializeOwned + std::fmt::Debug,
{
    let bytes = serialize(value).expect("SER-001: bincode::serialize must succeed for wire types");
    let back: T = deserialize(&bytes).expect("SER-001: bincode::deserialize must succeed");
    let again = serialize(&back).expect("SER-001: re-serialize after round-trip must succeed");
    assert_eq!(
        bytes, again,
        "SER-001: encoding must be stable across deserialize/re-serialize"
    );
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

/// **Test plan:** `header_bincode`
#[test]
fn ser001_l2_block_header_bincode_roundtrip() {
    assert_bincode_roundtrip(&sample_header_rich());
}

/// **Test plan:** `block_bincode`
#[test]
fn ser001_l2_block_bincode_roundtrip() {
    let h = test_header();
    let b = L2Block::new(
        h,
        vec![test_spend_bundle()],
        vec![vec![0x01, 0x02]],
        Signature::default(),
    );
    assert_bincode_roundtrip_stable_encoding(&b);
}

/// **Test plan:** `attested_bincode`
#[test]
fn ser001_attested_block_bincode_roundtrip() {
    let inner = L2Block::new(
        sample_header_rich(),
        vec![test_spend_bundle()],
        Vec::new(),
        Signature::default(),
    );
    let att = AttestedBlock::new(inner, 42, ReceiptList::default());
    assert_bincode_roundtrip_stable_encoding(&att);
}

/// **Test plan:** `checkpoint_bincode`
#[test]
fn ser001_checkpoint_bincode_roundtrip() {
    let mut c = Checkpoint::new();
    c.epoch = 5;
    c.block_count = 100;
    assert_bincode_roundtrip(&c);
}

/// **Test plan:** `submission_bincode`
#[test]
fn ser001_checkpoint_submission_bincode_roundtrip() {
    let ckpt = Checkpoint::new();
    let bitmap = SignerBitmap::new(4);
    let sub = CheckpointSubmission::new(
        ckpt,
        bitmap,
        Signature::default(),
        dig_block::PublicKey::default(),
        99,
        1,
    );
    assert_bincode_roundtrip_stable_encoding(&sub);
}

/// **Test plan:** `signer_bitmap_bincode`
#[test]
fn ser001_signer_bitmap_bincode_roundtrip() {
    let mut b = SignerBitmap::new(8);
    b.set_signed(3).expect("valid index");
    assert_bincode_roundtrip(&b);
}

/// **Test plan:** `receipt_bincode`
#[test]
fn ser001_receipt_and_receipt_list_bincode_roundtrip() {
    let r = Receipt::new(
        Bytes32::new([0xaa; 32]),
        1,
        0,
        ReceiptStatus::Success,
        0,
        Bytes32::new([0xbb; 32]),
        0,
    );
    assert_bincode_roundtrip(&r);
    let list = ReceiptList::from_receipts(vec![r]);
    assert_bincode_roundtrip(&list);
}

/// **Test plan:** `status_bincode`
#[test]
fn ser001_block_and_checkpoint_status_bincode_roundtrip() {
    assert_bincode_roundtrip(&BlockStatus::Validated);
    assert_bincode_roundtrip(&CheckpointStatus::Collecting);
}

/// **Test plan:** `execution_result_bincode`
#[test]
fn ser001_execution_result_bincode_roundtrip() {
    let ex = ExecutionResult::default();
    assert_bincode_roundtrip(&ex);
}

/// **Test plan:** `PendingAssertion` / EXE-009 serde + `from_condition`
#[test]
fn ser001_pending_assertion_bincode_and_from_condition() {
    let coin = Coin::new(Bytes32::new([0x03; 32]), Bytes32::new([0x04; 32]), 1);
    let spend = CoinSpend::new(coin, Program::from(vec![1]), Program::from(vec![0x80]));
    let cond: Condition<()> = Condition::AssertHeightAbsolute(AssertHeightAbsolute { height: 0 });
    let p = PendingAssertion::from_condition(&cond, &spend).expect("height absolute maps");
    assert_eq!(p.coin_id, coin.coin_id());
    assert_eq!(p.kind, AssertionKind::HeightAbsolute(0));
    assert_bincode_roundtrip(&p);

    // `AssertEphemeral` is a real consensus condition but not one of the eight height/time locks (EXE-009 mapping).
    let non_lock = Condition::<()>::AssertEphemeral(Default::default());
    assert!(
        PendingAssertion::from_condition(&non_lock, &spend).is_none(),
        "non height/time conditions must not produce PendingAssertion"
    );
}

/// **Test plan:** `no_schema_overhead` — bincode vs JSON byte size (debugging format must not be smaller).
#[test]
fn ser001_bincode_smaller_than_json_for_header() {
    let h = sample_header_rich();
    let bc = serialize(&h).expect("bincode");
    let js = serde_json::to_string(&h).expect("json for size probe only");
    assert!(
        bc.len() < js.len(),
        "bincode ({}) should be smaller than JSON ({}) for the same header",
        bc.len(),
        js.len()
    );
}
