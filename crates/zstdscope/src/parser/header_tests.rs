use super::header::parse_frame_header;
use crate::{ByteSpan, FrameHeader, ZstdError, cursor::Cursor};

const PREFIX_LEN: usize = 4;

#[test]
fn parses_non_single_segment_header_without_optional_fields() {
    // Frame_Header_Descriptor=0 means no FCS/Dictionary ID fields and requires Window_Descriptor.
    let header = parse_header(&[0x00, 0x00]).unwrap();

    assert_eq!(header.span, span(4, 2));
    assert_eq!(header.descriptor, 0x00);
    assert_eq!(header.descriptor_span, span(4, 1));
    assert_eq!(header.window_descriptor_span, Some(span(5, 1)));
    assert_eq!(header.frame_content_size, None);
    assert_eq!(header.dictionary_id, None);
    assert_eq!(header.window_size, 1024);
    assert!(!header.content_checksum_flag);
    assert!(!header.single_segment);
    assert!(!header.unused_bit);
}

#[test]
fn preserves_descriptor_flags_explicit_zero_dictionary_and_exact_spans() {
    // FCS flag=1 (2 bytes), Single Segment, Unused bit, checksum flag, DID flag=2 (2 bytes).
    let header = parse_header(&[0x76, 0x00, 0x00, 0x00, 0x00]).unwrap();

    assert_eq!(header.span, span(4, 5));
    assert_eq!(header.descriptor, 0x76);
    assert_eq!(header.descriptor_span, span(4, 1));
    assert_eq!(header.window_descriptor_span, None);
    assert_eq!(
        header.dictionary_id,
        Some(crate::DictionaryId {
            encoded: 0,
            span: span(5, 2),
        })
    );
    assert_eq!(
        header.frame_content_size,
        Some(crate::FrameContentSize {
            value: 256,
            span: span(7, 2),
        })
    );
    assert_eq!(header.window_size, 256);
    assert!(header.content_checksum_flag);
    assert!(header.single_segment);
    assert!(header.unused_bit);
}

#[test]
fn dictionary_id_widths_follow_descriptor_flag() {
    let cases = [
        (0x00, vec![0x00], None, 0_u64),
        (0x01, vec![0x00, 0xAB], Some(0xAB), 1_u64),
        (0x02, vec![0x00, 0xEF, 0xBE], Some(0xBEEF), 2_u64),
        (
            0x03,
            vec![0x00, 0xEF, 0xBE, 0xAD, 0xDE],
            Some(0xDEAD_BEEF),
            4_u64,
        ),
    ];

    for (descriptor, rest, expected, width) in cases {
        let mut bytes = vec![descriptor];
        bytes.extend(rest);
        let header = parse_header(&bytes).unwrap();

        match (header.dictionary_id, expected) {
            (None, None) => {}
            (Some(dictionary_id), Some(encoded)) => {
                assert_eq!(dictionary_id.encoded, encoded);
                assert_eq!(dictionary_id.span, span(6, width));
            }
            other => panic!("unexpected dictionary state: {other:?}"),
        }
    }
}

#[test]
fn frame_content_size_widths_follow_flag_and_two_byte_form_adds_256() {
    let cases = [
        (vec![0x20, 0x7F], 127_u64, span(5, 1), 127_u64),
        (vec![0x40, 0x00, 0x00, 0x00], 256_u64, span(6, 2), 1024_u64),
        (
            vec![0x80, 0x00, 0x78, 0x56, 0x34, 0x12],
            0x1234_5678_u64,
            span(6, 4),
            1024_u64,
        ),
        (
            vec![0xC0, 0x00, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01],
            0x0102_0304_0506_0708_u64,
            span(6, 8),
            1024_u64,
        ),
    ];

    for (bytes, expected_value, expected_span, expected_window_size) in cases {
        let header = parse_header(&bytes).unwrap();
        let frame_content_size = header.frame_content_size.unwrap();

        assert_eq!(frame_content_size.value, expected_value);
        assert_eq!(frame_content_size.span, expected_span);
        assert_eq!(header.window_size, expected_window_size);
    }
}

#[test]
fn window_descriptor_minimum_and_maximum_are_derived_with_checked_arithmetic() {
    // Window Descriptor: exponent bits 7..3, mantissa bits 2..0.
    let minimum = parse_header(&[0x00, 0x00]).unwrap();
    assert_eq!(minimum.window_size, 1024);

    let maximum = parse_header(&[0x00, 0xFF]).unwrap();
    assert_eq!(maximum.window_size, 4_123_168_604_160);
}

#[test]
fn reserved_descriptor_bit_is_rejected_at_descriptor_offset() {
    assert_eq!(
        parse_header(&[0x08]).unwrap_err(),
        ZstdError::ReservedFrameHeaderBit { offset: 4 }
    );
}

#[test]
fn truncated_descriptor_and_optional_fields_report_exact_offsets() {
    let cases = [
        (vec![], eof(4, 1, 0)),
        (vec![0x00], eof(5, 1, 0)),
        (vec![0x01, 0x00], eof(6, 1, 0)),
        (vec![0x02, 0x00, 0xAA], eof(6, 2, 1)),
        (vec![0x03, 0x00, 0xAA, 0xBB, 0xCC], eof(6, 4, 3)),
        (vec![0x20], eof(5, 1, 0)),
        (vec![0x40, 0x00, 0xAA], eof(6, 2, 1)),
        (vec![0x80, 0x00, 0xAA, 0xBB, 0xCC], eof(6, 4, 3)),
        (
            vec![0xC0, 0x00, 1, 2, 3, 4, 5, 6, 7],
            eof(6, 8, 7),
        ),
    ];

    for (bytes, expected_error) in cases {
        assert_eq!(parse_header(&bytes).unwrap_err(), expected_error);
    }
}

#[test]
fn parser_stops_exactly_at_the_end_of_the_header() {
    let mut input = vec![0_u8; PREFIX_LEN];
    input.extend_from_slice(&[0x76, 0x34, 0x12, 0x00, 0x00]);
    input.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    let mut cursor = Cursor::new(&input);
    cursor.skip(PREFIX_LEN).unwrap();

    let header = parse_frame_header(&mut cursor).unwrap();

    assert_eq!(header.span, span(4, 5));
    assert_eq!(cursor.position(), 9);
    assert_eq!(cursor.remaining(), 3);
}

fn parse_header(bytes: &[u8]) -> Result<FrameHeader, ZstdError> {
    let mut input = vec![0_u8; PREFIX_LEN];
    input.extend_from_slice(bytes);
    let mut cursor = Cursor::new(&input);
    cursor.skip(PREFIX_LEN).unwrap();

    parse_frame_header(&mut cursor)
}

fn span(offset: u64, length: u64) -> ByteSpan {
    ByteSpan { offset, length }
}

fn eof(offset: u64, needed: usize, remaining: usize) -> ZstdError {
    ZstdError::UnexpectedEof {
        offset,
        needed,
        remaining,
    }
}
