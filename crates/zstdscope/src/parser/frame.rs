use super::{
    block::{decoded_size_bounds, parse_blocks_with_limits},
    header::parse_frame_header,
};
use crate::{
    ByteSpan, ContentChecksum, Frame, FrameKind, InspectionLimits, ResourceLimitKind,
    SkippableFrame, StandardFrame, ZstdError, ZstdFile, cursor::Cursor,
};

const STANDARD_MAGIC: u32 = 0xFD2F_B528;
const SKIPPABLE_MAGIC_MIN: u32 = 0x184D_2A50;
const SKIPPABLE_MAGIC_MAX: u32 = 0x184D_2A5F;

/// Inspects one or more concatenated Zstandard frames without decompressing them.
///
/// This convenience entry point does not impose frame or block count limits.
/// Call [`inspect_with_limits`] when inspecting untrusted input under an
/// application-specific metadata budget.
///
/// The input must contain at least one complete Standard or Skippable Frame and
/// must end exactly at a frame boundary. Returned spans use zero-based encoded
/// byte offsets from the beginning of `input`.
///
/// Standard Frame block contents are treated as opaque bytes. Optional content
/// checksums are consumed and exposed as stored metadata but are not verified.
///
/// # Errors
///
/// Returns [`ZstdError`] for malformed or truncated input, unsupported top-level
/// magic values, reserved encodings, invalid structural block sizes, or checked
/// arithmetic failures.
pub fn inspect(input: &[u8]) -> Result<ZstdFile, ZstdError> {
    inspect_with_limits(input, InspectionLimits::UNLIMITED)
}

/// Inspects Zstandard frames while enforcing caller-provided metadata limits.
///
/// Limits are checked immediately before parsing the next affected frame or
/// block. A count equal to the configured maximum is accepted; an additional
/// frame or block returns [`ZstdError::ResourceLimitExceeded`] at the source
/// offset where that structure would begin. Skippable Frames count toward
/// `max_frames` but contain no blocks.
///
/// This function does not allocate buffers proportional to declared block or
/// Skippable payload sizes. The limits bound metadata counts; the input slice
/// itself remains fully resident in caller-managed memory.
///
/// # Errors
///
/// Returns the same structural errors as [`inspect`], plus
/// [`ZstdError::ResourceLimitExceeded`] when a configured count limit is
/// exhausted.
pub fn inspect_with_limits(
    input: &[u8],
    limits: InspectionLimits,
) -> Result<ZstdFile, ZstdError> {
    let input_size = u64::try_from(input.len())
        .map_err(|_| ZstdError::ArithmeticOverflow { offset: u64::MAX })?;
    let mut cursor = Cursor::new(input);
    let mut frames = Vec::new();
    let mut total_blocks = 0;

    while frames.is_empty() || cursor.remaining() != 0 {
        if frames.len() >= limits.max_frames {
            return Err(ZstdError::ResourceLimitExceeded {
                offset: public_offset(cursor.position())?,
                resource: ResourceLimitKind::Frames,
                limit: limits.max_frames,
            });
        }

        let index = frames.len();
        frames.push(parse_frame(
            &mut cursor,
            index,
            limits,
            &mut total_blocks,
        )?);
    }

    Ok(ZstdFile { input_size, frames })
}

fn parse_frame(
    cursor: &mut Cursor<'_>,
    index: usize,
    limits: InspectionLimits,
    total_blocks: &mut usize,
) -> Result<Frame, ZstdError> {
    let frame_start = cursor.position();
    let magic = cursor.read_u32_le()?;

    match magic {
        STANDARD_MAGIC => parse_standard_frame(cursor, index, frame_start, limits, total_blocks),
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
    index: usize,
    frame_start: usize,
    limits: InspectionLimits,
    total_blocks: &mut usize,
) -> Result<Frame, ZstdError> {
    let header = parse_frame_header(cursor)?;
    let blocks = parse_blocks_with_limits(
        cursor,
        header.window_size,
        limits.max_blocks_per_frame,
        limits.max_total_blocks,
        total_blocks,
    )?;
    validate_frame_content_size(&header, &blocks)?;
    let content_checksum = parse_content_checksum(cursor, header.content_checksum_flag)?;

    Ok(Frame {
        index,
        span: span_between(frame_start, cursor.position())?,
        kind: FrameKind::Standard(StandardFrame {
            magic_span: fixed_span(frame_start, 4)?,
            header,
            blocks,
            content_checksum,
        }),
    })
}

fn validate_frame_content_size(
    header: &crate::FrameHeader,
    blocks: &[crate::Block],
) -> Result<(), ZstdError> {
    let Some(frame_content_size) = header.frame_content_size else {
        return Ok(());
    };

    let (minimum, maximum) = decoded_size_bounds(blocks, header.window_size)?;
    let declared = u128::from(frame_content_size.value);

    if declared < minimum || declared > maximum {
        return Err(ZstdError::FrameContentSizeMismatch {
            offset: frame_content_size.span.offset,
            declared: frame_content_size.value,
            minimum,
            maximum,
        });
    }

    Ok(())
}

fn parse_content_checksum(
    cursor: &mut Cursor<'_>,
    present: bool,
) -> Result<Option<ContentChecksum>, ZstdError> {
    if !present {
        return Ok(None);
    }

    let checksum_start = cursor.position();
    let value = cursor.read_u32_le()?;

    Ok(Some(ContentChecksum {
        span: fixed_span(checksum_start, 4)?,
        value,
    }))
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
