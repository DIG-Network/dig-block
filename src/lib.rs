pub mod builder;
pub mod constants;
pub mod error;
pub mod hash;
pub mod traits;
pub mod types;
pub mod validation;

// -- Public re-exports (STR-003) --

// Block types
pub use types::attested::AttestedBlock;
pub use types::block::L2Block;
pub use types::checkpoint::{Checkpoint, CheckpointSubmission};
pub use types::header::L2BlockHeader;

// Status and supporting types
pub use types::receipt::{Receipt, ReceiptList, ReceiptStatus};
pub use types::signer_bitmap::SignerBitmap;
pub use types::status::{BlockStatus, CheckpointStatus};

// Error types
pub use error::{BlockError, BuilderError, CheckpointError, ReceiptError, SignerBitmapError};

// Constants
pub use constants::*;

// Traits
pub use traits::{BlockSigner, CoinLookup};

// Builder types
pub use builder::block_builder::BlockBuilder;
pub use builder::checkpoint_builder::CheckpointBuilder;

// Validation result
pub use validation::execution::ExecutionResult;
