//! ZstdScope core library.
//!
//! The parser implementation will be added incrementally according to the
//! accepted architecture and v0.1 implementation issues.

#![forbid(unsafe_code)]

mod cursor;
mod error;
mod model;

pub use error::ZstdError;
pub use model::ByteSpan;

#[cfg(test)]
mod cursor_tests;
