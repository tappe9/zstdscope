use zstdscope::{FrameKind, ZstdError, inspect};

const STANDARD_MAGIC: u32 = 0xFD2F_B528;
const SKIPPABLE_MAGIC_MIN: u32 = 0x184D_2A50;
const SKIPPABLE_MAGIC_MAX: u32 = 0x184D_2A5F;

#[test]
fn empty_input_is_a_typed_top_level_eof() {
    assert_eq!(
        inspect(&[]).unwrap_err(),
        ZstdError::UnexpectedEof {
            offset: 0,
            needed: 4,
            remaining: 0,
        }
    );
}

#[test]
fn one_to_three_trailing_magic_bytes_are_not_ignored() {
    for remaining in 1..=3 {
        let input = vec![0xAA; remaining];

        assert_eq!(
            inspect(&input).unwrap_err(),
            ZstdError::UnexpectedEof {
                offset: 0,
                needed: 4,
                remaining,
            }
        );
    }
}

#[test]
fn invalid_top_level_magic_is_typed_and_location_aware() {
    let magic: u32 = 0x1234_5678;

    assert_eq!(
        inspect(&magic.to_le_bytes()).unwrap_err(),
        ZstdError::InvalidMagic { offset: 0, magic }
    );
}

#[test]
fn standard_magic_dispatches_into_the_complete_standard_frame_parser() {
    let mut input = STANDARD_MAGIC.to_le_bytes().to_vec();
    assert_eq!(
        inspect(&input).unwrap_err(),
        ZstdError::UnexpectedEof {
            offset: 4,
            needed: 1,
            remaining: 0,
        }
    );

    input.push(0x00);
    assert_eq!(
        inspect(&input).unwrap_err(),
        ZstdError::UnexpectedEof {
            offset: 5,
            needed: 1,
            remaining: 0,
        }
    );

    input.push(0x00);
    assert_eq!(
        inspect(&input).unwrap_err(),
        ZstdError::UnexpectedEof {
            offset: 6,
            needed: 3,
            remaining: 0,
        }
    );

    input.extend_from_slice(&[0x01, 0x00, 0x00]);
    let file = inspect(&input).unwrap();
    assert_eq!(file.frames.len(), 1);
    assert!(matches!(file.frames[0].kind, FrameKind::Standard(_)));
}

#[test]
fn reserved_standard_frame_header_bit_is_a_typed_error() {
    let mut input = STANDARD_MAGIC.to_le_bytes().to_vec();
    input.push(0x08);

    assert_eq!(
        inspect(&input).unwrap_err(),
        ZstdError::ReservedFrameHeaderBit { offset: 4 }
    );
}

#[test]
fn truncated_standard_frame_optional_field_reports_its_offset() {
    let mut input = STANDARD_MAGIC.to_le_bytes().to_vec();
    input.extend_from_slice(&[0x02, 0x00, 0xAA]);

    assert_eq!(
        inspect(&input).unwrap_err(),
        ZstdError::UnexpectedEof {
            offset: 6,
            needed: 2,
            remaining: 1,
        }
    );
}

#[test]
fn reserved_standard_block_type_is_typed_and_location_aware() {
    let mut input = STANDARD_MAGIC.to_le_bytes().to_vec();
    input.extend_from_slice(&[0x00, 0x00, 0x07, 0x00, 0x00]);

    assert_eq!(
        inspect(&input).unwrap_err(),
        ZstdError::ReservedBlockType { offset: 6 }
    );
}

#[test]
fn truncated_standard_block_content_reports_the_content_offset() {
    let mut input = STANDARD_MAGIC.to_le_bytes().to_vec();
    input.extend_from_slice(&[0x00, 0x00, 0x11, 0x00, 0x00, 0xAA]);

    assert_eq!(
        inspect(&input).unwrap_err(),
        ZstdError::UnexpectedEof {
            offset: 9,
            needed: 2,
            remaining: 1,
        }
    );
}

#[test]
fn recognizes_all_sixteen_skippable_magic_variants() {
    for magic in SKIPPABLE_MAGIC_MIN..=SKIPPABLE_MAGIC_MAX {
        let input = skippable_frame(magic, &[]);
        let file = inspect(&input).unwrap();

        assert_eq!(file.input_size, 8);
        assert_eq!(file.frames.len(), 1);
        assert_eq!(file.frames[0].index, 0);
        assert_eq!(file.frames[0].span.offset, 0);
        assert_eq!(file.frames[0].span.length, 8);

        match &file.frames[0].kind {
            FrameKind::Skippable(frame) => {
                assert_eq!(frame.magic, magic);
                assert_eq!(frame.variant, (magic - SKIPPABLE_MAGIC_MIN) as u8);
                assert_eq!(frame.magic_span.offset, 0);
                assert_eq!(frame.magic_span.length, 4);
                assert_eq!(frame.size_field_span.offset, 4);
                assert_eq!(frame.size_field_span.length, 4);
                assert_eq!(frame.declared_payload_size, 0);
                assert_eq!(frame.payload_span.offset, 8);
                assert_eq!(frame.payload_span.length, 0);
            }
            FrameKind::Standard(_) => panic!("skippable magic dispatched as a standard frame"),
        }
    }
}

#[test]
fn skippable_payload_is_skipped_without_losing_source_spans() {
    let input = skippable_frame(SKIPPABLE_MAGIC_MIN + 3, &[0x11, 0x22, 0x33]);
    let file = inspect(&input).unwrap();

    assert_eq!(file.input_size, 11);
    assert_eq!(file.frames[0].span.offset, 0);
    assert_eq!(file.frames[0].span.length, 11);

    match &file.frames[0].kind {
        FrameKind::Skippable(frame) => {
            assert_eq!(frame.variant, 3);
            assert_eq!(frame.declared_payload_size, 3);
            assert_eq!(frame.payload_span.offset, 8);
            assert_eq!(frame.payload_span.length, 3);
        }
        FrameKind::Standard(_) => panic!("skippable magic dispatched as a standard frame"),
    }
}

#[test]
fn concatenated_skippable_frames_preserve_order_indexes_and_offsets() {
    let first = skippable_frame(SKIPPABLE_MAGIC_MIN, &[]);
    let second = skippable_frame(SKIPPABLE_MAGIC_MIN + 7, &[0xCC, 0xDD]);
    let third = skippable_frame(SKIPPABLE_MAGIC_MAX, &[]);
    let input = [first, second, third].concat();

    let file = inspect(&input).unwrap();

    assert_eq!(file.frames.len(), 3);
    assert_eq!(file.frames[0].index, 0);
    assert_eq!(file.frames[0].span.offset, 0);
    assert_eq!(file.frames[0].span.length, 8);
    assert_eq!(file.frames[1].index, 1);
    assert_eq!(file.frames[1].span.offset, 8);
    assert_eq!(file.frames[1].span.length, 10);
    assert_eq!(file.frames[2].index, 2);
    assert_eq!(file.frames[2].span.offset, 18);
    assert_eq!(file.frames[2].span.length, 8);
}

#[test]
fn mixed_standard_skippable_standard_stream_preserves_exact_boundaries() {
    // ZSTD-FORMAT section 13: each concatenated top-level frame is independently delimited.
    let first = minimal_standard_frame();
    let middle = skippable_frame(SKIPPABLE_MAGIC_MIN + 5, &[0xCC, 0xDD]);
    let third = minimal_standard_frame();
    let input = [first, middle, third].concat();

    let file = inspect(&input).unwrap();

    assert_eq!(file.frames.len(), 3);
    assert_eq!(file.frames[0].index, 0);
    assert_eq!(file.frames[0].span.offset, 0);
    assert_eq!(file.frames[0].span.length, 9);
    assert!(matches!(file.frames[0].kind, FrameKind::Standard(_)));

    assert_eq!(file.frames[1].index, 1);
    assert_eq!(file.frames[1].span.offset, 9);
    assert_eq!(file.frames[1].span.length, 10);
    let FrameKind::Skippable(skippable) = &file.frames[1].kind else {
        panic!("middle frame was not skippable");
    };
    assert_eq!(skippable.variant, 5);
    assert_eq!(skippable.size_field_span.offset, 13);
    assert_eq!(skippable.size_field_span.length, 4);
    assert_eq!(skippable.payload_span.offset, 17);
    assert_eq!(skippable.payload_span.length, 2);

    assert_eq!(file.frames[2].index, 2);
    assert_eq!(file.frames[2].span.offset, 19);
    assert_eq!(file.frames[2].span.length, 9);
    assert!(matches!(file.frames[2].kind, FrameKind::Standard(_)));
}

#[test]
fn trailing_partial_magic_after_complete_frame_is_an_error() {
    for trailing_len in 1..=3 {
        let mut input = skippable_frame(SKIPPABLE_MAGIC_MIN, &[]);
        input.extend(std::iter::repeat_n(0xAA, trailing_len));

        assert_eq!(
            inspect(&input).unwrap_err(),
            ZstdError::UnexpectedEof {
                offset: 8,
                needed: 4,
                remaining: trailing_len,
            }
        );
    }
}

#[test]
fn truncated_skippable_size_field_is_a_typed_eof_at_the_size_field() {
    // ZSTD-FORMAT section 12: the skippable Frame Size is exactly four little-endian bytes.
    for remaining in 0..=3 {
        let mut input = SKIPPABLE_MAGIC_MIN.to_le_bytes().to_vec();
        input.extend_from_slice(&[0xAA, 0xBB, 0xCC][..remaining]);

        assert_eq!(
            inspect(&input).unwrap_err(),
            ZstdError::UnexpectedEof {
                offset: 4,
                needed: 4,
                remaining,
            }
        );
    }
}

#[test]
fn truncated_skippable_payload_is_a_typed_eof() {
    let mut input = Vec::new();
    input.extend_from_slice(&SKIPPABLE_MAGIC_MIN.to_le_bytes());
    input.extend_from_slice(&4_u32.to_le_bytes());
    input.extend_from_slice(&[0xAA, 0xBB]);

    assert_eq!(
        inspect(&input).unwrap_err(),
        ZstdError::UnexpectedEof {
            offset: 8,
            needed: 4,
            remaining: 2,
        }
    );
}

#[test]
fn maximum_declared_skippable_payload_is_rejected_without_payload_allocation() {
    // The parser must validate/skip the declared size without allocating a payload-sized buffer.
    let mut input = Vec::new();
    input.extend_from_slice(&SKIPPABLE_MAGIC_MIN.to_le_bytes());
    input.extend_from_slice(&u32::MAX.to_le_bytes());

    let expected = match usize::try_from(u32::MAX) {
        Ok(needed) => ZstdError::UnexpectedEof {
            offset: 8,
            needed,
            remaining: 0,
        },
        Err(_) => ZstdError::ArithmeticOverflow { offset: 8 },
    };

    assert_eq!(inspect(&input).unwrap_err(), expected);
}

fn minimal_standard_frame() -> Vec<u8> {
    let mut frame = STANDARD_MAGIC.to_le_bytes().to_vec();
    frame.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0x00]);
    frame
}

fn skippable_frame(magic: u32, payload: &[u8]) -> Vec<u8> {
    let payload_len = u32::try_from(payload.len()).unwrap();
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.extend_from_slice(&magic.to_le_bytes());
    frame.extend_from_slice(&payload_len.to_le_bytes());
    frame.extend_from_slice(payload);
    frame
}
