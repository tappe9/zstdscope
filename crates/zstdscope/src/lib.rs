//! Structural inspection for encoded Zstandard data.
//!
//! ZstdScope parses one or more concatenated Zstandard Standard or Skippable
//! Frames and returns inspection-oriented metadata without decompressing block
//! contents. Public source locations are expressed as zero-based byte offsets
//! into the original encoded input.
//!
//! # Example
//!
//! ```
//! use zstdscope::{FrameKind, ZstdError, inspect};
//!
//! // Minimal Standard Frame: magic, non-single-segment header, and one empty
//! // Raw block marked as the last block.
//! let bytes = [
//!     0x28, 0xB5, 0x2F, 0xFD, // Standard Frame magic
//!     0x00, 0x00,             // descriptor + Window Descriptor
//!     0x01, 0x00, 0x00,       // empty Raw block, Last_Block = true
//! ];
//!
//! let file = inspect(&bytes)?;
//! assert_eq!(file.input_size, 9);
//! assert_eq!(file.frames.len(), 1);
//! assert!(matches!(file.frames[0].kind, FrameKind::Standard(_)));
//! # Ok::<(), ZstdError>(())
//! ```
//!
//! The parser is strict: empty input, malformed structures, unknown top-level
//! magic values, and trailing partial frames return [`ZstdError`] rather than
//! being silently ignored.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod cursor;
mod error;
mod model;
mod parser;

pub use error::ZstdError;
pub use model::{
    Block, BlockType, ByteSpan, ContentChecksum, DictionaryId, Frame, FrameContentSize,
    FrameHeader, FrameKind, SkippableFrame, StandardFrame, ZstdFile,
};
pub use parser::inspect;

#[cfg(test)]
mod cursor_tests;
