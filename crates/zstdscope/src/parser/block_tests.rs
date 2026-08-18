use super::block::parse_blocks;
use crate::{Block, BlockType, ByteSpan, ZstdError, cursor::Cursor};

const PREFIX_LEN: usize = 6;
const BLOCK_MAXIMUM_SIZE: u32 = 128 * 1024;

#[test]
fn parses_raw_block_with_exact_spans_and_sizes() {
    // Zstandard format, Blocks: Raw Block_Content length equals Block_Size.
    let mut bytes = block_header(true, 0, 3).to_vec();
    bytes.extend_from_slice(&[0x11, 0x22, 0x33]);

    let (blocks, position, remaining) = parse_blocks_with_prefix(&bytes, 1024).unwrap();

    assert_eq!(
        blocks,
        vec![Block {
            index: 0,
            header_span: span(6, 3),
            content_span: span(9, 3),
            block_type: BlockType::Raw,
            declared_size: 3,
            encoded_content_size: 3,
            is_last: true,
        }]
    );
    assert_eq!(position, 12);
    assert_eq!(remaining, 0);
}

#[test]
fn rle_uses_one_encoded_byte_and_preserves_multiple_block_offsets() {
    // Zstandard format, Blocks: RLE Block_Size is a repetition count, while content is 1 byte.
    let mut bytes = block_header(false, 1, 17).to_vec();
    bytes.push(0xAA);
    bytes.extend_from_slice(&block_header(true, 0, 2));
    bytes.extend_from_slice(&[0xBB, 0xCC]);

    let (blocks, position, remaining) = parse_blocks_with_prefix(&bytes, 1024).unwrap();

    assert_eq!(
        blocks,
        vec![
            Block {
                index: 0,
                header_span: span(6, 3),
                content_span: span(9, 1),
                block_type: BlockType::Rle,
                declared_size: 17,
                encoded_content_size: 1,
                is_last: false,
            },
            Block {
                index: 1,
                header_span: span(10, 3),
                content_span: span(13, 2),
                block_type: BlockType::Raw,
                declared_size: 2,
                encoded_content_size: 2,
                is_last: true,
            },
        ]
    );
    assert_eq!(position, 15);
    assert_eq!(remaining, 0);
}

#[test]
fn compressed_content_is_opaque_and_parsing_stops_after_the_last_block() {
    // Zstandard format, Compressed Blocks: Issue #6 records and skips internals as opaque bytes.
    let mut bytes = block_header(true, 2, 4).to_vec();
    bytes.extend_from_slice(&[0xFF, 0x00, 0xDE, 0xAD]);
    bytes.extend_from_slice(&[0xCA, 0xFE]);

    let (blocks, position, remaining) = parse_blocks_with_prefix(&bytes, 1024).unwrap();

    assert_eq!(
        blocks,
        vec![Block {
            index: 0,
            header_span: span(6, 3),
            content_span: span(9, 4),
            block_type: BlockType::Compressed,
            declared_size: 4,
            encoded_content_size: 4,
            is_last: true,
        }]
    );
    assert_eq!(position, 13);
    assert_eq!(remaining, 2);
}

#[test]
fn reserved_block_type_is_rejected_at_the_header_offset() {
    // Zstandard format, Block_Type value 3 is reserved and represents corrupted data.
    let bytes = block_header(true, 3, 0);

    assert_eq!(
        parse_blocks_with_prefix(&bytes, 1024).unwrap_err(),
        ZstdError::ReservedBlockType { offset: 6 }
    );
}

#[test]
fn truncated_block_header_reports_the_header_offset() {
    for remaining in 0..=2 {
        let bytes = vec![0xAA; remaining];

        assert_eq!(
            parse_blocks_with_prefix(&bytes, 1024).unwrap_err(),
            ZstdError::UnexpectedEof {
                offset: 6,
                needed: 3,
                remaining,
            }
        );
    }
}

#[test]
fn truncated_content_uses_each_block_types_encoded_length() {
    let cases = [
        (0_u8, 3_u32, vec![0x11, 0x22], 3_usize, 2_usize),
        (1_u8, 99_u32, vec![], 1_usize, 0_usize),
        (2_u8, 3_u32, vec![0x11, 0x22], 3_usize, 2_usize),
    ];

    for (block_type, declared_size, payload, needed, remaining) in cases {
        let mut bytes = block_header(true, block_type, declared_size).to_vec();
        bytes.extend_from_slice(&payload);

        assert_eq!(
            parse_blocks_with_prefix(&bytes, 1024).unwrap_err(),
            ZstdError::UnexpectedEof {
                offset: 9,
                needed,
                remaining,
            }
        );
    }
}

#[test]
fn window_size_limits_all_block_types_at_the_exact_boundary() {
    let cases = [
        (0_u8, BlockType::Raw, vec![0x11; 4], 4_u32),
        (1_u8, BlockType::Rle, vec![0x22], 1_u32),
        (2_u8, BlockType::Compressed, vec![0x33; 4], 4_u32),
    ];

    for (block_type, expected_type, payload, encoded_size) in cases {
        let mut bytes = block_header(true, block_type, 4).to_vec();
        bytes.extend_from_slice(&payload);
        let (blocks, _, _) = parse_blocks_with_prefix(&bytes, 4).unwrap();

        assert_eq!(blocks[0].block_type, expected_type);
        assert_eq!(blocks[0].declared_size, 4);
        assert_eq!(blocks[0].encoded_content_size, encoded_size);

        let oversized = block_header(true, block_type, 5);
        assert_eq!(
            parse_blocks_with_prefix(&oversized, 4).unwrap_err(),
            ZstdError::InvalidBlockSize {
                offset: 6,
                size: 5,
                maximum: 4,
            }
        );
    }
}

#[test]
fn format_ceiling_is_exactly_128_kib() {
    // Zstandard format, Block_Maximum_Size: min(Window_Size, 128 KiB).
    let mut bytes = block_header(true, 0, BLOCK_MAXIMUM_SIZE).to_vec();
    bytes.extend(vec![0_u8; BLOCK_MAXIMUM_SIZE as usize]);

    let (blocks, position, remaining) =
        parse_blocks_with_prefix(&bytes, u64::from(BLOCK_MAXIMUM_SIZE) + 1).unwrap();

    assert_eq!(blocks[0].declared_size, BLOCK_MAXIMUM_SIZE);
    assert_eq!(blocks[0].encoded_content_size, BLOCK_MAXIMUM_SIZE);
    assert_eq!(position, PREFIX_LEN + 3 + BLOCK_MAXIMUM_SIZE as usize);
    assert_eq!(remaining, 0);

    let oversized = block_header(true, 0, BLOCK_MAXIMUM_SIZE + 1);
    assert_eq!(
        parse_blocks_with_prefix(&oversized, u64::MAX).unwrap_err(),
        ZstdError::InvalidBlockSize {
            offset: 6,
            size: BLOCK_MAXIMUM_SIZE + 1,
            maximum: BLOCK_MAXIMUM_SIZE,
        }
    );
}

#[test]
fn zero_maximum_allows_empty_raw_but_not_one_byte_rle_content() {
    // Both encoded Block_Content and decompressed size must fit Block_Maximum_Size.
    let raw = block_header(true, 0, 0);
    let (blocks, position, remaining) = parse_blocks_with_prefix(&raw, 0).unwrap();

    assert_eq!(blocks[0].content_span, span(9, 0));
    assert_eq!(position, 9);
    assert_eq!(remaining, 0);

    let mut rle = block_header(true, 1, 0).to_vec();
    rle.push(0xAA);
    assert_eq!(
        parse_blocks_with_prefix(&rle, 0).unwrap_err(),
        ZstdError::InvalidBlockSize {
            offset: 6,
            size: 1,
            maximum: 0,
        }
    );
}

fn parse_blocks_with_prefix(
    bytes: &[u8],
    window_size: u64,
) -> Result<(Vec<Block>, usize, usize), ZstdError> {
    let mut input = Vec::with_capacity(PREFIX_LEN + bytes.len());
    input.resize(PREFIX_LEN, 0);
    input.extend_from_slice(bytes);

    let mut cursor = Cursor::new(&input);
    cursor.skip(PREFIX_LEN).unwrap();
    let blocks = parse_blocks(&mut cursor, window_size)?;

    Ok((blocks, cursor.position(), cursor.remaining()))
}

fn block_header(is_last: bool, block_type: u8, declared_size: u32) -> [u8; 3] {
    assert!(block_type <= 3);
    assert!(declared_size <= 0x1F_FFFF);

    let value = (declared_size << 3)
        | (u32::from(block_type) << 1)
        | u32::from(u8::from(is_last));
    let bytes = value.to_le_bytes();
    [bytes[0], bytes[1], bytes[2]]
}

fn span(offset: u64, length: u64) -> ByteSpan {
    ByteSpan { offset, length }
}
