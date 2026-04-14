//! ATT-002: [`AttestedBlock`] query API — signing progress, soft finality, hash identity.
//!
//! **Normative:** `docs/requirements/domains/attestation/NORMATIVE.md` (ATT-002)  
//! **Spec + test plan:** `docs/requirements/domains/attestation/specs/ATT-002.md`  
//! **Verification:** `docs/requirements/domains/attestation/VERIFICATION.md` (ATT-002)
//!
//! Tests mirror the ATT-002 **Test Plan** and acceptance criteria: delegation to [`SignerBitmap`] for
//! percentages / thresholds, and [`AttestedBlock::hash`] equivalence to [`L2Block::hash`].

use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use dig_block::{AttestedBlock, Bytes32, Cost, L2Block, L2BlockHeader, ReceiptList, Signature};

fn minimal_spend_bundle() -> SpendBundle {
    let coin = Coin::new(Bytes32::default(), Bytes32::default(), 1);
    let coin_spend = CoinSpend::new(coin, Program::from(vec![0x01]), Program::from(vec![0x80]));
    SpendBundle::new(vec![coin_spend], Signature::default())
}

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

fn sample_block() -> L2Block {
    L2Block::new(
        sample_header(),
        vec![minimal_spend_bundle()],
        vec![],
        Signature::default(),
    )
}

/// **Test plan:** “Percentage with no signers” — fresh [`AttestedBlock`] → `0%`.
#[test]
fn test_signing_percentage_zero_with_no_signers() {
    let attested = AttestedBlock::new(sample_block(), 100, ReceiptList::default());
    assert_eq!(attested.signing_percentage(), 0);
}

/// **Test plan:** “Percentage with all signers” — every index set → `100%`.
#[test]
fn test_signing_percentage_hundred_when_all_signed() {
    let mut attested = AttestedBlock::new(sample_block(), 4, ReceiptList::default());
    for i in 0_u32..4 {
        attested.signer_bitmap.set_signed(i).unwrap();
    }
    assert_eq!(attested.signing_percentage(), 100);
}

/// **Test plan:** “Percentage with partial signers” — half of 10 validators → `50%`.
#[test]
fn test_signing_percentage_partial_half() {
    let mut attested = AttestedBlock::new(sample_block(), 10, ReceiptList::default());
    for i in 0_u32..5 {
        attested.signer_bitmap.set_signed(i).unwrap();
    }
    assert_eq!(attested.signing_percentage(), 50);
}

/// **Acceptance:** `signing_percentage()` delegates to the embedded bitmap (same numeric result).
#[test]
fn test_signing_percentage_delegates_to_signer_bitmap() {
    let mut attested = AttestedBlock::new(sample_block(), 20, ReceiptList::default());
    attested.signer_bitmap.set_signed(3).unwrap();
    assert_eq!(
        attested.signing_percentage(),
        attested.signer_bitmap.signing_percentage()
    );
}

/// **Acceptance:** reported percentage stays in `0..=100` for a nontrivial set.
#[test]
fn test_signing_percentage_in_range_zero_to_hundred() {
    let mut attested = AttestedBlock::new(sample_block(), 7, ReceiptList::default());
    for i in 0_u32..7 {
        attested.signer_bitmap.set_signed(i).unwrap();
        let p = attested.signing_percentage();
        assert!(p <= 100, "got {p}");
    }
}

/// **Test plan:** “Soft finality below threshold” — `50%` vs required `67%` → `false`.
#[test]
fn test_has_soft_finality_false_below_threshold() {
    let mut attested = AttestedBlock::new(sample_block(), 10, ReceiptList::default());
    for i in 0_u32..5 {
        attested.signer_bitmap.set_signed(i).unwrap();
    }
    assert_eq!(attested.signing_percentage(), 50);
    assert!(!attested.has_soft_finality(67));
}

/// **Test plan:** “Soft finality at threshold” — exactly `67%` with 100 validators (67 signers).
#[test]
fn test_has_soft_finality_true_at_exact_threshold() {
    let mut attested = AttestedBlock::new(sample_block(), 100, ReceiptList::default());
    for i in 0_u32..67 {
        attested.signer_bitmap.set_signed(i).unwrap();
    }
    assert_eq!(attested.signing_percentage(), 67);
    assert!(attested.has_soft_finality(67));
}

/// **Test plan:** “Soft finality above threshold” — `80%` with 10 validators (8 signers).
#[test]
fn test_has_soft_finality_true_above_threshold() {
    let mut attested = AttestedBlock::new(sample_block(), 10, ReceiptList::default());
    for i in 0_u32..8 {
        attested.signer_bitmap.set_signed(i).unwrap();
    }
    assert_eq!(attested.signing_percentage(), 80);
    assert!(attested.has_soft_finality(67));
}

/// **Test plan:** “Hash delegation” — [`AttestedBlock::hash`] equals [`L2Block::hash`] on the inner block.
#[test]
fn test_hash_delegates_to_inner_block() {
    let block = sample_block();
    let attested = AttestedBlock::new(block.clone(), 8, ReceiptList::default());
    assert_eq!(attested.hash(), block.hash());
    assert_eq!(attested.hash(), attested.block.hash());
}
