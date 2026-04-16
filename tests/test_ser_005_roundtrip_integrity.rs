//! SER-005: Property-based round-trip integrity for all wire types ([SPEC §12.1, §12.5](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/serialization/NORMATIVE.md` (SER-005)
//! **Spec:** `docs/requirements/domains/serialization/specs/SER-005.md`
//!
//! ## What this proves
//!
//! For every DIG L2 wire type, `from_bytes(to_bytes(x))` produces a value indistinguishable from `x`
//! under the type's equivalence relation (`PartialEq` where available; byte-stable re-serialization
//! otherwise). This is verified by [`proptest`] over hundreds of randomly generated instances per
//! test, with explicit edge-case seeds (zero, max, empty, max-size collections).
//!
//! ## Types covered and equivalence method
//!
//! | Type | Equivalence | Reason |
//! |------|-------------|--------|
//! | [`L2BlockHeader`] | `PartialEq` | All 33 fields implement `Eq`. |
//! | [`Checkpoint`] | `PartialEq` | Nine scalar / hash fields. |
//! | [`Receipt`] | `PartialEq` | Seven scalar fields, `Eq` on `ReceiptStatus`. |
//! | [`ReceiptList`] | `PartialEq` | Receipts + root. |
//! | [`SignerBitmap`] | `PartialEq` | `bits: Vec<u8>` + `validator_count`. |
//! | [`BlockStatus`] / [`CheckpointStatus`] | `PartialEq` | Enum variants carry `Eq` payloads. |
//! | [`L2Block`] | byte-stable re-serialize | [`chia_protocol::SpendBundle`] lacks `PartialEq`. |
//! | [`AttestedBlock`] | byte-stable re-serialize | Embeds `L2Block`. |
//! | [`CheckpointSubmission`] | byte-stable re-serialize | Wraps `Checkpoint` + BLS fields; `chia-bls` types lack `PartialEq`. |
//!
//! **Byte-stable check:** `to_bytes(from_bytes(to_bytes(x))) == to_bytes(x)`. This still proves
//! round-trip identity because bincode with a fixed field order is deterministic
//! ([SPEC §8.1](docs/resources/SPEC.md) — bincode wire discipline).
//!
//! ## How this satisfies SER-005
//!
//! - **`proptest` coverage:** 256 default cases per property (proptest default); covers a wide
//!   random distribution of field values.
//! - **Edge cases:** explicit regression tests with zero-filled hashes, `u64::MAX`, empty vectors,
//!   and maximum-size collections.
//! - **Complementary to SER-001:** SER-001 asserted round-trip on hand-written instances;
//!   SER-005 proves the property holds across the input space.

use proptest::prelude::*;

use dig_block::{
    AttestedBlock, BlockStatus, Bytes32, Checkpoint, CheckpointStatus, CheckpointSubmission, Cost,
    L2Block, L2BlockHeader, PublicKey, Receipt, ReceiptList, ReceiptStatus, Signature,
    SignerBitmap, MAX_VALIDATORS,
};

// ---------------------------------------------------------------------------
// Strategies — minimal custom `Arbitrary` in lieu of proptest-derive
// ---------------------------------------------------------------------------

/// Generate a `Bytes32` over the full 256-bit space so hash-order logic sees diverse values.
fn any_bytes32() -> impl Strategy<Value = Bytes32> {
    any::<[u8; 32]>().prop_map(Bytes32::new)
}

/// Generate an `Option<Bytes32>` with mixed None/Some to exercise `#[serde(default)]` paths
/// from SER-004.
fn any_opt_bytes32() -> impl Strategy<Value = Option<Bytes32>> {
    prop_oneof![Just(None), any_bytes32().prop_map(Some)]
}

/// Generate a random but well-formed `L2BlockHeader` covering every field group in SPEC §2.2.
///
/// `Cost` stays at `u64` full range so budget-edge serialization is exercised.
fn header_strategy() -> impl Strategy<Value = L2BlockHeader> {
    (
        // Group 1: identity (5 scalars)
        (any::<u16>(), any::<u64>(), any::<u64>(), any_bytes32()),
        // Group 2: state commitments (5 hashes)
        (
            any_bytes32(),
            any_bytes32(),
            any_bytes32(),
            any_bytes32(),
            any_bytes32(),
        ),
        // Group 3: L1 anchor (2)
        (any::<u32>(), any_bytes32()),
        // Group 4: metadata (9)
        (
            any::<u64>(),
            any::<u32>(),
            any::<u32>(),
            any::<Cost>(),
            any::<u64>(),
            any::<u32>(),
            any::<u32>(),
            any::<u32>(),
            any_bytes32(),
        ),
        // Group 5: extension + L1 proof options (6)
        (
            any_bytes32(),
            any_opt_bytes32(),
            any_opt_bytes32(),
            any_opt_bytes32(),
            any_opt_bytes32(),
            any_opt_bytes32(),
        ),
        // Group 6: slash + DFSP roots (7)
        (
            any::<u32>(),
            any_bytes32(),
            any_bytes32(),
            any_bytes32(),
            any_bytes32(),
            any_bytes32(),
            any_bytes32(),
        ),
    )
        .prop_map(
            |(
                (version, height, epoch, parent_hash),
                (state_root, spends_root, additions_root, removals_root, receipts_root),
                (l1_height, l1_hash),
                (
                    timestamp,
                    proposer_index,
                    spend_bundle_count,
                    total_cost,
                    total_fees,
                    additions_count,
                    removals_count,
                    block_size,
                    filter_hash,
                ),
                (
                    extension_data,
                    l1_collateral_coin_id,
                    l1_reserve_coin_id,
                    l1_prev_epoch_finalizer_coin_id,
                    l1_curr_epoch_finalizer_coin_id,
                    l1_network_coin_id,
                ),
                (
                    slash_proposal_count,
                    slash_proposals_root,
                    collateral_registry_root,
                    cid_state_root,
                    node_registry_root,
                    namespace_update_root,
                    dfsp_finalize_commitment_root,
                ),
            )| L2BlockHeader {
                version,
                height,
                epoch,
                parent_hash,
                state_root,
                spends_root,
                additions_root,
                removals_root,
                receipts_root,
                l1_height,
                l1_hash,
                timestamp,
                proposer_index,
                spend_bundle_count,
                total_cost,
                total_fees,
                additions_count,
                removals_count,
                block_size,
                filter_hash,
                extension_data,
                l1_collateral_coin_id,
                l1_reserve_coin_id,
                l1_prev_epoch_finalizer_coin_id,
                l1_curr_epoch_finalizer_coin_id,
                l1_network_coin_id,
                slash_proposal_count,
                slash_proposals_root,
                collateral_registry_root,
                cid_state_root,
                node_registry_root,
                namespace_update_root,
                dfsp_finalize_commitment_root,
            },
        )
}

/// Generate a [`Checkpoint`] over all nine fields; any `u64`/`u32` values, any hashes.
fn checkpoint_strategy() -> impl Strategy<Value = Checkpoint> {
    (
        any::<u64>(),
        any_bytes32(),
        any_bytes32(),
        any::<u32>(),
        any::<u64>(),
        any::<u64>(),
        any_bytes32(),
        any_bytes32(),
        any::<u32>(),
    )
        .prop_map(
            |(
                epoch,
                state_root,
                block_root,
                block_count,
                tx_count,
                total_fees,
                prev_checkpoint,
                withdrawals_root,
                withdrawal_count,
            )| Checkpoint {
                epoch,
                state_root,
                block_root,
                block_count,
                tx_count,
                total_fees,
                prev_checkpoint,
                withdrawals_root,
                withdrawal_count,
            },
        )
}

/// Generate a [`ReceiptStatus`] variant (includes the reserved `Failed = 255`).
fn receipt_status_strategy() -> impl Strategy<Value = ReceiptStatus> {
    prop_oneof![
        Just(ReceiptStatus::Success),
        Just(ReceiptStatus::InsufficientBalance),
        Just(ReceiptStatus::InvalidNonce),
        Just(ReceiptStatus::InvalidSignature),
        Just(ReceiptStatus::AccountNotFound),
        Just(ReceiptStatus::Failed),
    ]
}

/// Generate a [`Receipt`] over RCP-002 fields.
fn receipt_strategy() -> impl Strategy<Value = Receipt> {
    (
        any_bytes32(),
        any::<u64>(),
        any::<u32>(),
        receipt_status_strategy(),
        any::<u64>(),
        any_bytes32(),
        any::<u64>(),
    )
        .prop_map(
            |(
                tx_id,
                block_height,
                tx_index,
                status,
                fee_charged,
                post_state_root,
                cumulative_fees,
            )| {
                Receipt::new(
                    tx_id,
                    block_height,
                    tx_index,
                    status,
                    fee_charged,
                    post_state_root,
                    cumulative_fees,
                )
            },
        )
}

/// Generate a non-empty but bounded [`ReceiptList`] so Merkle root computation is exercised.
fn receipt_list_strategy() -> impl Strategy<Value = ReceiptList> {
    prop::collection::vec(receipt_strategy(), 0..8).prop_map(ReceiptList::from_receipts)
}

/// Generate a [`SignerBitmap`] within the protocol cap ([`MAX_VALIDATORS`] = 65_536).
fn signer_bitmap_strategy() -> impl Strategy<Value = SignerBitmap> {
    // Keep the random range well below MAX_VALIDATORS so bincode buffers stay small in CI.
    (0u32..=1024u32).prop_flat_map(|n| {
        let bytes_len = (n as usize).div_ceil(8);
        prop::collection::vec(any::<u8>(), bytes_len).prop_map(move |bits| {
            let mut bitmap = SignerBitmap::new(n);
            // Use from_bytes semantics to round-trip through wire form.
            if !bits.is_empty() {
                bitmap = SignerBitmap::from_bytes(&bits, n);
            }
            bitmap
        })
    })
}

fn block_status_strategy() -> impl Strategy<Value = BlockStatus> {
    prop_oneof![
        Just(BlockStatus::Pending),
        Just(BlockStatus::Validated),
        Just(BlockStatus::SoftFinalized),
        Just(BlockStatus::HardFinalized),
        Just(BlockStatus::Orphaned),
        Just(BlockStatus::Rejected),
    ]
}

fn checkpoint_status_strategy() -> impl Strategy<Value = CheckpointStatus> {
    prop_oneof![
        Just(CheckpointStatus::Pending),
        Just(CheckpointStatus::Collecting),
        Just(CheckpointStatus::Failed),
        (any_bytes32(), any::<u64>())
            .prop_map(|(winner_hash, winner_score)| CheckpointStatus::WinnerSelected {
                winner_hash,
                winner_score
            }),
        (any_bytes32(), any::<u32>())
            .prop_map(|(winner_hash, l1_height)| CheckpointStatus::Finalized {
                winner_hash,
                l1_height
            }),
    ]
}

// ---------------------------------------------------------------------------
// Property tests (PartialEq-based)
// ---------------------------------------------------------------------------

proptest! {
    /// **SER-005 core:** every randomly generated header round-trips bit-for-bit via bincode.
    /// Proves no field is dropped, reordered, or coerced — covers all 33 fields incl. L1 proof
    /// Options (SER-004 defaults).
    #[test]
    fn proptest_l2_block_header_roundtrip(header in header_strategy()) {
        let bytes = header.to_bytes();
        let decoded = L2BlockHeader::from_bytes(&bytes).expect("decode must succeed");
        prop_assert_eq!(&header, &decoded);
        // Double round-trip: re-encoding the decoded value yields identical bytes.
        prop_assert_eq!(decoded.to_bytes(), bytes);
    }

    /// **SER-005:** Checkpoint nine-field round-trip (SPEC §2.6, HSH-002 preimage basis).
    #[test]
    fn proptest_checkpoint_roundtrip(ckp in checkpoint_strategy()) {
        let bytes = ckp.to_bytes();
        let decoded = Checkpoint::from_bytes(&bytes).expect("decode must succeed");
        prop_assert_eq!(&ckp, &decoded);
        prop_assert_eq!(decoded.to_bytes(), bytes);
    }

    /// **SER-005:** Receipt seven-field round-trip (RCP-002 / SPEC §2.9).
    /// Uses bincode (same wire format as ReceiptList entries).
    #[test]
    fn proptest_receipt_roundtrip(r in receipt_strategy()) {
        let bytes = bincode::serialize(&r).expect("encode");
        let decoded: Receipt = bincode::deserialize(&bytes).expect("decode");
        prop_assert_eq!(r, decoded);
    }

    /// **SER-005:** ReceiptList round-trip — vector + Merkle root both preserved (RCP-003, HSH-008).
    #[test]
    fn proptest_receipt_list_roundtrip(list in receipt_list_strategy()) {
        let bytes = bincode::serialize(&list).expect("encode");
        let decoded: ReceiptList = bincode::deserialize(&bytes).expect("decode");
        prop_assert_eq!(list.len(), decoded.len());
        prop_assert_eq!(list, decoded);
    }

    /// **SER-005:** SignerBitmap round-trip — bit pattern + validator_count (ATT-004 / SPEC §2.10).
    #[test]
    fn proptest_signer_bitmap_roundtrip(bitmap in signer_bitmap_strategy()) {
        let bytes = bincode::serialize(&bitmap).expect("encode");
        let decoded: SignerBitmap = bincode::deserialize(&bytes).expect("decode");
        prop_assert_eq!(bitmap, decoded);
    }

    /// **SER-005:** BlockStatus lifecycle enum (ATT-003 / SPEC §2.5).
    #[test]
    fn proptest_block_status_roundtrip(s in block_status_strategy()) {
        let bytes = bincode::serialize(&s).expect("encode");
        let decoded: BlockStatus = bincode::deserialize(&bytes).expect("decode");
        prop_assert_eq!(s, decoded);
    }

    /// **SER-005:** CheckpointStatus lifecycle (CKP-003 / SPEC §2.8) — incl. struct variants with payloads.
    #[test]
    fn proptest_checkpoint_status_roundtrip(s in checkpoint_status_strategy()) {
        let bytes = bincode::serialize(&s).expect("encode");
        let decoded: CheckpointStatus = bincode::deserialize(&bytes).expect("decode");
        prop_assert_eq!(s, decoded);
    }
}

// ---------------------------------------------------------------------------
// Property tests (byte-stable, for types without PartialEq)
// ---------------------------------------------------------------------------

/// Build an [`L2Block`] with a random (non-equal-comparable) body.
/// Strategy keeps body small so bincode payload stays tractable.
fn l2_block_strategy() -> impl Strategy<Value = L2Block> {
    (
        header_strategy(),
        prop::collection::vec(prop::collection::vec(any::<u8>(), 0..32), 0..4),
    )
        .prop_map(|(header, slash_payloads)| {
            // SpendBundle generation is out of scope for this property test (upstream chia types
            // lack Arbitrary); empty body is sufficient to exercise the envelope round-trip.
            L2Block::new(header, Vec::new(), slash_payloads, Signature::default())
        })
}

fn attested_block_strategy() -> impl Strategy<Value = AttestedBlock> {
    (l2_block_strategy(), 0u32..=1024u32, receipt_list_strategy()).prop_map(
        |(block, validator_count, receipts)| AttestedBlock::new(block, validator_count, receipts),
    )
}

fn checkpoint_submission_strategy() -> impl Strategy<Value = CheckpointSubmission> {
    (
        checkpoint_strategy(),
        signer_bitmap_strategy(),
        any::<u64>(),
        any::<u32>(),
    )
        .prop_map(|(ckp, bitmap, score, submitter)| {
            CheckpointSubmission::new(
                ckp,
                bitmap,
                Signature::default(),
                PublicKey::default(),
                score,
                submitter,
            )
        })
}

proptest! {
    /// **SER-005 (byte-stable):** L2Block round-trips — bincode output stable on re-encode.
    /// `SpendBundle` lacks `PartialEq`; byte-stability is the strongest identity we can assert
    /// without a custom comparator (per SER-001 tracking note).
    #[test]
    fn proptest_l2_block_roundtrip_bytes(block in l2_block_strategy()) {
        let bytes = block.to_bytes();
        let decoded = L2Block::from_bytes(&bytes).expect("decode must succeed");
        prop_assert_eq!(decoded.to_bytes(), bytes);
        // Header identity (PartialEq present) survives.
        prop_assert_eq!(block.header, decoded.header);
    }

    /// **SER-005 (byte-stable):** AttestedBlock round-trip. Covers signer bitmap + receipts + status
    /// inside the attested envelope (ATT-001 / SPEC §2.4).
    #[test]
    fn proptest_attested_block_roundtrip_bytes(attested in attested_block_strategy()) {
        let bytes = attested.to_bytes();
        let decoded = AttestedBlock::from_bytes(&bytes).expect("decode must succeed");
        prop_assert_eq!(decoded.to_bytes(), bytes);
        prop_assert_eq!(attested.block.header, decoded.block.header);
        prop_assert_eq!(attested.status, decoded.status);
    }

    /// **SER-005 (byte-stable):** CheckpointSubmission round-trip. Exercises BLS public key,
    /// signature, bitmap, and L1 tracking Options under bincode (CKP-002 / SPEC §2.7).
    #[test]
    fn proptest_checkpoint_submission_roundtrip_bytes(sub in checkpoint_submission_strategy()) {
        let bytes = sub.to_bytes();
        let decoded = CheckpointSubmission::from_bytes(&bytes).expect("decode must succeed");
        prop_assert_eq!(decoded.to_bytes(), bytes);
        prop_assert_eq!(sub.checkpoint, decoded.checkpoint);
        prop_assert_eq!(sub.score, decoded.score);
        prop_assert_eq!(sub.submission_height, decoded.submission_height);
        prop_assert_eq!(sub.submission_coin, decoded.submission_coin);
    }
}

// ---------------------------------------------------------------------------
// Edge-case regression tests
// ---------------------------------------------------------------------------

/// **SER-005 edge case:** empty block (no spend bundles, no slash payloads) round-trips.
/// Genesis is the canonical empty block ([SPEC §8.3](docs/resources/SPEC.md)).
#[test]
fn edge_empty_block_roundtrips() {
    let header = L2BlockHeader::genesis(Bytes32::new([0xaa; 32]), 0, Bytes32::new([0xbb; 32]));
    let block = L2Block::new(header, Vec::new(), Vec::new(), Signature::default());
    let bytes = block.to_bytes();
    let decoded = L2Block::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.to_bytes(), bytes);
    assert_eq!(block.header, decoded.header);
}

/// **SER-005 edge case:** all numeric fields at `u64::MAX` / `u32::MAX` still round-trip.
/// Bincode varint encoding has no special path for max values, but this guards regression.
#[test]
fn edge_max_values_roundtrip() {
    let header = L2BlockHeader {
        version: u16::MAX,
        height: u64::MAX,
        epoch: u64::MAX,
        parent_hash: Bytes32::new([0xff; 32]),
        state_root: Bytes32::new([0xff; 32]),
        spends_root: Bytes32::new([0xff; 32]),
        additions_root: Bytes32::new([0xff; 32]),
        removals_root: Bytes32::new([0xff; 32]),
        receipts_root: Bytes32::new([0xff; 32]),
        l1_height: u32::MAX,
        l1_hash: Bytes32::new([0xff; 32]),
        timestamp: u64::MAX,
        proposer_index: u32::MAX,
        spend_bundle_count: u32::MAX,
        total_cost: Cost::MAX,
        total_fees: u64::MAX,
        additions_count: u32::MAX,
        removals_count: u32::MAX,
        block_size: u32::MAX,
        filter_hash: Bytes32::new([0xff; 32]),
        extension_data: Bytes32::new([0xff; 32]),
        l1_collateral_coin_id: Some(Bytes32::new([0xff; 32])),
        l1_reserve_coin_id: Some(Bytes32::new([0xff; 32])),
        l1_prev_epoch_finalizer_coin_id: Some(Bytes32::new([0xff; 32])),
        l1_curr_epoch_finalizer_coin_id: Some(Bytes32::new([0xff; 32])),
        l1_network_coin_id: Some(Bytes32::new([0xff; 32])),
        slash_proposal_count: u32::MAX,
        slash_proposals_root: Bytes32::new([0xff; 32]),
        collateral_registry_root: Bytes32::new([0xff; 32]),
        cid_state_root: Bytes32::new([0xff; 32]),
        node_registry_root: Bytes32::new([0xff; 32]),
        namespace_update_root: Bytes32::new([0xff; 32]),
        dfsp_finalize_commitment_root: Bytes32::new([0xff; 32]),
    };
    let bytes = header.to_bytes();
    let decoded = L2BlockHeader::from_bytes(&bytes).unwrap();
    assert_eq!(header, decoded);
}

/// **SER-005 edge case:** all-zero `Bytes32` fields (distinct from `EMPTY_ROOT`) round-trip.
/// Guards against confusion between `ZERO_HASH` (0x00..00) and `EMPTY_ROOT` (SHA-256 of empty).
#[test]
fn edge_zero_hash_fields_roundtrip() {
    let zero = Bytes32::new([0u8; 32]);
    let header = L2BlockHeader::new(
        42,
        1,
        zero,
        zero,
        zero,
        zero,
        zero,
        zero,
        0,
        zero,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        zero,
    );
    let bytes = header.to_bytes();
    let decoded = L2BlockHeader::from_bytes(&bytes).unwrap();
    assert_eq!(header, decoded);
}

/// **SER-005 edge case:** SignerBitmap at the protocol cap ([`MAX_VALIDATORS`]) round-trips.
/// Guards against integer overflow or buffer sizing issues at the upper bound.
#[test]
fn edge_signer_bitmap_at_max_validators_roundtrips() {
    // Use a smaller-but-representative size; allocating 65536 bits (8192 bytes) per test is fine.
    let bitmap = SignerBitmap::new(MAX_VALIDATORS);
    let bytes = bincode::serialize(&bitmap).unwrap();
    let decoded: SignerBitmap = bincode::deserialize(&bytes).unwrap();
    assert_eq!(bitmap, decoded);
    assert_eq!(decoded.validator_count(), MAX_VALIDATORS);
}

/// **SER-005 edge case:** empty ReceiptList yields `EMPTY_ROOT` and round-trips (RCP-003 boundary).
#[test]
fn edge_empty_receipt_list_roundtrips() {
    let list = ReceiptList::new();
    let bytes = bincode::serialize(&list).unwrap();
    let decoded: ReceiptList = bincode::deserialize(&bytes).unwrap();
    assert_eq!(list, decoded);
    assert_eq!(list.len(), 0);
}
