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
fn standard_magic_dispatches_to_the_standard_frame_parser_stub() {
    assert_eq!(
        inspect(&STANDARD_MAGIC.to_le_bytes()).unwrap_err(),
        ZstdError::StandardFrameNotImplemented { offset: 0 }
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

fn skippable_frame(magic: u32, payload: &[u8]) -> Vec<u8> {
    let payload_len = u32::try_from(payload.len()).unwrap();
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.extend_from_slice(&magic.to_le_bytes());
    frame.extend_from_slice(&payload_len.to_le_bytes());
    frame.extend_from_slice(payload);
    frame
}
