//! ZstdScope core library.
//!
//! The parser implementation will be added incrementally according to the
//! accepted architecture and v0.1 implementation issues.

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
