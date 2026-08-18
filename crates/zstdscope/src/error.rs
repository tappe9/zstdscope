use std::fmt;

/// Errors returned while structurally inspecting encoded Zstandard data.
///
/// The enum is [`non_exhaustive`](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute):
/// downstream code must include a fallback arm when matching it so future
/// validation rules can add error categories without freezing the v0.1 set.
///
/// Offsets are zero-based byte offsets from the beginning of the input passed
/// to [`crate::inspect`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ZstdError {
    /// A required encoded field or payload extends past the end of the input.
    UnexpectedEof {
        /// Offset at which the failed read or skip began.
        offset: u64,
        /// Number of bytes required by the operation.
        needed: usize,
        /// Number of bytes actually remaining from `offset`.
        remaining: usize,
    },
    /// The next top-level four-byte magic is neither Standard nor Skippable.
    InvalidMagic {
        /// Offset of the unexpected four-byte magic.
        offset: u64,
        /// Little-endian decoded magic value that was encountered.
        magic: u32,
    },
    /// The reserved bit in a Standard Frame Header Descriptor is set.
    ReservedFrameHeaderBit {
        /// Offset of the descriptor byte containing the reserved bit.
        offset: u64,
    },
    /// A block header uses the reserved block-type encoding.
    ReservedBlockType {
        /// Offset of the three-byte block header.
        offset: u64,
    },
    /// A declared block size exceeds the structural maximum for the frame.
    InvalidBlockSize {
        /// Offset of the block header declaring the invalid size.
        offset: u64,
        /// Declared 21-bit `Block_Size` value.
        size: u32,
        /// Maximum permitted size for the current frame in bytes.
        maximum: u32,
    },
    /// Checked offset, length, or conversion arithmetic overflowed.
    ArithmeticOverflow {
        /// Best available byte offset associated with the failed arithmetic.
        offset: u64,
    },
}

impl fmt::Display for ZstdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof {
                offset,
                needed,
                remaining,
            } => write!(
                f,
                "unexpected end of input at byte offset {offset}: needed {needed} bytes, {remaining} remaining"
            ),
            Self::InvalidMagic { offset, magic } => {
                write!(
                    f,
                    "invalid top-level magic 0x{magic:08X} at byte offset {offset}"
                )
            }
            Self::ReservedFrameHeaderBit { offset } => {
                write!(f, "reserved frame header bit set at byte offset {offset}")
            }
            Self::ReservedBlockType { offset } => {
                write!(f, "reserved block type at byte offset {offset}")
            }
            Self::InvalidBlockSize {
                offset,
                size,
                maximum,
            } => write!(
                f,
                "invalid block size {size} at byte offset {offset}: maximum is {maximum}"
            ),
            Self::ArithmeticOverflow { offset } => {
                write!(f, "offset arithmetic overflow at byte offset {offset}")
            }
        }
    }
}

impl std::error::Error for ZstdError {}
