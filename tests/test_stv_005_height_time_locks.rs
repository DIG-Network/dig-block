//! STV-005: Height / time lock evaluation ([SPEC §7.5.4](docs/resources/SPEC.md)).
//!
//! **Normative:** `docs/requirements/domains/state_validation/NORMATIVE.md` (STV-005)
//! **Spec:** `docs/requirements/domains/state_validation/specs/STV-005.md`
//! **Chia parity:** [`block_body_validation.py` Check 21](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_body_validation.py).
//!
//! ## Rule
//!
//! Each [`dig_block::PendingAssertion`] from Tier 2 (EXE-004 / EXE-009) is evaluated against
//! chain context taken from [`dig_block::CoinLookup::get_chain_height`] /
//! [`dig_block::CoinLookup::get_chain_timestamp`]. Relative assertions additionally require the
//! owning coin's `created_height` from [`chia_protocol::CoinState::created_height`].
//!
//! | Variant | Pass rule |
//! |---|---|
//! | `HeightAbsolute(h)` | `chain_height >= h` |
//! | `HeightRelative(h)` | `chain_height >= coin.created_height + h` |
//! | `SecondsAbsolute(t)` | `chain_timestamp >= t` |
//! | `SecondsRelative(t)` | See **Implementation caveat** below |
//! | `BeforeHeightAbsolute(h)` | `chain_height < h` |
//! | `BeforeHeightRelative(h)` | `chain_height < coin.created_height + h` |
//! | `BeforeSecondsAbsolute(t)` | `chain_timestamp < t` |
//! | `BeforeSecondsRelative(t)` | See **Implementation caveat** |
//!
//! Failed assertions return [`dig_block::BlockError::AssertionFailed { condition, reason }`].
//!
//! ## Implementation caveat — relative seconds
//!
//! `chia_protocol::CoinState` (the value returned by [`CoinLookup`]) has `created_height:
//! Option<u32>` and `spent_height: Option<u32>` but **no** per-coin creation timestamp. Per the
//! NORMATIVE Implementation Notes, "relative assertions require the coin's confirmed
//! height/timestamp." The current implementation derives creation time linearly from chain
//! height: `created_timestamp_estimate = chain_timestamp - (chain_height - created_height) *
//! AVG_BLOCK_SECONDS`. This is an approximation; an extension to [`CoinLookup`] that exposes
//! per-coin creation timestamps is a future improvement. Relative-seconds tests exercise the
//! boundary behavior against this estimate.
//!
//! ## What this proves
//!
//! One test per each of the 8 variants, pass and fail, plus a block-level absolute sanity
//! (`coin_id = ZERO_HASH`) and a relative-assertion coin-lookup case.

mod common;

use chia_protocol::{Bytes32, Coin, CoinState};
use dig_block::{
    AssertionKind, BlockError, ExecutionResult, L2Block, L2BlockHeader, PendingAssertion,
    PublicKey, Signature,
};

/// Build a CoinLookup with an explicit chain height/timestamp and optional per-coin entries.
struct Chain {
    height: u64,
    timestamp: u64,
    coins: std::collections::HashMap<Bytes32, CoinState>,
}
impl Chain {
    fn new(height: u64, timestamp: u64) -> Self {
        Self {
            height,
            timestamp,
            coins: std::collections::HashMap::new(),
        }
    }
    fn add_coin_created_at(&mut self, created_height: u32) -> Bytes32 {
        let coin = Coin::new(
            Bytes32::new([(created_height & 0xff) as u8; 32]),
            Bytes32::new([0x22; 32]),
            1,
        );
        let id = coin.coin_id();
        self.coins.insert(
            id,
            CoinState {
                coin,
                created_height: Some(created_height),
                spent_height: None,
            },
        );
        id
    }
}
impl dig_block::CoinLookup for Chain {
    fn get_coin_state(&self, coin_id: &Bytes32) -> Option<CoinState> {
        self.coins.get(coin_id).cloned()
    }
    fn get_chain_height(&self) -> u64 {
        self.height
    }
    fn get_chain_timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// Build an empty L2Block signed by the shared STV test key pair. The returned block's
/// `proposer_signature` matches `stv_test_proposer_keypair().1` so STV-006 passes inside
/// `validate_state`.
fn empty_block() -> L2Block {
    let network_id = Bytes32::new([0x55; 32]);
    let l1_hash = Bytes32::new([0x66; 32]);
    let header = L2BlockHeader::genesis(network_id, 1, l1_hash);
    let mut block = L2Block::new(header, Vec::new(), Vec::new(), Signature::default());
    common::sync_block_header_for_validate_structure(&mut block);
    let (sk, _pk) = common::stv_test_proposer_keypair();
    common::stv_sign_proposer(&mut block, &sk);
    block
}

fn run(block: &L2Block, chain: &Chain, pending: Vec<PendingAssertion>) -> Result<(), BlockError> {
    let exec = ExecutionResult {
        pending_assertions: pending,
        ..Default::default()
    };
    let (_sk, pk) = common::stv_test_proposer_keypair();
    block.validate_state(&exec, chain, &pk).map(|_| ())
}

// ---------------------------------------------------------------------------
// Absolute height
// ---------------------------------------------------------------------------

/// **STV-005 `height_absolute_pass`:** `chain_height=100`, `ASSERT_HEIGHT_ABSOLUTE(50)` → passes.
#[test]
fn height_absolute_pass() {
    let chain = Chain::new(100, 1_700_000_000);
    let pa = PendingAssertion {
        kind: AssertionKind::HeightAbsolute(50),
        coin_id: Bytes32::default(),
    };
    run(&empty_block(), &chain, vec![pa]).expect("100 >= 50");
}

/// **STV-005 `height_absolute_fail`:** `chain_height=100`, `ASSERT_HEIGHT_ABSOLUTE(200)` → reject.
#[test]
fn height_absolute_fail() {
    let chain = Chain::new(100, 1_700_000_000);
    let pa = PendingAssertion {
        kind: AssertionKind::HeightAbsolute(200),
        coin_id: Bytes32::default(),
    };
    let err = run(&empty_block(), &chain, vec![pa]).expect_err("100 < 200");
    assert!(matches!(err, BlockError::AssertionFailed { .. }));
}

// ---------------------------------------------------------------------------
// Relative height
// ---------------------------------------------------------------------------

/// **STV-005 `height_relative_pass`:** `chain_height=100`, `created_height=80`,
/// `ASSERT_HEIGHT_RELATIVE(10)` → `100 >= 90`, passes.
#[test]
fn height_relative_pass() {
    let mut chain = Chain::new(100, 1_700_000_000);
    let coin_id = chain.add_coin_created_at(80);
    let pa = PendingAssertion {
        kind: AssertionKind::HeightRelative(10),
        coin_id,
    };
    run(&empty_block(), &chain, vec![pa]).expect("100 >= 90");
}

/// **STV-005 `height_relative_fail`:** `chain_height=100`, `created_height=80`,
/// `ASSERT_HEIGHT_RELATIVE(30)` → `100 < 110`, reject.
#[test]
fn height_relative_fail() {
    let mut chain = Chain::new(100, 1_700_000_000);
    let coin_id = chain.add_coin_created_at(80);
    let pa = PendingAssertion {
        kind: AssertionKind::HeightRelative(30),
        coin_id,
    };
    let err = run(&empty_block(), &chain, vec![pa]).expect_err("100 < 110");
    assert!(matches!(err, BlockError::AssertionFailed { .. }));
}

/// **STV-005:** Relative assertion without the owning coin in CoinLookup returns
/// `BlockError::CoinNotFound` (the `created_height` is required for evaluation).
#[test]
fn height_relative_missing_coin_rejects_with_coin_not_found() {
    let chain = Chain::new(100, 1_700_000_000);
    let unknown_id = Bytes32::new([0xEE; 32]);
    let pa = PendingAssertion {
        kind: AssertionKind::HeightRelative(10),
        coin_id: unknown_id,
    };
    let err = run(&empty_block(), &chain, vec![pa]).expect_err("missing coin must reject");
    match err {
        BlockError::CoinNotFound { coin_id } => assert_eq!(coin_id, unknown_id),
        other => panic!("expected CoinNotFound, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Absolute seconds
// ---------------------------------------------------------------------------

/// **STV-005 `seconds_absolute_pass`:** `timestamp=1000`, `ASSERT_SECONDS_ABSOLUTE(500)` → pass.
#[test]
fn seconds_absolute_pass() {
    let chain = Chain::new(100, 1_000);
    let pa = PendingAssertion {
        kind: AssertionKind::SecondsAbsolute(500),
        coin_id: Bytes32::default(),
    };
    run(&empty_block(), &chain, vec![pa]).expect("1000 >= 500");
}

/// **STV-005 `seconds_absolute_fail`:** `timestamp=1000`, `ASSERT_SECONDS_ABSOLUTE(2000)` → reject.
#[test]
fn seconds_absolute_fail() {
    let chain = Chain::new(100, 1_000);
    let pa = PendingAssertion {
        kind: AssertionKind::SecondsAbsolute(2_000),
        coin_id: Bytes32::default(),
    };
    let err = run(&empty_block(), &chain, vec![pa]).expect_err("1000 < 2000");
    assert!(matches!(err, BlockError::AssertionFailed { .. }));
}

// ---------------------------------------------------------------------------
// Before-absolute
// ---------------------------------------------------------------------------

/// **STV-005 `before_height_absolute_pass`:** `chain_height=100`,
/// `ASSERT_BEFORE_HEIGHT_ABSOLUTE(200)` → `100 < 200`, passes.
#[test]
fn before_height_absolute_pass() {
    let chain = Chain::new(100, 1_700_000_000);
    let pa = PendingAssertion {
        kind: AssertionKind::BeforeHeightAbsolute(200),
        coin_id: Bytes32::default(),
    };
    run(&empty_block(), &chain, vec![pa]).expect("100 < 200");
}

/// **STV-005 `before_height_absolute_fail`:** `chain_height=100`,
/// `ASSERT_BEFORE_HEIGHT_ABSOLUTE(50)` → `100 >= 50`, rejects.
#[test]
fn before_height_absolute_fail() {
    let chain = Chain::new(100, 1_700_000_000);
    let pa = PendingAssertion {
        kind: AssertionKind::BeforeHeightAbsolute(50),
        coin_id: Bytes32::default(),
    };
    let err = run(&empty_block(), &chain, vec![pa]).expect_err("100 >= 50");
    assert!(matches!(err, BlockError::AssertionFailed { .. }));
}

/// **STV-005:** `ASSERT_BEFORE_SECONDS_ABSOLUTE` pass + fail.
#[test]
fn before_seconds_absolute_behavior() {
    let chain = Chain::new(100, 1_000);

    let pass = PendingAssertion {
        kind: AssertionKind::BeforeSecondsAbsolute(2_000),
        coin_id: Bytes32::default(),
    };
    run(&empty_block(), &chain, vec![pass]).expect("1000 < 2000");

    let fail = PendingAssertion {
        kind: AssertionKind::BeforeSecondsAbsolute(500),
        coin_id: Bytes32::default(),
    };
    let err = run(&empty_block(), &chain, vec![fail]).expect_err("1000 >= 500");
    assert!(matches!(err, BlockError::AssertionFailed { .. }));
}

// ---------------------------------------------------------------------------
// Before relative height
// ---------------------------------------------------------------------------

/// **STV-005:** `BEFORE_HEIGHT_RELATIVE`: `chain=100`, `created=80`, `h=30` → `100 < 110`, pass.
#[test]
fn before_height_relative_pass() {
    let mut chain = Chain::new(100, 1_700_000_000);
    let coin_id = chain.add_coin_created_at(80);
    let pa = PendingAssertion {
        kind: AssertionKind::BeforeHeightRelative(30),
        coin_id,
    };
    run(&empty_block(), &chain, vec![pa]).expect("100 < 110");
}

/// **STV-005:** `BEFORE_HEIGHT_RELATIVE`: `chain=100`, `created=80`, `h=5` → `100 >= 85`, reject.
#[test]
fn before_height_relative_fail() {
    let mut chain = Chain::new(100, 1_700_000_000);
    let coin_id = chain.add_coin_created_at(80);
    let pa = PendingAssertion {
        kind: AssertionKind::BeforeHeightRelative(5),
        coin_id,
    };
    let err = run(&empty_block(), &chain, vec![pa]).expect_err("100 >= 85");
    assert!(matches!(err, BlockError::AssertionFailed { .. }));
}

// ---------------------------------------------------------------------------
// Relative seconds — covered with estimate-based evaluation (see caveat above)
// ---------------------------------------------------------------------------

/// **STV-005 relative seconds behavior:** `ASSERT_SECONDS_RELATIVE(t)` passes if `chain_timestamp
/// >= coin_ts_estimate + t`. The estimate uses `coin_ts_estimate = chain_timestamp -
/// (chain_height - created_height) * AVG_BLOCK_SECONDS` with `AVG_BLOCK_SECONDS = 10`. For
/// `chain=100`, `created=90`, `AVG=10`, `coin_ts_estimate ≈ chain_ts - 100`; `t=50` passes when
/// `chain_ts >= (chain_ts - 100) + 50`, i.e. always true (equivalent to `100 >= 50`).
#[test]
fn seconds_relative_pass_and_fail() {
    let mut chain = Chain::new(100, 1_000);
    let coin_id = chain.add_coin_created_at(90);

    // Estimate: coin_ts ≈ 1000 - (100-90)*10 = 900. `t=50`: 1000 >= 950 (pass).
    let pass = PendingAssertion {
        kind: AssertionKind::SecondsRelative(50),
        coin_id,
    };
    run(&empty_block(), &chain, vec![pass]).expect("relative seconds pass");

    // `t=200`: 1000 < 1100 (fail).
    let fail = PendingAssertion {
        kind: AssertionKind::SecondsRelative(200),
        coin_id,
    };
    let err = run(&empty_block(), &chain, vec![fail]).expect_err("relative seconds fail");
    assert!(matches!(err, BlockError::AssertionFailed { .. }));
}

/// **STV-005 before-relative seconds:** Similar estimate. `chain=100`, `created=90`, coin_ts≈900.
/// `t=50` → `1000 < 950` → fail. `t=200` → `1000 < 1100` → pass.
#[test]
fn before_seconds_relative_pass_and_fail() {
    let mut chain = Chain::new(100, 1_000);
    let coin_id = chain.add_coin_created_at(90);

    let pass = PendingAssertion {
        kind: AssertionKind::BeforeSecondsRelative(200),
        coin_id,
    };
    run(&empty_block(), &chain, vec![pass]).expect("before_seconds_relative pass");

    let fail = PendingAssertion {
        kind: AssertionKind::BeforeSecondsRelative(50),
        coin_id,
    };
    let err = run(&empty_block(), &chain, vec![fail]).expect_err("before_seconds_relative fail");
    assert!(matches!(err, BlockError::AssertionFailed { .. }));
}

// ---------------------------------------------------------------------------
// Multi-assertion ordering
// ---------------------------------------------------------------------------

/// **STV-005:** Multiple assertions evaluated in order; first failure halts.
#[test]
fn multiple_assertions_first_failure_halts() {
    let chain = Chain::new(100, 1_000);
    let good = PendingAssertion {
        kind: AssertionKind::HeightAbsolute(50),
        coin_id: Bytes32::default(),
    };
    let bad = PendingAssertion {
        kind: AssertionKind::HeightAbsolute(200),
        coin_id: Bytes32::default(),
    };
    let err = run(&empty_block(), &chain, vec![good, bad]).expect_err("second assertion fails");
    assert!(matches!(err, BlockError::AssertionFailed { .. }));
}
