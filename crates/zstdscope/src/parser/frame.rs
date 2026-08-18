use super::header::parse_frame_header;
use crate::{ByteSpan, Frame, FrameKind, SkippableFrame, ZstdError, ZstdFile, cursor::Cursor};

const STANDARD_MAGIC: u32 = 0xFD2F_B528;
const SKIPPABLE_MAGIC_MIN: u32 = 0x184D_2A50;
const SKIPPABLE_MAGIC_MAX: u32 = 0x184D_2A5F;

pub fn inspect(input: &[u8]) -> Result<ZstdFile, ZstdError> {
    let input_size = u64::try_from(input.len())
        .map_err(|_| ZstdError::ArithmeticOverflow { offset: u64::MAX })?;
    let mut cursor = Cursor::new(input);
    let mut frames = Vec::new();

    while frames.is_empty() || cursor.remaining() != 0 {
        let index = frames.len();
        frames.push(parse_frame(&mut cursor, index)?);
    }

    Ok(ZstdFile { input_size, frames })
}

fn parse_frame(cursor: &mut Cursor<'_>, index: usize) -> Result<Frame, ZstdError> {
    let frame_start = cursor.position();
    let magic = cursor.read_u32_le()?;

    match magic {
        STANDARD_MAGIC => parse_standard_frame(cursor, frame_start),
        SKIPPABLE_MAGIC_MIN..=SKIPPABLE_MAGIC_MAX => {
            parse_skippable_frame(cursor, index, frame_start, magic)
        }
        _ => Err(ZstdError::InvalidMagic {
            offset: public_offset(frame_start)?,
            magic,
        }),
    }
}

fn parse_standard_frame(
    cursor: &mut Cursor<'_>,
    frame_start: usize,
) -> Result<Frame, ZstdError> {
    let _header = parse_frame_header(cursor)?;

    Err(ZstdError::StandardFrameNotImplemented {
        offset: public_offset(frame_start)?,
    })
}

fn parse_skippable_frame(
    cursor: &mut Cursor<'_>,
    index: usize,
    frame_start: usize,
    magic: u32,
) -> Result<Frame, ZstdError> {
    let size_field_start = cursor.position();
    let declared_payload_size = cursor.read_u32_le()?;
    let payload_start = cursor.position();
    let payload_offset = public_offset(payload_start)?;
    let payload_len =
        usize::try_from(declared_payload_size).map_err(|_| ZstdError::ArithmeticOverflow {
            offset: payload_offset,
        })?;

    cursor.skip(payload_len)?;

    let frame_span = span_between(frame_start, cursor.position())?;
    let magic_span = fixed_span(frame_start, 4)?;
    let size_field_span = fixed_span(size_field_start, 4)?;
    let payload_span = ByteSpan {
        offset: payload_offset,
        length: u64::from(declared_payload_size),
    };

    Ok(Frame {
        index,
        span: frame_span,
        kind: FrameKind::Skippable(SkippableFrame {
            magic_span,
            magic,
            variant: (magic - SKIPPABLE_MAGIC_MIN) as u8,
            size_field_span,
            declared_payload_size,
            payload_span,
        }),
    })
}

fn fixed_span(start: usize, length: usize) -> Result<ByteSpan, ZstdError> {
    let offset = public_offset(start)?;
    let length = u64::try_from(length).map_err(|_| ZstdError::ArithmeticOverflow { offset })?;
    Ok(ByteSpan { offset, length })
}

fn span_between(start: usize, end: usize) -> Result<ByteSpan, ZstdError> {
    let offset = public_offset(start)?;
    let length = end
        .checked_sub(start)
        .ok_or(ZstdError::ArithmeticOverflow { offset })?;
    let length = u64::try_from(length).map_err(|_| ZstdError::ArithmeticOverflow { offset })?;
    Ok(ByteSpan { offset, length })
}

fn public_offset(position: usize) -> Result<u64, ZstdError> {
    u64::try_from(position).map_err(|_| ZstdError::ArithmeticOverflow { offset: u64::MAX })
}
