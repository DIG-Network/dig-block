//! BLD-001: [`dig_block::BlockBuilder`] — the twelve accumulation fields from [SPEC §6.1](docs/resources/SPEC.md) /
//! [NORMATIVE — BLD-001](docs/requirements/domains/block_production/NORMATIVE.md#bld-001-blockbuilder-struct-and-constructor),
//! and `BlockBuilder::new` initializes mutable totals to zero / empty collections.
//!
//! **Authoritative spec:** `docs/requirements/domains/block_production/specs/BLD-001.md` (code sample + acceptance criteria +
//! test plan table). **Flat test path:** `tests/test_bld_001_builder_struct_constructor.rs` per STR-002 (not
//! `tests/block_production/…` from the spec prose).
//!
//! ## How these tests prove BLD-001
//!
//! - **`bld001_new_sets_identity_fields`:** Matches test-plan `test_new_sets_identity_fields` — every constructor argument
//!   must appear verbatim on the struct (height, epoch, parent_hash, l1_height, l1_hash, proposer_index).
//! - **`bld001_new_empty_spend_bundles`:** `spend_bundles` must be an empty `Vec` immediately after `new()`.
//! - **`bld001_new_empty_slash_payloads`:** `slash_proposal_payloads` must be empty.
//! - **`bld001_new_zero_cost` / `bld001_new_zero_fees`:** Running totals start at `0` with the correct numeric types
//!   ([`dig_block::Cost`] alias for `total_cost`).
//! - **`bld001_new_empty_additions_removals`:** `additions` and `removals` start empty — BLD-002 will populate them when
//!   spends are added; empty here proves the constructor’s zero-state contract.
//!
//! **Tooling:** Repomix packs (`.repomix/pack-src.xml`, `.repomix/pack-block-production-reqs.xml`) were regenerated before
//! coding. GitNexus `impact BlockBuilder` reported **LOW** risk / zero upstream callers (stub replacement). SocratiCode
//! MCP was not available in this workspace.

use dig_block::{BlockBuilder, Bytes32, Cost};

/// Distinct non-zero patterns so identity-field assertions cannot pass by accidental defaulting.
fn sample_identity_args() -> (u64, u64, Bytes32, u32, Bytes32, u32) {
    (
        42,
        7,
        Bytes32::new([0x01; 32]),
        99_000,
        Bytes32::new([0x02; 32]),
        3,
    )
}

/// **Test plan:** `test_new_sets_identity_fields`
#[test]
fn bld001_new_sets_identity_fields() {
    let (height, epoch, parent_hash, l1_height, l1_hash, proposer_index) = sample_identity_args();
    let b = BlockBuilder::new(
        height,
        epoch,
        parent_hash,
        l1_height,
        l1_hash,
        proposer_index,
    );
    assert_eq!(b.height, height);
    assert_eq!(b.epoch, epoch);
    assert_eq!(b.parent_hash, parent_hash);
    assert_eq!(b.l1_height, l1_height);
    assert_eq!(b.l1_hash, l1_hash);
    assert_eq!(b.proposer_index, proposer_index);
}

/// **Test plan:** `test_new_empty_spend_bundles`
#[test]
fn bld001_new_empty_spend_bundles() {
    let (height, epoch, parent_hash, l1_height, l1_hash, proposer_index) = sample_identity_args();
    let b = BlockBuilder::new(
        height,
        epoch,
        parent_hash,
        l1_height,
        l1_hash,
        proposer_index,
    );
    assert!(b.spend_bundles.is_empty());
}

/// **Test plan:** `test_new_empty_slash_payloads`
#[test]
fn bld001_new_empty_slash_payloads() {
    let (height, epoch, parent_hash, l1_height, l1_hash, proposer_index) = sample_identity_args();
    let b = BlockBuilder::new(
        height,
        epoch,
        parent_hash,
        l1_height,
        l1_hash,
        proposer_index,
    );
    assert!(b.slash_proposal_payloads.is_empty());
}

/// **Test plan:** `test_new_zero_cost`
#[test]
fn bld001_new_zero_cost() {
    let (height, epoch, parent_hash, l1_height, l1_hash, proposer_index) = sample_identity_args();
    let b = BlockBuilder::new(
        height,
        epoch,
        parent_hash,
        l1_height,
        l1_hash,
        proposer_index,
    );
    assert_eq!(b.total_cost, 0 as Cost);
}

/// **Test plan:** `test_new_zero_fees`
#[test]
fn bld001_new_zero_fees() {
    let (height, epoch, parent_hash, l1_height, l1_hash, proposer_index) = sample_identity_args();
    let b = BlockBuilder::new(
        height,
        epoch,
        parent_hash,
        l1_height,
        l1_hash,
        proposer_index,
    );
    assert_eq!(b.total_fees, 0u64);
}

/// **Test plan:** `test_new_empty_additions_removals`
#[test]
fn bld001_new_empty_additions_removals() {
    let (height, epoch, parent_hash, l1_height, l1_hash, proposer_index) = sample_identity_args();
    let b = BlockBuilder::new(
        height,
        epoch,
        parent_hash,
        l1_height,
        l1_hash,
        proposer_index,
    );
    assert!(b.additions.is_empty());
    assert!(b.removals.is_empty());
}
