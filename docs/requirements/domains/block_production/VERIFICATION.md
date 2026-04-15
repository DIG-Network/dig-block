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
| BLD-003 | Not Started | add_slash_proposal with Limits           | Unit tests exceeding MAX_SLASH_PROPOSALS_PER_BLOCK and MAX_SLASH_PROPOSAL_PAYLOAD_BYTES   |
| BLD-004 | Not Started | Optional Setters                         | Unit tests verifying each setter stores the provided values correctly                     |
| BLD-005 | Not Started | Build Pipeline                           | Integration test: build a block, inspect all computed fields match expected values         |
| BLD-006 | Not Started | BlockSigner Trait Integration            | Unit tests with mock signer returning Ok and Err; verify signature and error mapping      |
| BLD-007 | Not Started | Builder Structural Validity Guarantee    | Round-trip test: build() then validate_structure() must succeed                           |
