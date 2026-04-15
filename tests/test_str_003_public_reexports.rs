//! STR-003: Public re-exports from crate root ([spec](docs/requirements/domains/crate_structure/specs/STR-003.md), [SPEC §10](docs/resources/SPEC.md)).
//!
//! ## What this proves
//!
//! - **API ergonomics:** Callers may `use dig_block::L2Block` (etc.) without reaching into `dig_block::types::…` — the facade matches [crate_structure NORMATIVE](docs/requirements/domains/crate_structure/NORMATIVE.md).
//! - **Glob import:** `use dig_block::*` remains usable for scripts and tests (sample of types + constants).
//! - **Constants + primitives:** Protocol limits and Chia re-exports (`Bytes32`, `Cost`, …) match documented values (cross-check with BLK-005 / BLK-006 tests for full coverage).

#[test]
fn block_type_imports() {
    // All block types importable from crate root.
    use dig_block::AttestedBlock;
    use dig_block::Checkpoint;
    use dig_block::CheckpointSubmission;
    use dig_block::L2Block;
    use dig_block::L2BlockHeader;

    // Type names are reachable (compile-time check).
    let _: Option<L2BlockHeader> = None;
    let _: Option<L2Block> = None;
    let _: Option<AttestedBlock> = None;
    let _: Option<Checkpoint> = None;
    let _: Option<CheckpointSubmission> = None;
}

#[test]
fn status_type_imports() {
    use dig_block::BlockStatus;
    use dig_block::CheckpointStatus;
    use dig_block::Receipt;
    use dig_block::ReceiptList;
    use dig_block::ReceiptStatus;
    use dig_block::SignerBitmap;

    let _ = BlockStatus::Pending;
    let _ = CheckpointStatus::Pending;
    let _ = ReceiptStatus::Success;
    let _: Option<Receipt> = None;
    let _: Option<ReceiptList> = None;
    let _: Option<SignerBitmap> = None;
}

#[test]
fn error_type_imports() {
    use dig_block::BlockError;
    use dig_block::BuilderError;
    use dig_block::CheckpointError;
    use dig_block::ReceiptError;
    use dig_block::SignerBitmapError;

    let _: Option<BlockError> = None;
    let _: Option<CheckpointError> = None;
    let _: Option<BuilderError> = None;
    let _: Option<SignerBitmapError> = None;
    let _: Option<ReceiptError> = None;
}

#[test]
fn trait_imports() {
    use dig_block::BlockSigner;
    use dig_block::CoinLookup;

    // Traits are importable (compile-time check).
    fn _requires_coin_lookup(_c: &dyn CoinLookup) {}
    fn _requires_block_signer(_s: &dyn BlockSigner) {}
}

#[test]
fn builder_imports() {
    use dig_block::BlockBuilder;
    use dig_block::CheckpointBuilder;

    let _: Option<BlockBuilder> = None;
    let _: Option<CheckpointBuilder> = None;
}

#[test]
fn execution_result_import() {
    use dig_block::ExecutionResult;

    let _: Option<ExecutionResult> = None;
}

#[test]
fn glob_import_all_public_types() {
    use dig_block::*;

    // Verify a sampling of types from each category via glob import.
    let _: Option<L2BlockHeader> = None;
    let _: Option<L2Block> = None;
    let _ = BlockStatus::Pending;
    let _ = ReceiptStatus::Success;
    let _ = EMPTY_ROOT;
    let _ = MAX_BLOCK_SIZE;
    let _: Option<BlockBuilder> = None;
    let _: Option<ExecutionResult> = None;
    // HSH-007: tagged Merkle helpers re-exported from crate root.
    let _ = hash_leaf(EMPTY_ROOT.as_ref());
    // HSH-008: receipts root helper re-exported from crate root (same algorithm as ReceiptList).
    let empty: &[Receipt] = &[];
    assert_eq!(compute_receipts_root(empty), EMPTY_ROOT);
}

#[test]
fn constants_accessible_from_root() {
    use dig_block::Bytes32;
    use dig_block::Cost;
    use dig_block::PublicKey;
    use dig_block::Signature;
    use dig_block::DFSP_ACTIVATION_HEIGHT;
    use dig_block::EMPTY_ROOT;
    use dig_block::HASH_LEAF_PREFIX;
    use dig_block::HASH_TREE_PREFIX;
    use dig_block::MAX_BLOCK_SIZE;
    use dig_block::MAX_COST_PER_BLOCK;
    use dig_block::MAX_FUTURE_TIMESTAMP_SECONDS;
    use dig_block::MAX_SLASH_PROPOSALS_PER_BLOCK;
    use dig_block::MAX_SLASH_PROPOSAL_PAYLOAD_BYTES;
    use dig_block::VERSION_V1;
    use dig_block::VERSION_V2;
    use dig_block::ZERO_HASH;

    let _: Cost = 0_u64;
    assert_eq!(VERSION_V1, 1);
    assert_eq!(VERSION_V2, 2);
    let _ = Bytes32::default();
    let _ = Signature::default();
    let _ = PublicKey::default();

    assert_eq!(MAX_BLOCK_SIZE, 10_000_000);
    assert_eq!(MAX_COST_PER_BLOCK, 550_000_000_000);
    assert_eq!(MAX_SLASH_PROPOSALS_PER_BLOCK, 64);
    assert_eq!(MAX_SLASH_PROPOSAL_PAYLOAD_BYTES, 65_536);
    assert_eq!(DFSP_ACTIVATION_HEIGHT, u64::MAX);
    assert_eq!(MAX_FUTURE_TIMESTAMP_SECONDS, 300);
    assert_eq!(HASH_LEAF_PREFIX, 0x01);
    assert_eq!(HASH_TREE_PREFIX, 0x02);
    assert_ne!(EMPTY_ROOT, ZERO_HASH);
}
