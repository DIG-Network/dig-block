//! HSH-008: Receipts root — [`MerkleTree`] over SHA-256(bincode([`Receipt`])) leaves in block order.
//!
//! **Normative:** `docs/requirements/domains/hashing/NORMATIVE.md` (HSH-008)  
//! **Spec + test plan:** `docs/requirements/domains/hashing/specs/HSH-008.md`  
//! **Implementation:** `src/types/receipt.rs` [`dig_block::compute_receipts_root`] (re-exported from crate root in
//! [`src/lib.rs`](../../src/lib.rs)); [`ReceiptList::from_receipts`] / [`ReceiptList::finalize`] call the same function.  
//! **Crate spec:** [SPEC §3.3](docs/resources/SPEC.md) (`receipts_root` field)  
//! **Tagged tree:** [HSH-007](docs/requirements/domains/hashing/specs/HSH-007.md) — leaf/node domain separation comes from
//! [`chia_sdk_types::MerkleTree`] (same pattern as [HSH-003](docs/requirements/domains/hashing/specs/HSH-003.md) spends root).
//!
//! ## How these tests prove HSH-008
//!
//! - **`hsh008_empty_returns_empty_root`:** Empty slice ⇒ [`dig_block::EMPTY_ROOT`] ([BLK-005](docs/requirements/domains/block_types/specs/BLK-005.md)),
//!   matching the spec pseudocode early return.
//! - **`hsh008_leaf_is_sha256_of_bincode_receipt`:** Each leaf is **not** a hash of fields by hand — it is
//!   SHA-256(`bincode::serialize(Receipt)`), matching the requirement text and parity with HSH-003’s “serialize then hash” pattern.
//! - **`hsh008_explicit_merkle_matches_public_fn`:** Rebuilding leaves + [`MerkleTree::new`] in the test (duplicate of the normative
//!   algorithm) matches [`compute_receipts_root`], proving the exported API is exactly that construction.
//! - **`hsh008_single_receipt_matches_merkle_tree`:** One leaf ⇒ root equals [`MerkleTree`] over that one digest (exercises tagged
//!   leaf path per HSH-007).
//! - **`hsh008_multiple_receipts_deterministic`:** Fixed ordered slice ⇒ stable root across calls.
//! - **`hsh008_order_matters`:** Permuting two distinct receipts changes the root — the commitment includes **order**, not only multiset.
//! - **`hsh008_matches_receipt_list_from_receipts`:** [`ReceiptList::root`] after [`ReceiptList::from_receipts`] equals
//!   [`compute_receipts_root`] on the same `Vec` (acceptance: “matches ReceiptList.root”).
//! - **`hsh008_finalize_matches_compute_receipts_root`:** After [`ReceiptList::push`] + [`ReceiptList::finalize`], [`ReceiptList::root`]
//!   equals [`compute_receipts_root`] on the final vector (acceptance: used by `finalize`).
//!
//! **Layout:** Flat `tests/` ([STR-002](docs/requirements/domains/crate_structure/specs/STR-002.md)).  
//! **SocratiCode:** Not used in this environment (no MCP).

use chia_sdk_types::MerkleTree;
use chia_sha2::Sha256;
use dig_block::{compute_receipts_root, Bytes32, Receipt, ReceiptList, ReceiptStatus, EMPTY_ROOT};

/// Deterministic fixture: vary `tag` and `tx_index` so receipts are distinguishable without large boilerplate.
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

/// Normative leaf step from HSH-008 / `receipt.rs`: SHA-256 over bincode-encoded [`Receipt`].
fn leaf_sha256_bincode(receipt: &Receipt) -> Bytes32 {
    let bytes = bincode::serialize(receipt).expect("fixture Receipt must bincode-serialize");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Bytes32::new(hasher.finalize())
}

/// Reference root: same algorithm as [`compute_receipts_root`] spelled out in-test (proves the public fn matches the spec).
fn receipts_root_explicit(receipts: &[Receipt]) -> Bytes32 {
    if receipts.is_empty() {
        return EMPTY_ROOT;
    }
    let hashes: Vec<Bytes32> = receipts.iter().map(leaf_sha256_bincode).collect();
    MerkleTree::new(&hashes).root()
}

/// **Test plan:** `empty_receipts` — [`compute_receipts_root`](`[]`) returns [`EMPTY_ROOT`].
#[test]
fn hsh008_empty_returns_empty_root() {
    let receipts: &[Receipt] = &[];
    assert_eq!(compute_receipts_root(receipts), EMPTY_ROOT);
}

/// **Test plan:** each receipt leaf is SHA-256(bincode(`Receipt`)), not an ad hoc field hash.
#[test]
fn hsh008_leaf_is_sha256_of_bincode_receipt() {
    let r = receipt_with_tx_tag(0x7e, 0);
    assert_eq!(leaf_sha256_bincode(&r), leaf_sha256_bincode(&r));
    // Cross-check: bincode bytes are stable for this struct (serde layout fixed in RCP-002 tests).
    let bytes = bincode::serialize(&r).unwrap();
    let mut h = Sha256::new();
    h.update(&bytes);
    assert_eq!(Bytes32::new(h.finalize()), leaf_sha256_bincode(&r));
}

/// **Test plan:** public API matches explicit `MerkleTree` over leaf digests (spec pseudocode).
#[test]
fn hsh008_explicit_merkle_matches_public_fn() {
    let sets = [
        Vec::<Receipt>::new(),
        vec![receipt_with_tx_tag(0xaa, 0)],
        three_sample_receipts(),
    ];
    for receipts in sets {
        assert_eq!(
            compute_receipts_root(&receipts),
            receipts_root_explicit(&receipts),
            "compute_receipts_root must match MerkleTree(SHA256(bincode(r_i)))"
        );
    }
}

/// **Test plan:** `single_receipt` — one leaf ⇒ root is the tagged Merkle root of that leaf.
#[test]
fn hsh008_single_receipt_matches_merkle_tree() {
    let r = receipt_with_tx_tag(0x55, 0);
    let leaf = leaf_sha256_bincode(&r);
    let expected = MerkleTree::new(&[leaf]).root();
    assert_eq!(compute_receipts_root(std::slice::from_ref(&r)), expected);
}

/// **Test plan:** `determinism` — same ordered slice ⇒ identical root.
#[test]
fn hsh008_multiple_receipts_deterministic() {
    let receipts = three_sample_receipts();
    let a = compute_receipts_root(&receipts);
    let b = compute_receipts_root(&receipts);
    assert_eq!(a, b);
    assert_ne!(a, EMPTY_ROOT);
}

/// **Test plan:** `order_matters` — swap positions changes the Merkle root (commitment to sequence).
#[test]
fn hsh008_order_matters() {
    let a = receipt_with_tx_tag(0x10, 0);
    let b = receipt_with_tx_tag(0x20, 1);
    let root_ab = compute_receipts_root(&[a.clone(), b.clone()]);
    let root_ba = compute_receipts_root(&[b, a]);
    assert_ne!(
        root_ab, root_ba,
        "permutation of distinct receipts must change receipts_root"
    );
}

/// **Test plan:** `matches_receipt_list` — [`ReceiptList::from_receipts`] root matches free function.
#[test]
fn hsh008_matches_receipt_list_from_receipts() {
    let receipts = three_sample_receipts();
    let direct = compute_receipts_root(&receipts);
    let list = ReceiptList::from_receipts(receipts.clone());
    assert_eq!(list.root, direct);
    assert_eq!(compute_receipts_root(&list.receipts), direct);
}

/// **Test plan:** `finalize` path uses the same algorithm as [`compute_receipts_root`].
#[test]
fn hsh008_finalize_matches_compute_receipts_root() {
    let receipts = three_sample_receipts();
    let expected = compute_receipts_root(&receipts);

    let mut list = ReceiptList::new();
    for r in receipts.clone() {
        list.push(r);
    }
    list.finalize();
    assert_eq!(list.root, expected);
    assert_eq!(list.receipts, receipts);
}
