//! Block and checkpoint construction ([SPEC §6](docs/resources/SPEC.md)).
//!
//! ## Modules
//!
//! | Module | Requirement | Purpose |
//! |--------|------------|---------|
//! | [`block_builder`] | BLD-001 — BLD-007 | Accumulates spend bundles → produces signed [`crate::L2Block`] |
//! | [`checkpoint_builder`] | CKP-006 | Accumulates block hashes → produces [`crate::Checkpoint`] |
//!
//! ## Design principle
//!
//! **"Build correct, validate everything"** ([SPEC §1.1](docs/resources/SPEC.md)): builders compute all
//! derived header fields (Merkle roots, counts, costs, filter hash, version) so the output is structurally
//! valid by construction. The validation pipeline ([`crate::validation`]) then re-derives everything from
//! scratch and rejects any mismatch — it trusts nothing from the header. This split means builders never
//! need to call validators internally, and validators never assume the block came from a builder.
//!
//! ## Chia parity
//!
//! The builder pattern parallels [`chia-blockchain/chia/consensus/block_creation.py`](https://github.com/Chia-Network/chia-blockchain/blob/main/chia/consensus/block_creation.py)
//! where `create_unfinished_block()` assembles header fields from body data. DIG's builder goes further
//! by enforcing cost/size budgets during accumulation (BLD-002/003), not just at validation time.

pub mod block_builder;
pub mod checkpoint_builder;
