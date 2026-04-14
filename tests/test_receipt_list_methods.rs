//! RCP-003: [`ReceiptList`] — `new`, `from_receipts`, `push`, `finalize`, `get`, `get_by_tx_id`, Merkle root.
//!
//! **Normative:** `docs/requirements/domains/receipt/NORMATIVE.md` (RCP-003)  
//! **Spec + test plan:** `docs/requirements/domains/receipt/specs/RCP-003.md`  
//! **Root algorithm:** [HSH-008](docs/requirements/domains/hashing/specs/HSH-008.md) (bincode leaf hash + `MerkleTree`)  
//! **Verification:** `docs/requirements/domains/receipt/VERIFICATION.md` (RCP-003)
//!
//! Tests follow the RCP-003 **Test Plan** and acceptance criteria: [`EMPTY_ROOT`] for empty lists, root
//! computation vs incremental `push` + `finalize`, and lookup helpers.

use dig_block::{Bytes32, Receipt, ReceiptList, ReceiptStatus, EMPTY_ROOT};

fn receipt_with_tx_tag(tag: u8, tx_index: u32) -> Receipt {
    Receipt::new(
        Bytes32::new([tag; 32]),
        100,
        tx_index,
        ReceiptStatus::Success,
        50,
        Bytes32::new([0xcc; 32]),
        1_000 + u64::from(tx_index),
    )
}

fn three_sample_receipts() -> Vec<Receipt> {
    vec![
        receipt_with_tx_tag(0x01, 0),
        receipt_with_tx_tag(0x02, 1),
        receipt_with_tx_tag(0x03, 2),
    ]
}

/// **Test plan:** “New list has EMPTY_ROOT”.
#[test]
fn test_new_list_root_is_empty_root() {
    let list = ReceiptList::new();
    assert_eq!(list.root, EMPTY_ROOT);
}

/// **Test plan:** “New list is empty”.
#[test]
fn test_new_list_has_no_receipts() {
    let list = ReceiptList::new();
    assert!(list.receipts.is_empty());
}

/// **Test plan:** “From receipts computes root” — non-empty list must not keep the empty sentinel as root.
#[test]
fn test_from_receipts_computes_non_empty_root() {
    let list = ReceiptList::from_receipts(three_sample_receipts());
    assert_eq!(list.receipts.len(), 3);
    assert_ne!(list.root, EMPTY_ROOT);
}

/// **Test plan:** “Push adds receipt”.
#[test]
fn test_push_increments_length() {
    let mut list = ReceiptList::new();
    list.push(receipt_with_tx_tag(0xab, 0));
    assert_eq!(list.receipts.len(), 1);
}

/// **Test plan:** “Finalize updates root” — after `push`, root stays stale until `finalize`.
#[test]
fn test_finalize_recomputes_root_after_push() {
    let mut list = ReceiptList::new();
    assert_eq!(list.root, EMPTY_ROOT);
    list.push(receipt_with_tx_tag(0x11, 0));
    assert_eq!(
        list.root, EMPTY_ROOT,
        "push alone must not update root per RCP-003"
    );
    list.finalize();
    assert_ne!(list.root, EMPTY_ROOT);
}

/// **Test plan:** “Push then finalize matches from_receipts” — same multiset/order → same Merkle root.
#[test]
fn test_push_finalize_matches_from_receipts_root() {
    let receipts = three_sample_receipts();
    let from_vec = ReceiptList::from_receipts(receipts.clone());
    let mut incremental = ReceiptList::new();
    for r in receipts {
        incremental.push(r);
    }
    incremental.finalize();
    assert_eq!(incremental.root, from_vec.root);
    assert_eq!(incremental.receipts, from_vec.receipts);
}

/// **Test plan:** “Get valid index”.
#[test]
fn test_get_returns_some_for_valid_index() {
    let list = ReceiptList::from_receipts(three_sample_receipts());
    let r = list.get(0).expect("index 0");
    assert_eq!(r.tx_index, 0);
}

/// **Test plan:** “Get invalid index”.
#[test]
fn test_get_returns_none_out_of_bounds() {
    let list = ReceiptList::from_receipts(three_sample_receipts());
    assert!(list.get(999).is_none());
}

/// **Test plan:** “Get by existing tx_id”.
#[test]
fn test_get_by_tx_id_finds_receipt() {
    let receipts = three_sample_receipts();
    let needle = receipts[1].tx_id;
    let list = ReceiptList::from_receipts(receipts);
    let found = list.get_by_tx_id(needle).expect("tx_id present");
    assert_eq!(found.tx_index, 1);
}

/// **Test plan:** “Get by missing tx_id”.
#[test]
fn test_get_by_tx_id_returns_none_when_missing() {
    let list = ReceiptList::from_receipts(three_sample_receipts());
    assert!(list.get_by_tx_id(Bytes32::default()).is_none());
}

/// **Acceptance:** [`ReceiptList::default`] matches [`ReceiptList::new`] (ATT-001 / ergonomics).
#[test]
fn test_default_matches_new() {
    let a = ReceiptList::new();
    let b = ReceiptList::default();
    assert_eq!(a.root, b.root);
    assert_eq!(a.receipts, b.receipts);
}
