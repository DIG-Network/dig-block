# Block Production - Verification Matrix

| Field   | Value              |
|---------|--------------------|
| Domain  | block_production   |
| Prefix  | BLD                |
| Version | 1.0                |
| Date    | 2026-04-14         |

| ID      | Status      | Summary                                  | Verification Approach                                                                    |
|---------|-------------|------------------------------------------|------------------------------------------------------------------------------------------|
| BLD-001 | implemented | BlockBuilder Struct and Constructor      | `tests/test_bld_001_builder_struct_constructor.rs`: identity args preserved; empty `spend_bundles` / `slash_proposal_payloads` / `additions` / `removals`; `total_cost` and `total_fees` zero (BLD-001 test plan). |
| BLD-002 | implemented | add_spend_bundle with Budget Enforcement | `tests/test_bld_002_add_spend_bundle_budget.rs`: `CostBudgetExceeded` / `SizeBudgetExceeded`; no mutation on `Err`; additions/removals/totals/`spend_bundle_count` / `remaining_cost` (BLD-002 test plan). |
| BLD-003 | implemented | add_slash_proposal with Limits           | `tests/test_bld_003_add_slash_proposal_limits.rs`: count cap, byte cap, `Ok` at boundaries, no mutation on `Err`, count-before-size ordering (BLD-003 test plan). |
| BLD-004 | implemented | Optional Setters                         | `tests/test_bld_004_optional_setters.rs`: L1 `Option` anchors, DFSP roots, `extension_data`, overwrite + defaults (BLD-004 test plan). |
| BLD-005 | implemented | Build Pipeline                           | `tests/test_bld_005_build_pipeline.rs`: EmptyBlock; full pipeline roots/counts/V1; `build_with_dfsp_activation` V2 + MissingDfspRoots; timestamp window; `block_size` vs `compute_size`; BLS verify on `header.hash()`; `validate_structure` Ok; `SigningFailed` mapping (BLD-005 test plan). |
| BLD-006 | Not Started | BlockSigner Trait Integration            | Unit tests with mock signer returning Ok and Err; verify signature and error mapping      |
| BLD-007 | Not Started | Builder Structural Validity Guarantee    | Round-trip test: build() then validate_structure() must succeed                           |
