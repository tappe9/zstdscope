use crate::{
    Block, BlockType, ByteSpan, ResourceLimitKind, ZstdError, cursor::Cursor,
};

const BLOCK_HEADER_SIZE: usize = 3;
const FORMAT_BLOCK_MAXIMUM_SIZE: u32 = 128 * 1024;
const MINIMUM_COMPRESSED_BLOCK_SIZE: u32 = 2;

#[cfg(test)]
pub(super) fn parse_blocks(
    cursor: &mut Cursor<'_>,
    window_size: u64,
) -> Result<Vec<Block>, ZstdError> {
    let mut total_blocks = 0;
    parse_blocks_with_limits(
        cursor,
        window_size,
        usize::MAX,
        usize::MAX,
        &mut total_blocks,
    )
}

pub(super) fn parse_blocks_with_limits(
    cursor: &mut Cursor<'_>,
    window_size: u64,
    max_blocks_per_frame: usize,
    max_total_blocks: usize,
    total_blocks: &mut usize,
) -> Result<Vec<Block>, ZstdError> {
    let maximum = block_maximum_size(cursor.position(), window_size)?;
    let mut blocks = Vec::new();

    loop {
        let header_offset = public_offset(cursor.position())?;
        if blocks.len() >= max_blocks_per_frame {
            return Err(ZstdError::ResourceLimitExceeded {
                offset: header_offset,
                resource: ResourceLimitKind::BlocksPerFrame,
                limit: max_blocks_per_frame,
            });
        }
        if *total_blocks >= max_total_blocks {
            return Err(ZstdError::ResourceLimitExceeded {
                offset: header_offset,
                resource: ResourceLimitKind::TotalBlocks,
                limit: max_total_blocks,
            });
        }

        let block = parse_block(cursor, blocks.len(), maximum)?;
        let is_last = block.is_last;
        blocks.push(block);
        *total_blocks = total_blocks
            .checked_add(1)
            .ok_or(ZstdError::ArithmeticOverflow {
                offset: header_offset,
            })?;

        if is_last {
            return Ok(blocks);
        }
    }
}

pub(super) fn decoded_size_bounds(
    blocks: &[Block],
    window_size: u64,
) -> Result<(u128, u128), ZstdError> {
    let maximum_block_size = u128::from(window_size.min(u64::from(FORMAT_BLOCK_MAXIMUM_SIZE)));
    let mut minimum = 0_u128;
    let mut maximum = 0_u128;

    for block in blocks {
        let offset = block.header_span.offset;
        let exact = u128::from(block.declared_size);

        match block.block_type {
            BlockType::Raw | BlockType::Rle => {
                minimum = minimum
                    .checked_add(exact)
                    .ok_or(ZstdError::ArithmeticOverflow { offset })?;
                maximum = maximum
                    .checked_add(exact)
                    .ok_or(ZstdError::ArithmeticOverflow { offset })?;
            }
            BlockType::Compressed => {
                maximum = maximum
                    .checked_add(maximum_block_size)
                    .ok_or(ZstdError::ArithmeticOverflow { offset })?;
            }
        }
    }

    Ok((minimum, maximum))
}

fn parse_block(cursor: &mut Cursor<'_>, index: usize, maximum: u32) -> Result<Block, ZstdError> {
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
    if block_type == BlockType::Compressed && declared_size < MINIMUM_COMPRESSED_BLOCK_SIZE {
        return Err(ZstdError::InvalidCompressedBlockSize {
            offset: header_offset,
            size: declared_size,
            minimum: MINIMUM_COMPRESSED_BLOCK_SIZE,
        });
    }

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
