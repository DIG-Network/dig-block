# Crate Structure - Verification Matrix

> **Domain:** crate_structure
> **Prefix:** STR
> **Normative:** [NORMATIVE.md](./NORMATIVE.md)
> **Tracking:** [TRACKING.yaml](./TRACKING.yaml)

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| STR-001 | verified | Cargo.toml Dependencies | `tests/test_str_001_cargo_dependencies.rs`: parse `Cargo.toml`, version pins, serde derive, import smoke |
| STR-002 | verified | Module Hierarchy | `tests/test_str_002_module_hierarchy.rs`: required `src/` files, `mod` wiring, flat `tests/`, compile smoke |
| STR-003 | verified | Public Re-exports in lib.rs | `tests/test_str_003_public_reexports.rs`: crate-root imports + glob sample + constants |
| STR-004 | verified | CoinLookup and BlockSigner Trait Definitions | `tests/test_str_004_coin_lookup_block_signer.rs`: trait impls, object safety, `CoinState` |
| STR-005 | verified | Test Infrastructure | `tests/test_str_005_test_infrastructure.rs`: `common.rs` mocks and fixture helpers |
