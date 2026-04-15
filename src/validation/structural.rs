//! Tier 1 **structural validation** (SVL-*): cheap, pure checks on headers and block bodies before CLVM or state.
//!
//! **Normative:** [structural_validation NORMATIVE](docs/requirements/domains/structural_validation/NORMATIVE.md) /
//! [SPEC §5](docs/resources/SPEC.md) (validation pipeline).
//!
//! ## Module layout
//!
//! Most checks are implemented as methods on the types they inspect so call sites read naturally
//! (`header.validate()`, `block.validate_structure()`, …). This file holds **module-level documentation**
//! and may later host shared helpers (for example constants-only predicates) that are not tied to a
//! single type.
//!
//! | ID | Responsibility | Primary API |
//! |----|----------------|-------------|
//! | [SVL-001](docs/requirements/domains/structural_validation/specs/SVL-001.md) | Header `version` vs `height` / DFSP activation | [`crate::L2BlockHeader::validate`], [`crate::L2BlockHeader::validate_with_dfsp_activation`] |
//! | [SVL-002](docs/requirements/domains/structural_validation/specs/SVL-002.md) | DFSP roots empty before activation height | [`crate::L2BlockHeader::validate_with_dfsp_activation`] (chained from [`crate::L2BlockHeader::validate`]) |
//! | [SVL-003](docs/requirements/domains/structural_validation/specs/SVL-003.md) | Declared `total_cost` / `block_size` vs protocol caps | [`crate::L2BlockHeader::validate_with_dfsp_activation`] (chained from [`crate::L2BlockHeader::validate`]) |
//! | [SVL-004](docs/requirements/domains/structural_validation/specs/SVL-004.md) | Header `timestamp` vs `now + MAX_FUTURE_TIMESTAMP_SECONDS` | [`crate::L2BlockHeader::validate_with_dfsp_activation_at_unix`] (tests), same pipeline in [`crate::L2BlockHeader::validate`] |
//! | [SVL-004](docs/requirements/domains/structural_validation/specs/SVL-004.md) | `timestamp` vs `now + MAX_FUTURE_TIMESTAMP_SECONDS` | [`crate::L2BlockHeader::validate_with_dfsp_activation`], [`crate::L2BlockHeader::validate_with_dfsp_activation_at_unix`] |
//! | [SVL-005](docs/requirements/domains/structural_validation/specs/SVL-005.md) | Header/body count fields agree | [`crate::L2Block::validate_structure`] |
//! | [SVL-006](docs/requirements/domains/structural_validation/specs/SVL-006.md) | Merkle roots, duplicate/double-spend, slash caps, filter, bincode size | [`crate::L2Block::validate_structure`] |
//!
//! **Rationale:** Keeping SVL-001 on [`crate::L2BlockHeader`](crate::types::header::L2BlockHeader) reuses the existing
//! BLK-007 helpers [`L2BlockHeader::protocol_version_for_height_with_activation`] and
//! [`L2BlockHeader::protocol_version_for_height`], avoiding a second copy of the fork rule.
