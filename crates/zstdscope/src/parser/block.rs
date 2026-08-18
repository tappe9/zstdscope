use crate::{Block, BlockType, ByteSpan, ZstdError, cursor::Cursor};

const BLOCK_HEADER_SIZE: usize = 3;
const FORMAT_BLOCK_MAXIMUM_SIZE: u32 = 128 * 1024;

pub(super) fn parse_blocks(
    cursor: &mut Cursor<'_>,
    window_size: u64,
) -> Result<Vec<Block>, ZstdError> {
    let maximum = block_maximum_size(cursor.position(), window_size)?;
    let mut blocks = Vec::new();

    loop {
        let block = parse_block(cursor, blocks.len(), maximum)?;
        let is_last = block.is_last;
        blocks.push(block);

        if is_last {
            return Ok(blocks);
        }
    }
}

fn parse_block(
    cursor: &mut Cursor<'_>,
    index: usize,
    maximum: u32,
) -> Result<Block, ZstdError> {
    let header_start = cursor.position();
    let header_offset = public_offset(header_start)?;
    let header = cursor.read_u24_le()?;
    let is_last = header & 1 != 0;
    let block_type = match (header >> 1) & 0x03 {
        0 => BlockType::Raw,
        1 => BlockType::Rle,
        2 => BlockType::Compressed,
        _ => {
            return Err(ZstdError::ReservedBlockType {
                offset: header_offset,
            });
        }
    };
    let declared_size = header >> 3;

    validate_block_size(declared_size, maximum, header_offset)?;

    let encoded_content_size = match block_type {
        BlockType::Raw | BlockType::Compressed => declared_size,
        BlockType::Rle => 1,
    };

    validate_block_size(encoded_content_size, maximum, header_offset)?;

    let content_start = cursor.position();
    let content_offset = public_offset(content_start)?;
    let content_length =
        usize::try_from(encoded_content_size).map_err(|_| ZstdError::ArithmeticOverflow {
            offset: content_offset,
        })?;

    cursor.skip(content_length)?;

    Ok(Block {
        index,
        header_span: fixed_span(header_start, BLOCK_HEADER_SIZE)?,
        content_span: ByteSpan {
            offset: content_offset,
            length: u64::from(encoded_content_size),
        },
        block_type,
        declared_size,
        encoded_content_size,
        is_last,
    })
}

fn validate_block_size(size: u32, maximum: u32, offset: u64) -> Result<(), ZstdError> {
    if size > maximum {
        return Err(ZstdError::InvalidBlockSize {
            offset,
            size,
            maximum,
        });
    }

    Ok(())
}

fn block_maximum_size(position: usize, window_size: u64) -> Result<u32, ZstdError> {
    let offset = public_offset(position)?;
    let maximum = window_size.min(u64::from(FORMAT_BLOCK_MAXIMUM_SIZE));

    u32::try_from(maximum).map_err(|_| ZstdError::ArithmeticOverflow { offset })
}

fn fixed_span(start: usize, length: usize) -> Result<ByteSpan, ZstdError> {
    let offset = public_offset(start)?;
    let length = u64::try_from(length).map_err(|_| ZstdError::ArithmeticOverflow { offset })?;
    Ok(ByteSpan { offset, length })
}

fn public_offset(position: usize) -> Result<u64, ZstdError> {
    u64::try_from(position).map_err(|_| ZstdError::ArithmeticOverflow { offset: u64::MAX })
}
