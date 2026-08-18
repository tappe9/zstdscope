use crate::{
    ByteSpan, DictionaryId, FrameContentSize, FrameHeader, ZstdError, cursor::Cursor,
};

const RESERVED_BIT: u8 = 0x08;
const UNUSED_BIT: u8 = 0x10;
const SINGLE_SEGMENT_BIT: u8 = 0x20;
const CONTENT_CHECKSUM_BIT: u8 = 0x04;

pub(super) fn parse_frame_header(cursor: &mut Cursor<'_>) -> Result<FrameHeader, ZstdError> {
    let header_start = cursor.position();
    let descriptor_start = cursor.position();
    let descriptor = cursor.read_u8()?;
    let descriptor_offset = public_offset(descriptor_start)?;

    if descriptor & RESERVED_BIT != 0 {
        return Err(ZstdError::ReservedFrameHeaderBit {
            offset: descriptor_offset,
        });
    }

    let single_segment = descriptor & SINGLE_SEGMENT_BIT != 0;
    let unused_bit = descriptor & UNUSED_BIT != 0;
    let content_checksum_flag = descriptor & CONTENT_CHECKSUM_BIT != 0;
    let dictionary_id_flag = descriptor & 0x03;
    let frame_content_size_flag = descriptor >> 6;

    let descriptor_span = fixed_span(descriptor_start, 1)?;

    let (window_descriptor_span, descriptor_window_size) = if single_segment {
        (None, None)
    } else {
        let window_descriptor_start = cursor.position();
        let window_descriptor = cursor.read_u8()?;
        let window_descriptor_offset = public_offset(window_descriptor_start)?;
        let window_size = derive_window_size(window_descriptor, window_descriptor_offset)?;

        (
            Some(fixed_span(window_descriptor_start, 1)?),
            Some(window_size),
        )
    };

    let dictionary_id = read_dictionary_id(cursor, dictionary_id_flag)?;
    let frame_content_size = read_frame_content_size(
        cursor,
        frame_content_size_width(frame_content_size_flag, single_segment),
    )?;

    let window_size = if single_segment {
        frame_content_size.as_ref().map_or(0, |size| size.value)
    } else {
        descriptor_window_size.unwrap_or(0)
    };

    Ok(FrameHeader {
        span: span_between(header_start, cursor.position())?,
        descriptor,
        descriptor_span,
        window_descriptor_span,
        frame_content_size,
        dictionary_id,
        window_size,
        content_checksum_flag,
        single_segment,
        unused_bit,
    })
}

fn read_dictionary_id(
    cursor: &mut Cursor<'_>,
    flag: u8,
) -> Result<Option<DictionaryId>, ZstdError> {
    let width = match flag {
        0 => return Ok(None),
        1 => 1,
        2 => 2,
        3 => 4,
        _ => unreachable!(),
    };
    let start = cursor.position();
    let encoded = match width {
        1 => u32::from(cursor.read_u8()?),
        2 => u32::from(cursor.read_u16_le()?),
        4 => cursor.read_u32_le()?,
        _ => unreachable!(),
    };

    Ok(Some(DictionaryId {
        encoded,
        span: fixed_span(start, width)?,
    }))
}

fn read_frame_content_size(
    cursor: &mut Cursor<'_>,
    width: usize,
) -> Result<Option<FrameContentSize>, ZstdError> {
    if width == 0 {
        return Ok(None);
    }

    let start = cursor.position();
    let offset = public_offset(start)?;
    let value = match width {
        1 => u64::from(cursor.read_u8()?),
        2 => u64::from(cursor.read_u16_le()?)
            .checked_add(256)
            .ok_or(ZstdError::ArithmeticOverflow { offset })?,
        4 => u64::from(cursor.read_u32_le()?),
        8 => cursor.read_u64_le()?,
        _ => return Err(ZstdError::ArithmeticOverflow { offset }),
    };

    Ok(Some(FrameContentSize {
        value,
        span: fixed_span(start, width)?,
    }))
}

fn frame_content_size_width(flag: u8, single_segment: bool) -> usize {
    match flag {
        0 if single_segment => 1,
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 8,
        _ => 0,
    }
}

fn derive_window_size(window_descriptor: u8, offset: u64) -> Result<u64, ZstdError> {
    let exponent = u32::from(window_descriptor >> 3);
    let mantissa = u64::from(window_descriptor & 0x07);
    let window_log = 10_u32
        .checked_add(exponent)
        .ok_or(ZstdError::ArithmeticOverflow { offset })?;
    let window_base = 1_u64
        .checked_shl(window_log)
        .ok_or(ZstdError::ArithmeticOverflow { offset })?;
    let window_add = (window_base / 8)
        .checked_mul(mantissa)
        .ok_or(ZstdError::ArithmeticOverflow { offset })?;

    window_base
        .checked_add(window_add)
        .ok_or(ZstdError::ArithmeticOverflow { offset })
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
