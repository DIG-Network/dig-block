//! BLD-004: [`dig_block::BlockBuilder`] optional header fragments — `set_l1_proofs`, `set_dfsp_roots`,
//! `set_extension_data` ([SPEC §6.3](docs/resources/SPEC.md),
//! [NORMATIVE — BLD-004](docs/requirements/domains/block_production/NORMATIVE.md#bld-004-optional-setters)).
//!
//! **Authoritative spec:** `docs/requirements/domains/block_production/specs/BLD-004.md` (acceptance + test plan).
//! **Flat test path:** `tests/test_bld_004_optional_setters.rs` (STR-002; not `tests/block_production/…` from spec prose).
//!
//! ## How these tests prove BLD-004
//!
//! - **L1 proof anchors:** [`L2BlockHeader`](dig_block::L2BlockHeader) stores these as `Option<Bytes32>`; the builder
//!   mirrors that shape so BLD-005 can copy `Some(hash)` into the header. Assertions on all five slots prove
//!   `set_l1_proofs` stores the tuple in one shot ([BLD-004 test plan](docs/requirements/domains/block_production/specs/BLD-004.md#test-plan)).
//! - **DFSP roots:** Five mandatory `Bytes32` roots default to [`dig_block::EMPTY_ROOT`] before activation; `set_dfsp_roots`
//!   overwrites them together (SVL-002 / BLK-001 header field parity).
//! - **`extension_data`:** Defaults to [`dig_block::ZERO_HASH`] per header constructors; `set_extension_data` replaces it.
//! - **Overwrite:** Second calls replace first — proves infallible setters are idempotent assignments, not accumulators.
//! - **Defaults:** Fresh [`BlockBuilder::new`] without setters must expose `None` L1 options, `EMPTY_ROOT` DFSP roots, and
//!   `ZERO_HASH` extension so pre-DFSP / proofless blocks need no extra setup ([BLD-004 acceptance](docs/requirements/domains/block_production/specs/BLD-004.md#acceptance-criteria)).
//!
//! **Tooling:** Repomix (`.repomix/pack-src.xml`, `pack-block-production-reqs.xml`) regenerated before edits.
//! `npx gitnexus impact BlockBuilder` → **LOW** / zero upstream callers. SocratiCode MCP not configured here.
//! `docs/prompt/tree/dt-wf-select.md` is absent from this repository.

use dig_block::{BlockBuilder, Bytes32, EMPTY_ROOT, ZERO_HASH};

fn mk_builder() -> BlockBuilder {
    BlockBuilder::new(
        2,
        1,
        Bytes32::new([0x11; 32]),
        10,
        Bytes32::new([0x22; 32]),
        3,
    )
}

fn distinct_hashes() -> [Bytes32; 5] {
    [
        Bytes32::new([0x01; 32]),
        Bytes32::new([0x02; 32]),
        Bytes32::new([0x03; 32]),
        Bytes32::new([0x04; 32]),
        Bytes32::new([0x05; 32]),
    ]
}

/// **Test plan:** `test_set_l1_proofs_stores_values`
#[test]
fn bld004_set_l1_proofs_stores_values() {
    let mut b = mk_builder();
    let [c, r, p, u, n] = distinct_hashes();
    b.set_l1_proofs(c, r, p, u, n);
    assert_eq!(b.l1_collateral_coin_id, Some(c));
    assert_eq!(b.l1_reserve_coin_id, Some(r));
    assert_eq!(b.l1_prev_epoch_finalizer_coin_id, Some(p));
    assert_eq!(b.l1_curr_epoch_finalizer_coin_id, Some(u));
    assert_eq!(b.l1_network_coin_id, Some(n));
}

/// **Test plan:** `test_set_dfsp_roots_stores_values`
#[test]
fn bld004_set_dfsp_roots_stores_values() {
    let mut b = mk_builder();
    let [a, c, d, e, f] = distinct_hashes();
    b.set_dfsp_roots(a, c, d, e, f);
    assert_eq!(b.collateral_registry_root, a);
    assert_eq!(b.cid_state_root, c);
    assert_eq!(b.node_registry_root, d);
    assert_eq!(b.namespace_update_root, e);
    assert_eq!(b.dfsp_finalize_commitment_root, f);
}

/// **Test plan:** `test_set_extension_data_stores_value`
#[test]
fn bld004_set_extension_data_stores_value() {
    let mut b = mk_builder();
    let x = Bytes32::new([0x99; 32]);
    b.set_extension_data(x);
    assert_eq!(b.extension_data, x);
}

/// **Test plan:** `test_setter_overwrites_previous`
#[test]
fn bld004_setter_overwrites_previous() {
    let mut b = mk_builder();
    let first = distinct_hashes();
    b.set_l1_proofs(first[0], first[1], first[2], first[3], first[4]);
    let second = [
        Bytes32::new([0xa0; 32]),
        Bytes32::new([0xa1; 32]),
        Bytes32::new([0xa2; 32]),
        Bytes32::new([0xa3; 32]),
        Bytes32::new([0xa4; 32]),
    ];
    b.set_l1_proofs(second[0], second[1], second[2], second[3], second[4]);
    assert_eq!(b.l1_collateral_coin_id, Some(second[0]));

    b.set_dfsp_roots(first[0], first[1], first[2], first[3], first[4]);
    b.set_dfsp_roots(second[0], second[1], second[2], second[3], second[4]);
    assert_eq!(b.collateral_registry_root, second[0]);

    b.set_extension_data(first[0]);
    b.set_extension_data(second[0]);
    assert_eq!(b.extension_data, second[0]);
}

/// **Test plan:** `test_defaults_when_setters_not_called`
#[test]
fn bld004_defaults_when_setters_not_called() {
    let b = mk_builder();
    assert!(b.l1_collateral_coin_id.is_none());
    assert!(b.l1_reserve_coin_id.is_none());
    assert!(b.l1_prev_epoch_finalizer_coin_id.is_none());
    assert!(b.l1_curr_epoch_finalizer_coin_id.is_none());
    assert!(b.l1_network_coin_id.is_none());
    assert_eq!(b.collateral_registry_root, EMPTY_ROOT);
    assert_eq!(b.cid_state_root, EMPTY_ROOT);
    assert_eq!(b.node_registry_root, EMPTY_ROOT);
    assert_eq!(b.namespace_update_root, EMPTY_ROOT);
    assert_eq!(b.dfsp_finalize_commitment_root, EMPTY_ROOT);
    assert_eq!(b.extension_data, ZERO_HASH);
}
