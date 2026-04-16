//! BLD-006: [`dig_block::BlockSigner`] integration inside [`dig_block::BlockBuilder::build`]
//! ([SPEC §6.4 step 6](docs/resources/SPEC.md),
//! [NORMATIVE — BLD-006](docs/requirements/domains/block_production/NORMATIVE.md)).
//!
//! **Authoritative spec:** `docs/requirements/domains/block_production/specs/BLD-006.md` (trait contract, acceptance
//! criteria, test plan). **Flat test path:** `tests/test_bld_006_block_signer_integration.rs` (STR-002 — not
//! `tests/block_production/…` from the spec example).
//!
//! ## Relationship to BLK-003 / HSH-001
//!
//! The BLD-006 prose sometimes says “`header.proposer_signature`”; in this crate the BLS attestation lives on
//! [`dig_block::L2Block::proposer_signature`] ([BLK-003](docs/requirements/domains/block_types/specs/BLK-003.md)) while
//! [`dig_block::L2BlockHeader::hash`] ([HSH-001](docs/requirements/domains/hashing/specs/HSH-001.md)) commits to the
//! **710-byte** preimage that includes `block_size` but **not** the block-level signature — the signer message is the
//! canonical header digest after the two-pass size write ([`src/builder/block_builder.rs`](src/builder/block_builder.rs)).
//!
//! ## How these tests prove BLD-006
//!
//! - **`test_sign_success`:** [`dig_block::MockBlockSigner`] returns `Ok` → [`chia_bls::verify`] on
//!   `(proposer_signature, public_key, header_hash_bytes)` proves the trait supplied a valid BLS signature over the
//!   header hash ([`dig_block::BlockSigner::sign_block`]).
//! - **`test_sign_failure_maps_to_builder_error`:** A signer returning [`dig_block::traits::SignerError::SigningFailed`]
//!   forces [`dig_block::BuilderError::SigningFailed`] — the public [`dig_block::BuilderError`] surface required by
//!   [ERR-004](docs/requirements/domains/error_types/specs/ERR-004.md).
//! - **`test_signer_error_display_preserved_in_builder_error`:** [`dig_block::BuilderError::SigningFailed`] stores a
//!   `String` (not a nested `SignerError` type) so the enum stays clone-friendly; BLD-006 “preservation” is proven by
//!   retaining the [`std::fmt::Display`] output of [`dig_block::traits::SignerError`] inside that string for logging /
//!   diagnostics.
//! - **`test_header_hash_passed_to_signer_matches_final_header`:** [`RecordingSigner`] captures the `Bytes32` passed to
//!   `sign_block` and asserts it equals [`dig_block::L2BlockHeader::hash`] **after** `build` returns — proves the builder
//!   passes the same digest clients would recompute from the returned header ([`RecordingSigner`] in this file).
//! - **`test_signature_covers_header_with_final_block_size`:** The recorded hash **differs** from the hash of a clone
//!   of the built header with `block_size` forced to `0`, showing the signing step ran after the two-pass size field was
//!   written (BLD-005 ordering dependency for BLD-006).
//! - **`test_build_accepts_dyn_block_signer`:** `build(..., signer as &dyn BlockSigner)` compiles and succeeds — proves
//!   object safety promised in [STR-004](docs/requirements/domains/crate_structure/specs/STR-004.md).
//!
//! **Tooling:** `npx gitnexus impact BlockSigner` → **LOW** (implementors are tests + builder). Repomix packs refreshed
//! under `.repomix/`. **SocratiCode:** `codebase_search` / `codebase_status` MCP tools were not available in this session
//! (per `docs/prompt/start.md` step 2–5).

mod common;

use std::sync::{Arc, Mutex};

use chia_bls::verify;
use chia_protocol::Bytes32;

use dig_block::traits::{BlockSigner, SignerError};
use dig_block::{BlockBuilder, BuilderError, Signature, EMPTY_ROOT};

use common::{test_spend_bundle, MockBlockSigner};

fn mk_builder() -> BlockBuilder {
    BlockBuilder::new(
        1,
        0,
        Bytes32::new([0xca; 32]),
        1,
        Bytes32::new([0xfe; 32]),
        0,
    )
}

/// [`BlockSigner`] decorator that records the last `header_hash` argument for assertions.
///
/// **Rationale:** BLD-006 requires the builder to pass the **correct** [`dig_block::L2BlockHeader::hash`] into
/// `sign_block`; interior mutability lets the test read the captured value without changing the trait API.
struct RecordingSigner {
    last_hash: Arc<Mutex<Option<Bytes32>>>,
    inner: MockBlockSigner,
}

impl RecordingSigner {
    fn new(inner: MockBlockSigner) -> Self {
        Self {
            last_hash: Arc::new(Mutex::new(None)),
            inner,
        }
    }

    fn recorded(&self) -> Option<Bytes32> {
        *self.last_hash.lock().expect("test mutex not poisoned")
    }
}

impl BlockSigner for RecordingSigner {
    fn sign_block(&self, header_hash: &Bytes32) -> Result<Signature, SignerError> {
        *self.last_hash.lock().expect("test mutex not poisoned") = Some(*header_hash);
        self.inner.sign_block(header_hash)
    }
}

/// **Test plan:** `test_sign_success`
#[test]
fn bld006_sign_success_produces_verifiable_bls_signature() {
    let mut b = mk_builder();
    b.add_spend_bundle(test_spend_bundle(), 1, 0)
        .expect("fixture within budgets");
    let signer = MockBlockSigner::new();
    let block = b
        .build(EMPTY_ROOT, EMPTY_ROOT, &signer)
        .expect("signer returns Ok");

    let msg = block.header.hash();
    assert!(verify(
        &block.proposer_signature,
        &signer.public_key(),
        msg.as_ref()
    ));
}

/// **Test plan:** `test_sign_failure_maps_to_builder_error`
#[test]
fn bld006_sign_failure_maps_to_builder_error() {
    struct AlwaysFailSigner;

    impl BlockSigner for AlwaysFailSigner {
        fn sign_block(&self, _header_hash: &Bytes32) -> Result<Signature, SignerError> {
            Err(SignerError::SigningFailed("injected failure".into()))
        }
    }

    let mut b = mk_builder();
    b.add_spend_bundle(test_spend_bundle(), 1, 0)
        .expect("fixture");
    let err = b
        .build(EMPTY_ROOT, EMPTY_ROOT, &AlwaysFailSigner)
        .expect_err("signer must fail");
    assert!(matches!(err, BuilderError::SigningFailed(_)));
}

/// **Test plan:** preservation / diagnostics row in BLD-006 acceptance (via ERR-004 `String` carrier).
#[test]
fn bld006_signer_error_display_preserved_in_builder_error() {
    struct MarkerSigner;

    impl BlockSigner for MarkerSigner {
        fn sign_block(&self, _header_hash: &Bytes32) -> Result<Signature, SignerError> {
            Err(SignerError::SigningFailed(
                "unique-signer-failure-marker-7f3a".into(),
            ))
        }
    }

    let mut b = mk_builder();
    b.add_spend_bundle(test_spend_bundle(), 1, 0)
        .expect("fixture");
    let err = b
        .build(EMPTY_ROOT, EMPTY_ROOT, &MarkerSigner)
        .expect_err("must propagate signing failure");

    let msg = err.to_string();
    assert!(
        msg.contains("unique-signer-failure-marker-7f3a"),
        "BuilderError display should embed SignerError payload for operators: {msg}"
    );
}

/// **Test plan:** `test_header_hash_passed_to_signer`
#[test]
fn bld006_header_hash_passed_to_signer_matches_final_header() {
    let mut b = mk_builder();
    b.add_spend_bundle(test_spend_bundle(), 2, 0)
        .expect("fixture");
    let inner = MockBlockSigner::new();
    let signer = RecordingSigner::new(inner);
    let block = b
        .build(EMPTY_ROOT, EMPTY_ROOT, &signer)
        .expect("build with recording signer");

    let recorded = signer
        .recorded()
        .expect("sign_block should have been invoked");
    assert_eq!(
        recorded,
        block.header.hash(),
        "signer must see the same header digest as an independent observer"
    );
}

/// **Test plan:** `test_signature_set_after_all_fields` — `block_size` is part of the HSH-001 710-byte header preimage
/// ([`dig_block::L2BlockHeader::hash_preimage_bytes`]).
#[test]
fn bld006_signature_covers_header_with_final_block_size() {
    let mut b = mk_builder();
    b.add_spend_bundle(test_spend_bundle(), 0, 0)
        .expect("fixture");
    let signer = RecordingSigner::new(MockBlockSigner::new());
    let block = b.build(EMPTY_ROOT, EMPTY_ROOT, &signer).expect("build ok");

    let recorded = signer.recorded().expect("hash captured");
    let mut header_if_size_zero = block.header.clone();
    header_if_size_zero.block_size = 0;
    assert_ne!(
        recorded,
        header_if_size_zero.hash(),
        "recorded signing hash must not be the digest of a header that still declares block_size == 0"
    );
    assert_eq!(recorded, block.header.hash());
}

/// **Test plan:** object-safety / `&dyn BlockSigner` call path.
#[test]
fn bld006_build_accepts_dyn_block_signer() {
    let mut b = mk_builder();
    b.add_spend_bundle(test_spend_bundle(), 0, 0)
        .expect("fixture");
    let signer = MockBlockSigner::new();
    let dyn_signer: &dyn BlockSigner = &signer;
    let block = b
        .build(EMPTY_ROOT, EMPTY_ROOT, dyn_signer)
        .expect("dyn signer");

    assert!(verify(
        &block.proposer_signature,
        &signer.public_key(),
        block.header.hash().as_ref()
    ));
}
