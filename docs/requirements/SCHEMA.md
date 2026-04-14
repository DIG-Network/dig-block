# Requirements Schema

This document defines the data model and conventions for all requirements in the
dig-block project.

---

## Three-Document Pattern

Each domain has exactly three files in `docs/requirements/domains/{domain}/`:

| File | Purpose |
|------|---------|
| `NORMATIVE.md` | Authoritative requirement statements with MUST/SHOULD/MAY keywords |
| `VERIFICATION.md` | QA approach and verification status per requirement |
| `TRACKING.yaml` | Machine-readable status, test references, and implementation notes |

Each requirement also has a dedicated specification file in
`docs/requirements/domains/{domain}/specs/{PREFIX-NNN}.md`.

---

## Requirement ID Format

**Pattern:** `{PREFIX}-{NNN}`

- **PREFIX**: 2-4 letter domain identifier (uppercase)
- **NNN**: Zero-padded numeric ID starting at 001

| Domain | Directory | Prefix | Description |
|--------|-----------|--------|-------------|
| Crate Structure | `crate_structure/` | `STR` | Crate layout, Cargo.toml, dependencies, traits, test infrastructure |
| Block Types | `block_types/` | `BLK` | L2BlockHeader, L2Block, constructors, helpers, constants, primitives |
| Attestation & Status | `attestation/` | `ATT` | AttestedBlock, BlockStatus, SignerBitmap |
| Checkpoint | `checkpoint/` | `CKP` | Checkpoint, CheckpointSubmission, CheckpointStatus, CheckpointBuilder |
| Receipt | `receipt/` | `RCP` | Receipt, ReceiptList, ReceiptStatus |
| Hashing | `hashing/` | `HSH` | Block hash, checkpoint hash, Merkle roots, BIP158 filter, tagged hashing |
| Error Types | `error_types/` | `ERR` | BlockError, CheckpointError, BuilderError, SignerBitmapError, ReceiptError |
| Structural Validation | `structural_validation/` | `SVL` | Header validation, block structural validation (Tier 1) |
| Block Production | `block_production/` | `BLD` | BlockBuilder, build pipeline, BlockSigner trait |
| Execution Validation | `execution_validation/` | `EXE` | Tier 2: CLVM execution, conditions, signatures, conservation |
| State Validation | `state_validation/` | `STV` | Tier 3: CoinLookup, coin existence, height/time locks, state root |
| Serialization | `serialization/` | `SER` | Bincode format, to_bytes/from_bytes, genesis block, round-trips |

**Immutability:** Requirement IDs are permanent. Deprecate requirements rather
than renumbering.

---

## Requirement Keywords

Per RFC 2119:

| Keyword | Meaning | Impact |
|---------|---------|--------|
| **MUST** | Absolute requirement | Blocks "done" status if not met |
| **MUST NOT** | Absolute prohibition | Blocks "done" status if violated |
| **SHOULD** | Expected behavior; may be deferred with rationale | Phase 2+ polish items |
| **SHOULD NOT** | Discouraged behavior | Phase 2+ polish items |
| **MAY** | Optional, nice-to-have | Stretch goals |

---

## Status Values

| Status | Description |
|--------|-------------|
| `gap` | Not implemented |
| `partial` | Implementation in progress or incomplete |
| `implemented` | Code complete, awaiting verification |
| `verified` | Implemented and verified per VERIFICATION.md |
| `deferred` | Explicitly postponed with rationale |

---

## TRACKING.yaml Item Schema

```yaml
- id: PREFIX-NNN           # Requirement ID (required)
  section: "Section Name"  # Logical grouping within domain (required)
  summary: "Brief title"   # Human-readable description (required)
  status: gap              # One of: gap, partial, implemented, verified, deferred
  spec_ref: "docs/requirements/domains/{domain}/specs/{PREFIX-NNN}.md"
  tests: []                # Array of test names or ["manual"]
  notes: ""                # Implementation notes, blockers, or evidence
```

---

## Testing Requirements

All dig-block requirements MUST be tested using:

### 1. Unit Tests (MUST)

All block type, hashing, validation, and production paths MUST be tested with:

1. **Construct** block types with test data (test headers, mock SpendBundles)
2. **Compute** hashes, roots, and derived fields
3. **Validate** structural, execution, and state validation
4. **Verify** error conditions, edge cases, and boundary values

### 2. Integration Tests (MUST for multi-domain requirements)

Tests MUST demonstrate correct interaction between domains by:
- BlockBuilder producing blocks that pass all three validation tiers
- Chia Merkle root parity (additions root, removals root)
- Full validation pipeline (structural → execution → state)
- Genesis block construction and validation
- Ephemeral coin handling across SpendBundles

### 3. Property Tests (SHOULD for hash/serialization requirements)

Hash and serialization requirements SHOULD include property-based tests:
- Hash uniqueness and stability across serialization round-trips
- Validation determinism (same input → same result)
- Builder always produces structurally valid blocks

### 4. Required Test Infrastructure

```toml
# Cargo.toml [dev-dependencies]
tempfile = "3"
rand = "0.8"
tokio = { version = "1", features = ["test-util", "macros"] }
```

```rust
use dig_block::{L2BlockHeader, L2Block, AttestedBlock, BlockBuilder, BlockError};
use dig_block::{Checkpoint, CheckpointSubmission, CheckpointBuilder};
use dig_block::{SignerBitmap, ReceiptList, Receipt, BlockStatus};
use dig_block::{EMPTY_ROOT, ZERO_HASH, MAX_BLOCK_SIZE, MAX_COST_PER_BLOCK};
use chia_protocol::{Bytes32, SpendBundle, CoinSpend, Coin, CoinState};
use chia_bls::{Signature, PublicKey};
```

---

## Master Spec Reference

All requirements trace back to the SPEC:
[SPEC.md](../resources/SPEC.md)
