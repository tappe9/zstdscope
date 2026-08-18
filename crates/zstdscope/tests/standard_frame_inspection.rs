use zstdscope::{BlockType, FrameKind, ZstdError, inspect};

const STANDARD_MAGIC: u32 = 0xFD2F_B528;
const SKIPPABLE_MAGIC_MIN: u32 = 0x184D_2A50;

#[test]
fn minimal_standard_frame_produces_complete_model() {
    let input = standard_frame(false, &[(true, 0, &[])], None);
    let file = inspect(&input).unwrap();

    assert_eq!(file.input_size, 9);
    assert_eq!(file.frames.len(), 1);
    let frame = &file.frames[0];
    assert_eq!(frame.index, 0);
    assert_eq!(frame.span.offset, 0);
    assert_eq!(frame.span.length, 9);

    let FrameKind::Standard(standard) = &frame.kind else {
        panic!("standard magic did not produce a StandardFrame");
    };
    assert_eq!(standard.magic_span.offset, 0);
    assert_eq!(standard.magic_span.length, 4);
    assert_eq!(standard.header.span.offset, 4);
    assert_eq!(standard.header.span.length, 2);
    assert!(!standard.header.content_checksum_flag);
    assert_eq!(standard.blocks.len(), 1);
    assert_eq!(standard.blocks[0].index, 0);
    assert_eq!(standard.blocks[0].header_span.offset, 6);
    assert_eq!(standard.blocks[0].header_span.length, 3);
    assert_eq!(standard.blocks[0].content_span.offset, 9);
    assert_eq!(standard.blocks[0].content_span.length, 0);
    assert_eq!(standard.blocks[0].block_type, BlockType::Raw);
    assert!(standard.blocks[0].is_last);
    assert_eq!(standard.content_checksum, None);
}

#[test]
fn multiple_blocks_are_preserved_through_last_block() {
    let input = standard_frame(
        false,
        &[(false, 1, &[0xAA]), (true, 0, &[0xBB, 0xCC])],
        None,
    );
    let file = inspect(&input).unwrap();
    let FrameKind::Standard(standard) = &file.frames[0].kind else {
        panic!("expected standard frame");
    };

    assert_eq!(standard.blocks.len(), 2);
    assert_eq!(standard.blocks[0].index, 0);
    assert_eq!(standard.blocks[0].block_type, BlockType::Rle);
    assert_eq!(standard.blocks[0].declared_size, 17);
    assert_eq!(standard.blocks[0].encoded_content_size, 1);
    assert!(!standard.blocks[0].is_last);
    assert_eq!(standard.blocks[1].index, 1);
    assert_eq!(standard.blocks[1].header_span.offset, 10);
    assert_eq!(standard.blocks[1].content_span.offset, 13);
    assert_eq!(standard.blocks[1].content_span.length, 2);
    assert!(standard.blocks[1].is_last);
    assert_eq!(file.frames[0].span.length, 15);
}

#[test]
fn stored_checksum_is_exposed_without_verification() {
    let stored = 0xDEAD_BEEF;
    let input = standard_frame(true, &[(true, 0, &[])], Some(stored));
    let file = inspect(&input).unwrap();
    let FrameKind::Standard(standard) = &file.frames[0].kind else {
        panic!("expected standard frame");
    };

    let checksum = standard
        .content_checksum
        .expect("checksum flag must expose stored metadata");
    assert_eq!(checksum.value, stored);
    assert_eq!(checksum.span.offset, 9);
    assert_eq!(checksum.span.length, 4);
    assert_eq!(file.frames[0].span.offset, 0);
    assert_eq!(file.frames[0].span.length, 13);
}

#[test]
fn truncated_checksum_is_a_typed_eof_at_checksum_start() {
    for remaining in 0..=3 {
        let mut input = standard_frame(true, &[(true, 0, &[])], None);
        input.extend_from_slice(&[0xAA, 0xBB, 0xCC][..remaining]);

        assert_eq!(
            inspect(&input).unwrap_err(),
            ZstdError::UnexpectedEof {
                offset: 9,
                needed: 4,
                remaining,
            }
        );
    }
}

#[test]
fn standard_frame_leaves_cursor_at_next_skippable_frame() {
    let first = standard_frame(false, &[(true, 0, &[])], None);
    let second = skippable_frame(SKIPPABLE_MAGIC_MIN + 2, &[0xCC]);
    let input = [first, second].concat();
    let file = inspect(&input).unwrap();

    assert_eq!(file.frames.len(), 2);
    assert_eq!(file.frames[0].index, 0);
    assert_eq!(file.frames[0].span.offset, 0);
    assert_eq!(file.frames[0].span.length, 9);
    assert_eq!(file.frames[1].index, 1);
    assert_eq!(file.frames[1].span.offset, 9);
    assert_eq!(file.frames[1].span.length, 9);
}

#[test]
fn concatenated_standard_frames_have_exact_boundaries() {
    let first = standard_frame(false, &[(true, 0, &[])], None);
    let second = standard_frame(true, &[(true, 0, &[])], Some(0x0102_0304));
    let input = [first, second].concat();
    let file = inspect(&input).unwrap();

    assert_eq!(file.frames.len(), 2);
    assert_eq!(file.frames[0].span.offset, 0);
    assert_eq!(file.frames[0].span.length, 9);
    assert_eq!(file.frames[1].index, 1);
    assert_eq!(file.frames[1].span.offset, 9);
    assert_eq!(file.frames[1].span.length, 13);
}

fn standard_frame(
    checksum: bool,
    blocks: &[(bool, u8, &[u8])],
    checksum_value: Option<u32>,
) -> Vec<u8> {
    let mut frame = STANDARD_MAGIC.to_le_bytes().to_vec();
    frame.push(if checksum { 0x04 } else { 0x00 });
    frame.push(0x00);

    for (index, (is_last, block_type, payload)) in blocks.iter().enumerate() {
        let declared_size = if *block_type == 1 {
            17
        } else {
            payload.len() as u32
        };
        let value =
            (declared_size << 3) | (u32::from(*block_type) << 1) | u32::from(u8::from(*is_last));
        let bytes = value.to_le_bytes();
        frame.extend_from_slice(&bytes[..3]);
        frame.extend_from_slice(payload);
        assert_eq!(*is_last, index + 1 == blocks.len());
    }

    if let Some(value) = checksum_value {
        frame.extend_from_slice(&value.to_le_bytes());
    }
    frame
}

fn skippable_frame(magic: u32, payload: &[u8]) -> Vec<u8> {
    let mut frame = magic.to_le_bytes().to_vec();
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(payload);
    frame
}
