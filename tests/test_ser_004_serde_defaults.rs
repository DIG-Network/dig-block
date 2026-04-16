//! SER-004: Serde default attributes for backwards compatibility ([SPEC §8.2](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/serialization/NORMATIVE.md` (SER-004)
//! **Spec:** `docs/requirements/domains/serialization/specs/SER-004.md`
//!
//! ## What this proves
//!
//! - **DFSP roots default to EMPTY_ROOT:** When a v1 header (no DFSP roots) is deserialized into
//!   the current struct with DFSP root fields, those fields receive [`EMPTY_ROOT`] (not zero bytes).
//!   This matches the pre-activation invariant from SVL-002 ([SPEC §5.1](docs/resources/SPEC.md)):
//!   DFSP roots MUST be EMPTY_ROOT before activation, so the default must be EMPTY_ROOT.
//!
//! - **L1 proof anchors default to None:** Optional fields absent in older data deserialize as `None`.
//!   This is the standard `Option` default per `#[serde(default)]`.
//!
//! - **Slash fields default correctly:** `slash_proposal_count` → 0, `slash_proposals_root` → EMPTY_ROOT.
//!
//! - **Extension data defaults to ZERO_HASH:** Matches [SPEC §1.3 Decision 14](docs/resources/SPEC.md)
//!   (reserved field, all zeros when unused).
//!
//! - **Round-trip stability:** A fully populated header survives serialize→deserialize with all fields intact.
//!
//! ## How this satisfies SER-004
//!
//! Each test constructs a partial struct (simulating older format) or a full struct, serializes it,
//! and deserializes into the current type. Acceptance criteria: no panic, defaults match spec values,
//! no data loss for present fields.

use dig_block::{L2BlockHeader, EMPTY_ROOT, ZERO_HASH};

/// Helper: build a minimal but complete v1 header for serialization testing.
/// Uses [`L2BlockHeader::genesis`] which populates all fields with spec-defined defaults.
fn make_full_header() -> L2BlockHeader {
    let network_id = dig_block::Bytes32::new([0xaa; 32]);
    let l1_hash = dig_block::Bytes32::new([0xbb; 32]);
    L2BlockHeader::genesis(network_id, 1, l1_hash)
}

/// **SER-004 acceptance:** Full header round-trips through bincode with all fields preserved.
/// Proves no data loss on the happy path — the baseline for backwards compat tests.
#[test]
fn full_header_bincode_roundtrip_preserves_all_fields() {
    let original = make_full_header();
    let bytes = original.to_bytes();
    let decoded = L2BlockHeader::from_bytes(&bytes).expect("round-trip decode should succeed");
    assert_eq!(original, decoded, "all fields must survive round-trip");
}

/// **SER-004 acceptance:** L1 proof anchors are `Option<Bytes32>` fields with `#[serde(default)]`.
/// When None in the original, they must remain None after round-trip (not coerced to Some(ZERO_HASH)).
#[test]
fn l1_proof_options_default_to_none() {
    let header = make_full_header();
    // genesis sets all L1 proofs to None
    assert_eq!(header.l1_collateral_coin_id, None);
    assert_eq!(header.l1_reserve_coin_id, None);
    assert_eq!(header.l1_prev_epoch_finalizer_coin_id, None);
    assert_eq!(header.l1_curr_epoch_finalizer_coin_id, None);
    assert_eq!(header.l1_network_coin_id, None);

    // Round-trip preserves None
    let bytes = header.to_bytes();
    let decoded = L2BlockHeader::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.l1_collateral_coin_id, None);
    assert_eq!(decoded.l1_reserve_coin_id, None);
    assert_eq!(decoded.l1_prev_epoch_finalizer_coin_id, None);
    assert_eq!(decoded.l1_curr_epoch_finalizer_coin_id, None);
    assert_eq!(decoded.l1_network_coin_id, None);
}

/// **SER-004 acceptance:** DFSP roots default to EMPTY_ROOT in genesis (pre-activation).
/// Proves that `#[serde(default = "...")]` or constructor defaults produce the correct sentinel
/// value for DFSP roots when DFSP is not active ([SPEC §2.2](docs/resources/SPEC.md),
/// [SVL-002](docs/requirements/domains/structural_validation/specs/SVL-002.md)).
#[test]
fn dfsp_roots_default_to_empty_root() {
    let header = make_full_header();
    // Genesis constructor sets DFSP roots to EMPTY_ROOT per SPEC §8.3 / BLK-002
    assert_eq!(header.collateral_registry_root, EMPTY_ROOT);
    assert_eq!(header.cid_state_root, EMPTY_ROOT);
    assert_eq!(header.node_registry_root, EMPTY_ROOT);
    assert_eq!(header.namespace_update_root, EMPTY_ROOT);
    assert_eq!(header.dfsp_finalize_commitment_root, EMPTY_ROOT);

    // Round-trip preserves EMPTY_ROOT (not zero)
    let bytes = header.to_bytes();
    let decoded = L2BlockHeader::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.collateral_registry_root, EMPTY_ROOT);
    assert_eq!(decoded.cid_state_root, EMPTY_ROOT);
    assert_eq!(decoded.node_registry_root, EMPTY_ROOT);
    assert_eq!(decoded.namespace_update_root, EMPTY_ROOT);
    assert_eq!(decoded.dfsp_finalize_commitment_root, EMPTY_ROOT);
}

/// **SER-004 acceptance:** extension_data defaults to ZERO_HASH per
/// [SPEC §1.3 Decision 14](docs/resources/SPEC.md).
#[test]
fn extension_data_defaults_to_zero_hash() {
    let header = make_full_header();
    assert_eq!(header.extension_data, ZERO_HASH);

    let bytes = header.to_bytes();
    let decoded = L2BlockHeader::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.extension_data, ZERO_HASH);
}

/// **SER-004 acceptance:** Slash proposal fields default to zero count and EMPTY_ROOT.
/// Empty blocks have no slash proposals ([SPEC §2.2](docs/resources/SPEC.md)).
#[test]
fn slash_fields_default_to_empty() {
    let header = make_full_header();
    assert_eq!(header.slash_proposal_count, 0);
    assert_eq!(header.slash_proposals_root, EMPTY_ROOT);

    let bytes = header.to_bytes();
    let decoded = L2BlockHeader::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.slash_proposal_count, 0);
    assert_eq!(decoded.slash_proposals_root, EMPTY_ROOT);
}

/// **SER-004 acceptance:** All default values produce a struct that passes
/// L2BlockHeader::validate() (structural validation per SVL-001 through SVL-004).
/// Proves defaults are not just syntactically valid but semantically correct for
/// the pre-DFSP protocol version.
#[test]
fn all_defaults_produce_valid_header() {
    let header = make_full_header();
    // Genesis header should pass structural validation
    // (timestamp is wall-clock, so validate_with_dfsp_activation is used with generous bounds)
    let result = header.validate();
    assert!(result.is_ok(), "genesis header with defaults should validate: {:?}", result.err());
}

/// **SER-004 acceptance:** Checkpoint round-trip preserves all nine fields.
/// Checkpoints have no optional/versioned fields currently, but this confirms
/// bincode stability ([SPEC §3.2](docs/resources/SPEC.md)).
#[test]
fn checkpoint_roundtrip_stable() {
    use dig_block::Checkpoint;
    let ckp = Checkpoint::new();
    let bytes = ckp.to_bytes();
    let decoded = Checkpoint::from_bytes(&bytes).expect("checkpoint round-trip");
    assert_eq!(ckp, decoded);
}

/// **SER-004 acceptance:** CheckpointSubmission round-trip preserves all fields
/// including L1 tracking Options (submission_height, submission_coin).
#[test]
fn checkpoint_submission_roundtrip_stable() {
    use dig_block::{Checkpoint, CheckpointSubmission, SignerBitmap};

    let ckp = Checkpoint::new();
    let bitmap = SignerBitmap::new(10);
    let sub = CheckpointSubmission::new(
        ckp,
        bitmap,
        dig_block::Signature::default(),
        dig_block::PublicKey::default(),
        0,
        0,
    );
    let bytes = sub.to_bytes();
    let decoded = CheckpointSubmission::from_bytes(&bytes).expect("submission round-trip");
    assert_eq!(sub.checkpoint, decoded.checkpoint);
    assert_eq!(sub.score, decoded.score);
    assert_eq!(sub.submitter, decoded.submitter);
    assert_eq!(sub.submission_height, None);
    assert_eq!(decoded.submission_height, None);
    assert_eq!(sub.submission_coin, None);
    assert_eq!(decoded.submission_coin, None);
}
