use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ZstdError {
    UnexpectedEof {
        offset: u64,
        needed: usize,
        remaining: usize,
    },
    InvalidMagic {
        offset: u64,
        magic: u32,
    },
    StandardFrameNotImplemented {
        offset: u64,
    },
    ArithmeticOverflow {
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
                write!(f, "invalid top-level magic 0x{magic:08X} at byte offset {offset}")
            }
            Self::StandardFrameNotImplemented { offset } => write!(
                f,
                "standard frame parsing is not implemented yet at byte offset {offset}"
            ),
            Self::ArithmeticOverflow { offset } => {
                write!(f, "offset arithmetic overflow at byte offset {offset}")
            }
        }
    }
}

impl std::error::Error for ZstdError {}
