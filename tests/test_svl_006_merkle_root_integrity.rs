//! SVL-006: [`L2Block::validate_structure`] — Merkle commitments, duplicate/double-spend integrity, slash limits, BIP158
//! `filter_hash`, and full-block bincode size ([SPEC §5.2 steps 3, 6–15](docs/resources/SPEC.md),
//! [NORMATIVE — SVL-006](docs/requirements/domains/structural_validation/NORMATIVE.md#svl-006-block-merkle-root-and-integrity-checks)).
//!
//! **Authoritative spec:** `docs/requirements/domains/structural_validation/specs/SVL-006.md` (acceptance criteria +
//! test plan table). **Implementation:** `src/types/block.rs` — [`dig_block::L2Block::validate_structure`]. **Errors:**
//! [`dig_block::BlockError`] variants `InvalidSpendsRoot`, `DuplicateOutput`, `DoubleSpendInBlock`, `InvalidAdditionsRoot`,
//! `InvalidRemovalsRoot`, `InvalidFilterHash`, `TooManySlashProposals`, `SlashProposalPayloadTooLarge`,
//! `InvalidSlashProposalsRoot`, `TooLarge` ([ERR-001](docs/requirements/domains/error_types/specs/ERR-001.md)).
//!
//! ## How these tests prove SVL-006
//!
//! Each case maps to the SVL-006 **Test Plan** row: we build a block whose **SVL-005 counts** already agree with the
//! body, then call [`common::sync_block_header_for_validate_structure`] so every Merkle-derived header field matches
//! the body **except** the dimension under test (or we grow the body past [`dig_block::MAX_BLOCK_SIZE`]). The expected
//! [`BlockError`] variant proves that sub-check runs in the spec’s pipeline order after counts.
//!
//! **Flat path:** `tests/test_svl_006_merkle_root_integrity.rs` (project rule: one file per requirement under `tests/`,
//! not `tests/structural_validation/…` from the spec prose).
//!
//! **Tooling:** Repomix packs regenerated under `.repomix/` before implementation. SocratiCode MCP was not configured in
//! this workspace; discovery used ripgrep / pack context. `npx gitnexus` failed on this Windows host (npm EPERM / null
//! package target) — impact was reasoned from call sites of [`L2Block::validate_structure`].

mod common;

use chia_bls::G2Element;
use chia_protocol::{Coin, CoinSpend, Program, SpendBundle};
use clvmr::serde::node_to_bytes;
use clvmr::Allocator;
use common::sync_block_header_for_validate_structure;
use dig_block::{
    BlockError, Bytes32, Cost, L2Block, L2BlockHeader, Signature, EMPTY_ROOT, MAX_BLOCK_SIZE,
    MAX_SLASH_PROPOSALS_PER_BLOCK, MAX_SLASH_PROPOSAL_PAYLOAD_BYTES,
};

/// Inert header shell; counts and Merkle fields are overwritten by [`sync_block_header_for_validate_structure`].
fn base_header_shell() -> L2BlockHeader {
    let mut h = L2BlockHeader::new(
        1,
        0,
        Bytes32::new([0x11; 32]),
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        EMPTY_ROOT,
        1,
        Bytes32::new([0x22; 32]),
        0,
        0,
        0 as Cost,
        0,
        0,
        0,
        0,
        EMPTY_ROOT,
    );
    h.slash_proposal_count = 0;
    h
}

/// Standard single-`CREATE_COIN` [`SpendBundle`] from Chia-style hex (same provenance as BLK-004 / SVL-005 tests).
fn spend_single_create_hex_coin() -> SpendBundle {
    let test_coin = Coin::new(
        hex::decode("4444444444444444444444444444444444444444444444444444444444444444")
            .unwrap()
            .try_into()
            .unwrap(),
        hex::decode("3333333333333333333333333333333333333333333333333333333333333333")
            .unwrap()
            .try_into()
            .unwrap(),
        1,
    );
    let solution = hex::decode(
        "ffff33ffa02222222222222222222222222222222222222222222222222222222222222222ff01\
         8080",
    )
    .unwrap();
    let spend = CoinSpend::new(
        test_coin,
        Program::new(vec![1u8].into()),
        Program::new(solution.into()),
    );
    SpendBundle::new(vec![spend], G2Element::default())
}

/// `CREATE_COIN` spend with a distinct output `puzzle_hash` (used to synthesize many unique bundles for the size test).
/// Same CLVM pattern as [`bundle_create_coin_with_ph`], but the **spent** coin’s `parent_coin_info` varies so each
/// bundle removes a distinct coin id (required when many bundles appear in one block — otherwise SVL-006’s double-spend
/// check fires before the oversized-body `TooLarge` case).
fn bundle_unique_spend_create_coin(removal_idx: u64, output_ph: Bytes32) -> SpendBundle {
    let mut parent = [0x44u8; 32];
    parent[24..32].copy_from_slice(&removal_idx.to_le_bytes());
    let spent = Coin::new(
        Bytes32::new(parent),
        Bytes32::new([0x33; 32]),
        1,
    );
    let mut sol = vec![0xffu8, 0xff, 0x33, 0xff, 0xa0];
    sol.extend_from_slice(output_ph.as_ref());
    sol.extend_from_slice(&[0xff, 0x01, 0x80, 0x80]);
    let spend = CoinSpend::new(
        spent,
        Program::new(vec![1u8].into()),
        Program::new(sol.into()),
    );
    SpendBundle::new(vec![spend], G2Element::default())
}

/// Serialized CLVM solution encoding **two** identical `CREATE_COIN` rows (opcode 51) for the same `(puzzle_hash, amount)`.
///
/// **Rationale:** [`SpendBundle::additions`](https://docs.rs/chia-protocol/latest/chia_protocol/spend_bundle/struct.SpendBundle.html#method.additions)
/// walks every `(51 . (ph . (amount . NIL)))` pair; duplicate rows mint the same [`Coin::coin_id`] twice, exercising
/// Chia check 13 without mutating header roots away from the body (roots still reflect the duplicated multiset).
fn solution_two_duplicate_creates(ph: Bytes32) -> Program {
    let mut a = Allocator::new();
    let mk_cond = |alloc: &mut Allocator| {
        let op = alloc.new_atom(&[51u8]).expect("CREATE_COIN opcode atom");
        let ph_atom = alloc.new_atom(ph.as_ref()).expect("puzzle_hash");
        let amt = alloc.new_small_number(1).expect("amount");
        let tail = alloc.new_pair(amt, alloc.nil()).expect("amount tail");
        let args = alloc.new_pair(ph_atom, tail).expect("CREATE_COIN args");
        alloc.new_pair(op, args).expect("one condition cell")
    };
    let c1 = mk_cond(&mut a);
    let c2 = mk_cond(&mut a);
    let tail = a.new_pair(c2, a.nil()).expect("nil tail");
    let list = a.new_pair(c1, tail).expect("two-condition solution list");
    Program::new(node_to_bytes(&a, list).expect("serialize solution").into())
}

/// One spend whose CLVM output lists the same `CREATE_COIN` twice → duplicate addition coin IDs (SVL-006 / Chia check 13).
fn spend_bundle_duplicate_outputs() -> SpendBundle {
    let test_coin = Coin::new(
        hex::decode("4444444444444444444444444444444444444444444444444444444444444444")
            .unwrap()
            .try_into()
            .unwrap(),
        hex::decode("3333333333333333333333333333333333333333333333333333333333333333")
            .unwrap()
            .try_into()
            .unwrap(),
        1,
    );
    let ph = Bytes32::new([0x22; 32]);
    let sol = solution_two_duplicate_creates(ph);
    let spend = CoinSpend::new(test_coin, Program::new(vec![1u8].into()), sol);
    SpendBundle::new(vec![spend], G2Element::default())
}

/// Well-formed single-bundle block: SVL-005 counts + SVL-006 commitments derived from the body.
fn well_formed_one_bundle_block() -> L2Block {
    let sb = spend_single_create_hex_coin();
    let mut b = L2Block::new(base_header_shell(), vec![sb], vec![], Signature::default());
    sync_block_header_for_validate_structure(&mut b);
    b
}

/// **Test plan:** `test_valid_block_passes_all_checks`
#[test]
fn svl006_valid_block_passes_all_checks() {
    let b = well_formed_one_bundle_block();
    b.validate_structure()
        .expect("synced one-bundle fixture must pass SVL-005 and SVL-006");
}

/// **Test plan:** `test_invalid_spends_root`
#[test]
fn svl006_invalid_spends_root() {
    let mut b = well_formed_one_bundle_block();
    let computed = b.compute_spends_root();
    b.header.spends_root = EMPTY_ROOT;
    match b.validate_structure() {
        Err(BlockError::InvalidSpendsRoot {
            expected,
            computed: c,
        }) => {
            assert_eq!(expected, EMPTY_ROOT);
            assert_eq!(c, computed);
        }
        o => panic!("expected InvalidSpendsRoot, got {o:?}"),
    }
}

/// **Test plan:** `test_duplicate_output`
#[test]
fn svl006_duplicate_output() {
    let sb = spend_bundle_duplicate_outputs();
    let adds = sb
        .additions()
        .expect("CLVM must yield two duplicate CREATE_COIN rows");
    assert_eq!(adds.len(), 2);
    assert_eq!(adds[0].coin_id(), adds[1].coin_id());

    let mut b = L2Block::new(base_header_shell(), vec![sb], vec![], Signature::default());
    sync_block_header_for_validate_structure(&mut b);
    let dup_id = adds[0].coin_id();
    match b.validate_structure() {
        Err(BlockError::DuplicateOutput { coin_id }) => assert_eq!(coin_id, dup_id),
        o => panic!("expected DuplicateOutput, got {o:?}"),
    }
}

/// **Test plan:** `test_double_spend`
#[test]
fn svl006_double_spend() {
    let puzzle = Program::new(vec![1u8].into());
    let solution = Program::new(vec![0x80].into());
    let coin = Coin::new(Bytes32::new([0xde; 32]), Bytes32::new([0xad; 32]), 1);
    let coin_id = coin.coin_id();
    let cs1 = CoinSpend::new(coin, puzzle.clone(), solution.clone());
    let cs2 = CoinSpend::new(
        Coin::new(Bytes32::new([0xde; 32]), Bytes32::new([0xad; 32]), 1),
        puzzle,
        solution,
    );
    let sb = SpendBundle::new(vec![cs1, cs2], G2Element::default());
    let mut b = L2Block::new(base_header_shell(), vec![sb], vec![], Signature::default());
    sync_block_header_for_validate_structure(&mut b);
    match b.validate_structure() {
        Err(BlockError::DoubleSpendInBlock { coin_id: id }) => assert_eq!(id, coin_id),
        o => panic!("expected DoubleSpendInBlock, got {o:?}"),
    }
}

/// **Test plan:** `test_invalid_additions_root`
#[test]
fn svl006_invalid_additions_root() {
    let mut b = well_formed_one_bundle_block();
    b.header.additions_root = EMPTY_ROOT;
    assert!(matches!(
        b.validate_structure(),
        Err(BlockError::InvalidAdditionsRoot)
    ));
}

/// **Test plan:** `test_invalid_removals_root`
#[test]
fn svl006_invalid_removals_root() {
    let mut b = well_formed_one_bundle_block();
    b.header.removals_root = EMPTY_ROOT;
    assert!(matches!(
        b.validate_structure(),
        Err(BlockError::InvalidRemovalsRoot)
    ));
}

/// **Test plan:** `test_invalid_filter_hash`
#[test]
fn svl006_invalid_filter_hash() {
    let mut b = well_formed_one_bundle_block();
    b.header.filter_hash = EMPTY_ROOT;
    assert!(matches!(
        b.validate_structure(),
        Err(BlockError::InvalidFilterHash)
    ));
}

/// **Test plan:** `test_too_many_slash_proposals`
#[test]
fn svl006_too_many_slash_proposals() {
    let payloads: Vec<Vec<u8>> = (0..=MAX_SLASH_PROPOSALS_PER_BLOCK)
        .map(|_| vec![0u8])
        .collect();
    let mut b = L2Block::new(
        base_header_shell(),
        vec![spend_single_create_hex_coin()],
        payloads,
        Signature::default(),
    );
    sync_block_header_for_validate_structure(&mut b);
    assert!(matches!(
        b.validate_structure(),
        Err(BlockError::TooManySlashProposals)
    ));
}

/// **Test plan:** `test_slash_proposal_payload_too_large`
#[test]
fn svl006_slash_proposal_payload_too_large() {
    let oversized = vec![0xabu8; MAX_SLASH_PROPOSAL_PAYLOAD_BYTES as usize + 1];
    let mut b = L2Block::new(
        base_header_shell(),
        vec![spend_single_create_hex_coin()],
        vec![oversized],
        Signature::default(),
    );
    sync_block_header_for_validate_structure(&mut b);
    assert!(matches!(
        b.validate_structure(),
        Err(BlockError::SlashProposalPayloadTooLarge)
    ));
}

/// **Test plan:** `test_invalid_slash_proposals_root`
#[test]
fn svl006_invalid_slash_proposals_root() {
    let mut b = L2Block::new(
        base_header_shell(),
        vec![spend_single_create_hex_coin()],
        vec![vec![1, 2, 3]],
        Signature::default(),
    );
    sync_block_header_for_validate_structure(&mut b);
    b.header.slash_proposals_root = EMPTY_ROOT;
    assert!(matches!(
        b.validate_structure(),
        Err(BlockError::InvalidSlashProposalsRoot)
    ));
}

/// **Test plan:** `test_serialized_size_exceeds_limit`
///
/// **Approach:** Estimate marginal `bincode` bytes per unique [`SpendBundle`] (one removal + one addition each), then
/// materialize enough bundles so the serialized [`L2Block`] exceeds [`MAX_BLOCK_SIZE`], keeping the header synced so
/// earlier SVL-006 gates stay green and only the final size check fails.
#[test]
fn svl006_serialized_size_exceeds_limit() {
    let h = base_header_shell();
    let mut b_empty = L2Block::new(h.clone(), vec![], vec![], Signature::default());
    sync_block_header_for_validate_structure(&mut b_empty);
    let s0 = b_empty.compute_size();

    let mut b_one = L2Block::new(
        h.clone(),
        vec![bundle_unique_spend_create_coin(0, Bytes32::new([0x77; 32]))],
        vec![],
        Signature::default(),
    );
    sync_block_header_for_validate_structure(&mut b_one);
    let inc = b_one.compute_size().saturating_sub(s0).max(1);

    let target = MAX_BLOCK_SIZE as usize + 1;
    let n = (target / inc).saturating_add(5).clamp(3, 500_000);
    let mut bundles = Vec::with_capacity(n);
    for idx in 0..n {
        let i = idx as u64;
        let mut ph = [0u8; 32];
        ph[0..8].copy_from_slice(&i.to_le_bytes());
        bundles.push(bundle_unique_spend_create_coin(i, Bytes32::new(ph)));
    }
    let mut block = L2Block::new(h, bundles, vec![], Signature::default());
    sync_block_header_for_validate_structure(&mut block);
    assert!(
        block.compute_size() > MAX_BLOCK_SIZE as usize,
        "fixture sizing failed: inc={inc} n={n} size={}",
        block.compute_size()
    );
    let sz = block.compute_size();
    match block.validate_structure() {
        Err(BlockError::TooLarge { size, max }) => {
            assert_eq!(max, MAX_BLOCK_SIZE);
            assert_eq!(size, u32::try_from(sz).unwrap_or(u32::MAX));
        }
        o => panic!("expected TooLarge, got {o:?}"),
    }
}
