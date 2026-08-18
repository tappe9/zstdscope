use std::fmt;

/// Resource category that exceeded an [`crate::InspectionLimits`] value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLimitKind {
    /// Maximum number of top-level frames.
    Frames,
    /// Maximum number of blocks within one Standard Frame.
    BlocksPerFrame,
    /// Maximum total number of blocks across all Standard Frames.
    TotalBlocks,
}

/// Errors returned while structurally inspecting encoded Zstandard data.
///
/// The enum is [`non_exhaustive`](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute):
/// downstream code must include a fallback arm when matching it so future
/// validation rules can add error categories without freezing the v0.1 set.
///
/// Offsets are zero-based byte offsets from the beginning of the input passed
/// to [`crate::inspect`] or [`crate::inspect_with_limits`].
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
        /// Size value that violates the frame block-size constraint.
        size: u32,
        /// Maximum permitted size for the current frame in bytes.
        maximum: u32,
    },
    /// A Compressed Block is too small to contain its mandatory section headers.
    InvalidCompressedBlockSize {
        /// Offset of the three-byte block header.
        offset: u64,
        /// Declared compressed Block Content size.
        size: u32,
        /// Minimum structurally possible Compressed Block Content size.
        minimum: u32,
    },
    /// The declared Frame Content Size cannot match the parsed block structure.
    FrameContentSizeMismatch {
        /// Offset of the encoded Frame Content Size field.
        offset: u64,
        /// Declared decompressed frame size.
        declared: u64,
        /// Minimum decoded size possible from the parsed block metadata.
        minimum: u128,
        /// Maximum decoded size possible from the parsed block metadata.
        maximum: u128,
    },
    /// A caller-provided inspection resource limit was exhausted.
    ResourceLimitExceeded {
        /// Offset of the frame magic or block header that would exceed the limit.
        offset: u64,
        /// Resource category whose limit was exhausted.
        resource: ResourceLimitKind,
        /// Configured maximum count for the resource.
        limit: usize,
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
            Self::InvalidCompressedBlockSize {
                offset,
                size,
                minimum,
            } => write!(
                f,
                "invalid compressed block size {size} at byte offset {offset}: minimum is {minimum}"
            ),
            Self::FrameContentSizeMismatch {
                offset,
                declared,
                minimum,
                maximum,
            } => write!(
                f,
                "frame content size {declared} at byte offset {offset} is outside decoded-size bounds {minimum}..={maximum}"
            ),
            Self::ResourceLimitExceeded {
                offset,
                resource,
                limit,
            } => write!(
                f,
                "inspection resource limit {resource:?} exceeded at byte offset {offset}: limit is {limit}"
            ),
            Self::ArithmeticOverflow { offset } => {
                write!(f, "offset arithmetic overflow at byte offset {offset}")
            }
        }
    }
}

impl std::error::Error for ZstdError {}
