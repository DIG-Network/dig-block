//! Block, checkpoint, and supporting type definitions ([SPEC §2](docs/resources/SPEC.md)).
//!
//! | Module | SPEC section | Primary type |
//! |--------|-------------|-------------|
//! | [`header`] | §2.2 | [`L2BlockHeader`](header::L2BlockHeader) — 33-field independently hashable header |
//! | [`block`] | §2.3 | [`L2Block`](block::L2Block) — header + spend bundles + slash payloads + proposer sig |
//! | [`attested`] | §2.4 | [`AttestedBlock`](attested::AttestedBlock) — block + signer bitmap + aggregate sig |
//! | [`checkpoint`] | §2.6–§2.7 | [`Checkpoint`](checkpoint::Checkpoint), [`CheckpointSubmission`](checkpoint::CheckpointSubmission) |
//! | [`receipt`] | §2.9 | [`Receipt`](receipt::Receipt), [`ReceiptList`](receipt::ReceiptList), [`ReceiptStatus`](receipt::ReceiptStatus) |
//! | [`signer_bitmap`] | §2.10 | [`SignerBitmap`](signer_bitmap::SignerBitmap) — compact bit vector (up to 65536 validators) |
//! | [`status`] | §2.5, §2.8 | [`BlockStatus`](status::BlockStatus), [`CheckpointStatus`](status::CheckpointStatus) |

pub mod attested;
pub mod block;
pub mod checkpoint;
pub mod header;
pub mod receipt;
pub mod signer_bitmap;
pub mod status;
