# dig-block Requirements

This directory contains the formal requirements for the dig-block crate,
following the same two-tier requirements structure as dig-gossip
with full traceability.

## Quick Links

- [SCHEMA.md](SCHEMA.md) — Data model and conventions
- [REQUIREMENTS_REGISTRY.yaml](REQUIREMENTS_REGISTRY.yaml) — Central domain registry
- [domains/](domains/) — All requirement domains

## Structure

```
requirements/
├── README.md                    # This file
├── SCHEMA.md                    # Data model and conventions
├── REQUIREMENTS_REGISTRY.yaml   # Central registry
├── IMPLEMENTATION_ORDER.md      # Phased implementation checklist
└── domains/
    ├── crate_structure/         # STR-* Crate layout, dependencies, traits, test infra
    ├── block_types/             # BLK-* L2BlockHeader, L2Block, constructors, helpers, constants
    ├── attestation/             # ATT-* AttestedBlock, BlockStatus, SignerBitmap
    ├── checkpoint/              # CKP-* Checkpoint, CheckpointSubmission, CheckpointBuilder
    ├── receipt/                 # RCP-* Receipt, ReceiptList, ReceiptStatus
    ├── hashing/                 # HSH-* Block hash, checkpoint hash, Merkle roots, BIP158 filter
    ├── error_types/             # ERR-* BlockError, CheckpointError, BuilderError, SignerBitmapError
    ├── structural_validation/   # SVL-* Header validation, block structural validation (Tier 1)
    ├── block_production/        # BLD-* BlockBuilder, CheckpointBuilder, build pipeline
    ├── execution_validation/    # EXE-* CLVM execution, conditions, signatures (Tier 2)
    ├── state_validation/        # STV-* Coin lookups, height/time locks, state root (Tier 3)
    └── serialization/           # SER-* Bincode format, round-trips, genesis block
```

## Three-Document Pattern

Each domain contains:

| File | Purpose |
|------|---------|
| `NORMATIVE.md` | Authoritative requirement statements (MUST/SHOULD/MAY) |
| `VERIFICATION.md` | QA approach and status per requirement |
| `TRACKING.yaml` | Machine-readable status, tests, and notes |

## Specification Files

Individual requirement specifications are in each domain's `specs/` subdirectory:

```
domains/
├── crate_structure/specs/            # STR-001.md through STR-005.md
├── block_types/specs/                # BLK-001.md through BLK-007.md
├── attestation/specs/                # ATT-001.md through ATT-005.md
├── checkpoint/specs/                 # CKP-001.md through CKP-006.md
├── receipt/specs/                    # RCP-001.md through RCP-004.md
├── hashing/specs/                    # HSH-001.md through HSH-007.md
├── error_types/specs/                # ERR-001.md through ERR-005.md
├── structural_validation/specs/      # SVL-001.md through SVL-006.md
├── block_production/specs/           # BLD-001.md through BLD-007.md
├── execution_validation/specs/       # EXE-001.md through EXE-008.md
├── state_validation/specs/           # STV-001.md through STV-007.md
└── serialization/specs/              # SER-001.md through SER-005.md
```

## Reference Document

All requirements are derived from:
- [SPEC.md](../resources/SPEC.md) — dig-block specification

## Requirement Count

| Domain | Prefix | Count |
|--------|--------|-------|
| Crate Structure | STR | 5 |
| Block Types | BLK | 7 |
| Attestation & Status | ATT | 5 |
| Checkpoint | CKP | 6 |
| Receipt | RCP | 4 |
| Hashing | HSH | 7 |
| Error Types | ERR | 5 |
| Structural Validation | SVL | 6 |
| Block Production | BLD | 7 |
| Execution Validation | EXE | 8 |
| State Validation | STV | 7 |
| Serialization | SER | 5 |
| **Total** | | **72** |
