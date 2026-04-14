//! RCP-004: [`ReceiptList`] aggregate queries — `len`, `success_count`, `failure_count`, `total_fees`.
//!
//! **Normative:** `docs/requirements/domains/receipt/NORMATIVE.md` (RCP-004)  
//! **Spec + test plan:** `docs/requirements/domains/receipt/specs/RCP-004.md`  
//! **Verification:** `docs/requirements/domains/receipt/VERIFICATION.md` (RCP-004)
//!
//! Covers the RCP-004 **Test Plan** and the invariant `success_count + failure_count == len`.

use dig_block::{Bytes32, Receipt, ReceiptList, ReceiptStatus};

fn receipt(status: ReceiptStatus, fee_charged: u64, tag: u8, idx: u32) -> Receipt {
    Receipt::new(
        Bytes32::new([tag; 32]),
        1,
        idx,
        status,
        fee_charged,
        Bytes32::default(),
        0,
    )
}

/// **Test plan:** “Empty list len” — [`ReceiptList::len`] is `0`.
#[test]
fn test_len_empty_list() {
    let list = ReceiptList::new();
    assert_eq!(list.len(), 0);
}

/// **Test plan:** “Non-empty list len” — five receipts → `len() == 5`.
#[test]
fn test_len_five_receipts() {
    let receipts: Vec<_> = (0_u8..5)
        .map(|i| receipt(ReceiptStatus::Success, 0, i, u32::from(i)))
        .collect();
    let list = ReceiptList::from_receipts(receipts);
    assert_eq!(list.len(), 5);
}

/// **Test plan:** “All success” — three `Success` → `success_count == 3`, `failure_count == 0`.
#[test]
fn test_all_success_counts() {
    let receipts = vec![
        receipt(ReceiptStatus::Success, 0, 1, 0),
        receipt(ReceiptStatus::Success, 0, 2, 1),
        receipt(ReceiptStatus::Success, 0, 3, 2),
    ];
    let list = ReceiptList::from_receipts(receipts);
    assert_eq!(list.success_count(), 3);
    assert_eq!(list.failure_count(), 0);
}

/// **Test plan:** “All failure” — three non-success (here `Failed`) → `success == 0`, `failure == 3`.
#[test]
fn test_all_failure_counts() {
    let receipts = vec![
        receipt(ReceiptStatus::Failed, 0, 1, 0),
        receipt(ReceiptStatus::Failed, 0, 2, 1),
        receipt(ReceiptStatus::Failed, 0, 3, 2),
    ];
    let list = ReceiptList::from_receipts(receipts);
    assert_eq!(list.success_count(), 0);
    assert_eq!(list.failure_count(), 3);
}

/// **Test plan:** “Mixed statuses” — 2 Success, 1 InsufficientBalance, 1 InvalidNonce → success 2, failure 2.
#[test]
fn test_mixed_status_counts() {
    let receipts = vec![
        receipt(ReceiptStatus::Success, 0, 1, 0),
        receipt(ReceiptStatus::Success, 0, 2, 1),
        receipt(ReceiptStatus::InsufficientBalance, 0, 3, 2),
        receipt(ReceiptStatus::InvalidNonce, 0, 4, 3),
    ];
    let list = ReceiptList::from_receipts(receipts);
    assert_eq!(list.success_count(), 2);
    assert_eq!(list.failure_count(), 2);
}

/// **Test plan:** “Success + failure equals len” — holds for an arbitrary mix of statuses.
#[test]
fn test_success_plus_failure_equals_len_invariant() {
    let statuses = [
        ReceiptStatus::Success,
        ReceiptStatus::Failed,
        ReceiptStatus::InvalidSignature,
        ReceiptStatus::Success,
        ReceiptStatus::AccountNotFound,
    ];
    let receipts: Vec<_> = statuses
        .iter()
        .enumerate()
        .map(|(i, &s)| receipt(s, 0, i as u8, i as u32))
        .collect();
    let list = ReceiptList::from_receipts(receipts);
    assert_eq!(
        list.success_count() + list.failure_count(),
        list.len(),
        "RCP-004 acceptance: counts partition the list"
    );
}

/// **Test plan:** “Total fees empty list”.
#[test]
fn test_total_fees_empty() {
    let list = ReceiptList::new();
    assert_eq!(list.total_fees(), 0);
}

/// **Test plan:** “Total fees summation” — 100 + 200 + 300 = 600.
#[test]
fn test_total_fees_summation() {
    let receipts = vec![
        receipt(ReceiptStatus::Success, 100, 1, 0),
        receipt(ReceiptStatus::Success, 200, 2, 1),
        receipt(ReceiptStatus::Success, 300, 3, 2),
    ];
    let list = ReceiptList::from_receipts(receipts);
    assert_eq!(list.total_fees(), 600);
}

/// **Test plan:** “Total fees includes failed tx” — Success100 + Failed 50 → 150.
#[test]
fn test_total_fees_includes_failed_transactions() {
    let receipts = vec![
        receipt(ReceiptStatus::Success, 100, 1, 0),
        receipt(ReceiptStatus::Failed, 50, 2, 1),
    ];
    let list = ReceiptList::from_receipts(receipts);
    assert_eq!(list.total_fees(), 150);
}
