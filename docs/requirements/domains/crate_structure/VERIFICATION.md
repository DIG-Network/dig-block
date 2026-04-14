# Crate Structure - Verification Matrix

> **Domain:** crate_structure
> **Prefix:** STR
> **Normative:** [NORMATIVE.md](./NORMATIVE.md)
> **Tracking:** [TRACKING.yaml](./TRACKING.yaml)

| ID | Status | Summary | Verification Approach |
|----|--------|---------|----------------------|
| STR-001 | verified | Cargo.toml Dependencies | Parse Cargo.toml and assert all required dependencies are present at specified minimum versions |
| STR-002 | verified | Module Hierarchy | Verify all required module files exist and mod.rs/lib.rs declarations match the specified hierarchy |
| STR-003 | verified | Public Re-exports in lib.rs | Compile-time test that all listed types are accessible from the crate root |
| STR-004 | verified | CoinLookup and BlockSigner Trait Definitions | Compile-time verification that traits define the required methods with correct signatures |
| STR-005 | verified | Test Infrastructure | Verify mock implementations compile, implement the correct traits, and helper functions produce valid test fixtures |
