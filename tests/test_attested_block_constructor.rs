//! ATT-001: [`AttestedBlock`] field layout and [`AttestedBlock::new`] initialization rules.
//!
//! **Normative:** `docs/requirements/domains/attestation/NORMATIVE.md` (ATT-001)  
//! **Spec + test plan:** `docs/requirements/domains/attestation/specs/ATT-001.md`  
//! **Verification:** `docs/requirements/domains/attestation/VERIFICATION.md` (ATT-001)
//!
//! Each test maps to the ATT-001 **Test Plan** table or an acceptance bullet: constructor wiring, empty
//! bitmap, `Pending` status, and proposer signature bootstrap for `aggregate_signature`.

use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use dig_block::{
    AttestedBlock, BlockStatus, Bytes32, Cost, L2Block, L2BlockHeader, ReceiptList, Signature,
};

/// Minimal [`SpendBundle`] for typing — same pattern as BLK-003 attestation tests (not consensus-valid).
fn minimal_spend_bundle() -> SpendBundle {
    let coin = Coin::new(Bytes32::default(), Bytes32::default(), 1);
    let coin_spend = CoinSpend::new(coin, Program::from(vec![0x01]), Program::from(vec![0x80]));
    SpendBundle::new(vec![coin_spend], Signature::default())
}

/// Deterministic header for [`L2Block::new`] (height / epoch arbitrary but stable).
fn sample_header() -> L2BlockHeader {
    let tag = |b: u8| Bytes32::new([b; 32]);
    L2BlockHeader::new(
        7,
        3,
        tag(0x01),
        tag(0x02),
        tag(0x03),
        tag(0x04),
        tag(0x05),
        tag(0x06),
        100,
        tag(0x07),
        2,
        1,
        100 as Cost,
        0,
        0,
        0,
        0,
        tag(0x08),
    )
}

/// **Test plan:** “Construct with valid inputs” — `new` stores `block` and `receipts`; other fields follow ATT-001.
#[test]
fn test_attested_block_new_stores_block_and_receipts() {
    let header = sample_header();
    let proposer_sig = Signature::default();
    let block = L2Block::new(
        header,
        vec![minimal_spend_bundle()],
        vec![vec![0xab]],
        proposer_sig,
    );
    let receipts = ReceiptList::default();

    let attested = AttestedBlock::new(block.clone(), 100, receipts.clone());

    assert_eq!(attested.block.header, block.header);
    assert_eq!(
        attested.block.spend_bundles.len(),
        block.spend_bundles.len()
    );
    assert_eq!(attested.receipts, receipts);
}

/// **Test plan:** “Verify empty bitmap” — fresh bitmap has zero signers for the given `validator_count`.
#[test]
fn test_attested_block_new_empty_signer_bitmap() {
    let block = L2Block::new(sample_header(), vec![], vec![], Signature::default());
    let attested = AttestedBlock::new(block, 64, ReceiptList::default());
    assert_eq!(attested.signer_bitmap.signer_count(), 0);
    assert_eq!(attested.signer_bitmap.validator_count(), 64);
}

/// **Test plan:** “Verify Pending status” — constructor does not skip to validated/finalized states.
#[test]
fn test_attested_block_new_status_pending() {
    let block = L2Block::new(sample_header(), vec![], vec![], Signature::default());
    let attested = AttestedBlock::new(block, 8, ReceiptList::default());
    assert_eq!(attested.status, BlockStatus::Pending);
}

/// **Test plan:** “Verify proposer signature” — `aggregate_signature` must equal the block’s proposer signature.
#[test]
fn test_attested_block_aggregate_signature_matches_proposer() {
    let proposer_sig = Signature::default();
    let block = L2Block::new(sample_header(), vec![], vec![], proposer_sig.clone());
    let attested = AttestedBlock::new(block, 10, ReceiptList::default());
    assert_eq!(attested.aggregate_signature, proposer_sig);
    // Proof obligation: same bytes as embedded block field (clone identity).
    assert_eq!(
        attested.aggregate_signature,
        attested.block.proposer_signature
    );
}
