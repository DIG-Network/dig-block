//! Public API smoke test — exercise the crate the way a downstream consumer would.
//!
//! **Purpose:** Integration coverage for the *crate surface* (re-exports, prelude, builder →
//! validator round-trip). No unit internals; everything here is reachable from `dig_block::*`
//! or `dig_block::prelude::*`.
//!
//! ## What this proves
//!
//! - **Prelude completeness:** `use dig_block::prelude::*` brings in the types needed to
//!   build + validate a block end-to-end. Compilation demonstrates coverage.
//! - **Crate-root re-exports:** Every item mentioned in the README's public-API table is
//!   reachable via `dig_block::Name` without navigating internal module paths (STR-003 /
//!   SPEC §10).
//! - **End-to-end round-trip:** `BlockBuilder::build(...) → L2Block::validate_full(...)`
//!   succeeds on an empty block with aligned state root and proposer signature. Proves the
//!   validation pipeline composes with the builder pipeline (BLD-007 + STV-001).
//! - **Bincode wire round-trip:** `L2Block::to_bytes()` / `L2Block::from_bytes()` preserve the
//!   block intact (SER-001 / SER-002 / SER-005).
//! - **Trait object safety:** `Box<dyn CoinLookup>` / `Box<dyn BlockSigner>` are usable handles
//!   (STR-004).

mod common;

use chia_protocol::CoinState;
use dig_clvm::ValidationConfig;

use dig_block::prelude::*;
use dig_block::traits::SignerError;

// ---------------------------------------------------------------------------
// Minimal consumer-side implementations of the two integration traits.
// ---------------------------------------------------------------------------

/// Empty coin-set lookup. Returns `None` for every id; fixed chain context.
struct EmptyCoins;
impl CoinLookup for EmptyCoins {
    fn get_coin_state(&self, _id: &Bytes32) -> Option<CoinState> {
        None
    }
    fn get_chain_height(&self) -> u64 {
        0
    }
    fn get_chain_timestamp(&self) -> u64 {
        0
    }
}

/// Deterministic BLS signer over a seeded key pair.
struct SmokeSigner {
    sk: chia_bls::SecretKey,
}
impl SmokeSigner {
    fn new() -> Self {
        let (sk, _) = common::stv_test_proposer_keypair();
        Self { sk }
    }
    fn public_key(&self) -> PublicKey {
        self.sk.public_key()
    }
}
impl BlockSigner for SmokeSigner {
    fn sign_block(&self, header_hash: &Bytes32) -> Result<Signature, SignerError> {
        Ok(chia_bls::sign(&self.sk, header_hash.as_ref()))
    }
}

// ---------------------------------------------------------------------------
// Prelude coverage
// ---------------------------------------------------------------------------

/// The prelude brings in every type named in the crate-level quickstart without qualified paths.
/// Compilation itself is the test.
#[test]
fn prelude_covers_quickstart_surface() {
    // Block types
    let _: Option<L2BlockHeader> = None;
    let _: Option<L2Block> = None;
    let _: Option<AttestedBlock> = None;
    let _: Option<Checkpoint> = None;
    let _: Option<CheckpointSubmission> = None;
    // Receipts + attestation
    let _: Option<Receipt> = None;
    let _: Option<ReceiptList> = None;
    let _ = ReceiptStatus::Success;
    let _: Option<SignerBitmap> = None;
    // Status
    let _ = BlockStatus::Pending;
    let _ = CheckpointStatus::Pending;
    // Errors
    let _: Option<BlockError> = None;
    let _: Option<BuilderError> = None;
    let _: Option<CheckpointError> = None;
    // Primitives
    let _: Cost = 0;
    let _: Bytes32 = Bytes32::default();
    let _: Signature = Signature::default();
    let _: PublicKey = PublicKey::default();
    // Execution bridge
    let _: Option<ExecutionResult> = None;
    // Builder
    let _: Option<BlockBuilder> = None;
    let _: Option<CheckpointBuilder> = None;
}

// ---------------------------------------------------------------------------
// End-to-end: build → validate_full round-trip
// ---------------------------------------------------------------------------

/// **Smoke:** `BlockBuilder::build(state_root=EMPTY_ROOT, receipts_root=EMPTY_ROOT, &signer)`
/// yields an [`L2Block`] whose `validate_full` passes under an empty CoinLookup + the matching
/// public key.
#[test]
fn builder_output_validates_end_to_end() {
    let signer = SmokeSigner::new();
    let pk = signer.public_key();

    // Builder inputs mirror the README quickstart.
    let parent = Bytes32::new([0x11; 32]);
    let l1_hash = Bytes32::new([0x22; 32]);
    let mut builder = BlockBuilder::new(
        /*height=*/ 1, /*epoch=*/ 0, parent, /*l1_height=*/ 100, l1_hash,
        /*proposer_index=*/ 0,
    );

    // BuilderError::EmptyBlock rejects bodies with zero bundles — add the minimal structurally
    // complete bundle from common fixtures. We snapshot the builder's additions/removals before
    // `build()` consumes it so the same values flow into the Tier-3 ExecutionResult below.
    let bundle = common::test_spend_bundle();
    builder
        .add_spend_bundle(bundle, /*cost=*/ 0, /*fee=*/ 0)
        .expect("add bundle");

    // Snapshot the delta for later STV-007 / STV-003 cross-checks.
    let additions_snapshot = builder.additions.clone();
    let removals_snapshot = builder.removals.clone();
    // Treat the test bundle's spent coins as ephemeral (also in additions) so STV-002 can
    // resolve them without a populated CoinLookup.
    let mut ephemeral_additions = additions_snapshot.clone();
    for coin_spend in &builder.spend_bundles[0].coin_spends {
        // Add the spent coin itself to additions so STV-002 sees it ephemeral.
        ephemeral_additions.push(coin_spend.coin);
    }

    let state_root =
        dig_block::compute_state_root_from_delta(&ephemeral_additions, &removals_snapshot);
    let receipts_root = dig_block::EMPTY_ROOT;
    let block = builder
        .build(state_root, receipts_root, &signer)
        .expect("builder must produce a valid block");

    // Tier 1 passes by construction (BLD-007).
    block
        .validate_structure()
        .expect("builder output must be structurally valid");

    // Tier 3 happy path with matching state_root. Full CLVM execution (Tier 2) is exercised
    // via `validate_execution_with_context` in the EXE-003 integration tests; here we supply
    // the snapshot directly as an ExecutionResult so the state-root recompute matches. We
    // pass `ephemeral_additions` (not the raw builder additions) so STV-007 recomputes the
    // same root that went into the header.
    let exec = ExecutionResult {
        additions: ephemeral_additions,
        removals: removals_snapshot,
        ..Default::default()
    };
    // STV-003 compares CoinLookup entry's puzzle_hash to the spend's puzzle_hash; the test
    // bundle's coin has a zero puzzle_hash and we don't populate CoinLookup, so STV-003 is
    // skipped (get_coin_state = None). STV-004 checks additions against CoinLookup (also None).
    let returned_root = block
        .validate_state(&exec, &EmptyCoins, &pk)
        .expect("tier 3 with aligned state_root");
    assert_eq!(returned_root, state_root);
    let _ = ValidationConfig::default(); // type accessibility check
}

// ---------------------------------------------------------------------------
// Wire round-trip via the public serde surface
// ---------------------------------------------------------------------------

/// **Smoke:** `L2Block::to_bytes` + `L2Block::from_bytes` preserve the block (SER-001 / SER-002).
/// Uses the same builder output as the end-to-end test so the shape under round-trip is
/// realistic, not hand-crafted.
#[test]
fn wire_round_trip_via_public_serde() {
    let signer = SmokeSigner::new();
    let mut builder = BlockBuilder::new(
        1,
        0,
        Bytes32::new([0x33; 32]),
        1,
        Bytes32::new([0x44; 32]),
        0,
    );
    builder
        .add_spend_bundle(common::test_spend_bundle(), 0, 0)
        .expect("add bundle");
    let mut additions = builder.additions.clone();
    let removals = builder.removals.clone();
    // Same ephemeral-coin trick as the end-to-end test — not strictly needed for wire
    // round-trip (validate_state is not invoked here), but keeps the produced block
    // closer to the on-wire shape of a validated block.
    for coin_spend in &builder.spend_bundles[0].coin_spends {
        additions.push(coin_spend.coin);
    }
    let state_root = dig_block::compute_state_root_from_delta(&additions, &removals);
    let block = builder
        .build(state_root, dig_block::EMPTY_ROOT, &signer)
        .expect("builder");

    let bytes = block.to_bytes();
    let decoded = L2Block::from_bytes(&bytes).expect("decode");
    // Header equality (L2Block lacks PartialEq due to chia-protocol SpendBundle); byte-stable
    // re-encoding proves fidelity.
    assert_eq!(block.header, decoded.header);
    assert_eq!(decoded.to_bytes(), bytes);
}

// ---------------------------------------------------------------------------
// Trait object safety — dyn CoinLookup, dyn BlockSigner
// ---------------------------------------------------------------------------

/// **Smoke:** Integration trait methods are callable through `&dyn` / `Box<dyn>` — proves object
/// safety without special ceremony (STR-004).
#[test]
fn traits_are_object_safe() {
    let boxed_coins: Box<dyn CoinLookup> = Box::new(EmptyCoins);
    assert_eq!(boxed_coins.get_chain_height(), 0);
    let _ = boxed_coins.get_coin_state(&Bytes32::default());

    let signer = SmokeSigner::new();
    let boxed_signer: Box<dyn BlockSigner> = Box::new(signer);
    let sig = boxed_signer
        .sign_block(&Bytes32::new([0xAB; 32]))
        .expect("sign");
    // Produces a non-default signature against a deterministic key.
    assert_ne!(sig, Signature::default());
}

// ---------------------------------------------------------------------------
// Helper coverage — every hashing helper exposed at the crate root works.
// ---------------------------------------------------------------------------

/// **Smoke:** Every compute_* helper re-exported at the crate root produces `EMPTY_ROOT` on an
/// empty input — the basic sanity check for the HSH-003..008 public surface.
#[test]
fn empty_inputs_produce_empty_roots() {
    assert_eq!(dig_block::compute_spends_root(&[]), dig_block::EMPTY_ROOT);
    assert_eq!(
        dig_block::compute_additions_root(&[]),
        dig_block::EMPTY_ROOT
    );
    assert_eq!(dig_block::compute_removals_root(&[]), dig_block::EMPTY_ROOT);
    assert_eq!(dig_block::compute_receipts_root(&[]), dig_block::EMPTY_ROOT);
    assert_eq!(
        dig_block::compute_state_root_from_delta(&[], &[]),
        dig_block::EMPTY_ROOT
    );
}

// ---------------------------------------------------------------------------
// Constants + version tags
// ---------------------------------------------------------------------------

/// **Smoke:** SPEC §2.11 constants are accessible at the crate root with the declared values.
#[test]
fn protocol_constants_exposed_at_crate_root() {
    assert_eq!(dig_block::VERSION_V1, 1);
    assert_eq!(dig_block::VERSION_V2, 2);
    assert_eq!(dig_block::MAX_BLOCK_SIZE, 10_000_000);
    assert_eq!(dig_block::MAX_COST_PER_BLOCK, 550_000_000_000);
    assert_eq!(dig_block::MAX_FUTURE_TIMESTAMP_SECONDS, 300);
    assert_eq!(dig_block::HASH_LEAF_PREFIX, 0x01);
    assert_eq!(dig_block::HASH_TREE_PREFIX, 0x02);
    assert_eq!(dig_block::MAX_VALIDATORS, 65_536);
    assert_eq!(dig_block::DFSP_ACTIVATION_HEIGHT, u64::MAX);
    assert_ne!(dig_block::EMPTY_ROOT, dig_block::ZERO_HASH);
}
