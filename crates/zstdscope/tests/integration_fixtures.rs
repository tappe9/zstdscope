use zstdscope::{BlockType, FrameKind, ZstdError, inspect};

const REFERENCE_RAW_NO_CHECKSUM: &str =
    include_str!("fixtures/reference/raw-no-checksum.zst.hex");
const REFERENCE_COMPRESSED_CHECKSUM: &str =
    include_str!("fixtures/reference/compressed-checksum.zst.hex");
const HAND_BUILT: &str = include_str!("fixtures/hand-built.hex");

#[test]
fn reference_generated_frames_parse_with_exact_structure_and_spans() {
    let raw_input = decode_hex(REFERENCE_RAW_NO_CHECKSUM);
    let raw_file = inspect(&raw_input).expect("reference Raw frame must parse");

    assert_eq!(raw_file.input_size, 50);
    assert_eq!(raw_file.frames.len(), 1);
    let raw_frame = &raw_file.frames[0];
    assert_eq!(raw_frame.span.offset, 0);
    assert_eq!(raw_frame.span.length, 50);
    let FrameKind::Standard(raw) = &raw_frame.kind else {
        panic!("reference Raw fixture did not produce a Standard frame");
    };
    assert_eq!(raw.magic_span.offset, 0);
    assert_eq!(raw.magic_span.length, 4);
    assert_eq!(raw.header.span.offset, 4);
    assert_eq!(raw.header.span.length, 2);
    assert_eq!(raw.header.descriptor, 0x20);
    assert!(raw.header.single_segment);
    assert!(!raw.header.content_checksum_flag);
    assert_eq!(raw.header.window_size, 41);
    let raw_fcs = raw
        .header
        .frame_content_size
        .as_ref()
        .expect("reference Raw fixture must store Frame Content Size");
    assert_eq!(raw_fcs.value, 41);
    assert_eq!(raw_fcs.span.offset, 5);
    assert_eq!(raw_fcs.span.length, 1);
    assert_eq!(raw.blocks.len(), 1);
    assert_eq!(raw.blocks[0].block_type, BlockType::Raw);
    assert_eq!(raw.blocks[0].declared_size, 41);
    assert_eq!(raw.blocks[0].encoded_content_size, 41);
    assert_eq!(raw.blocks[0].header_span.offset, 6);
    assert_eq!(raw.blocks[0].header_span.length, 3);
    assert_eq!(raw.blocks[0].content_span.offset, 9);
    assert_eq!(raw.blocks[0].content_span.length, 41);
    assert!(raw.content_checksum.is_none());

    let compressed_input = decode_hex(REFERENCE_COMPRESSED_CHECKSUM);
    let compressed_file =
        inspect(&compressed_input).expect("reference Compressed frame must parse");

    assert_eq!(compressed_file.input_size, 63);
    assert_eq!(compressed_file.frames.len(), 1);
    let compressed_frame = &compressed_file.frames[0];
    assert_eq!(compressed_frame.span.offset, 0);
    assert_eq!(compressed_frame.span.length, 63);
    let FrameKind::Standard(compressed) = &compressed_frame.kind else {
        panic!("reference Compressed fixture did not produce a Standard frame");
    };
    assert_eq!(compressed.header.span.offset, 4);
    assert_eq!(compressed.header.span.length, 3);
    assert_eq!(compressed.header.descriptor, 0x64);
    assert!(compressed.header.single_segment);
    assert!(compressed.header.content_checksum_flag);
    assert_eq!(compressed.header.window_size, 10_240);
    let compressed_fcs = compressed
        .header
        .frame_content_size
        .as_ref()
        .expect("reference Compressed fixture must store Frame Content Size");
    assert_eq!(compressed_fcs.value, 10_240);
    assert_eq!(compressed_fcs.span.offset, 5);
    assert_eq!(compressed_fcs.span.length, 2);
    assert_eq!(compressed.blocks.len(), 1);
    assert_eq!(compressed.blocks[0].block_type, BlockType::Compressed);
    assert_eq!(compressed.blocks[0].declared_size, 49);
    assert_eq!(compressed.blocks[0].encoded_content_size, 49);
    assert_eq!(compressed.blocks[0].header_span.offset, 7);
    assert_eq!(compressed.blocks[0].header_span.length, 3);
    assert_eq!(compressed.blocks[0].content_span.offset, 10);
    assert_eq!(compressed.blocks[0].content_span.length, 49);
    let checksum = compressed
        .content_checksum
        .as_ref()
        .expect("reference Compressed fixture must store a checksum");
    assert_eq!(checksum.value, 0xC260_8365);
    assert_eq!(checksum.span.offset, 59);
    assert_eq!(checksum.span.length, 4);
}

#[test]
fn hand_built_dictionary_id_widths_preserve_encoded_fidelity() {
    let cases = [
        ("dict_id_1_explicit_zero", 0_u32, 1_u64, 3_u64),
        ("dict_id_2", 0x1234_u32, 2_u64, 4_u64),
        ("dict_id_4", 0x1234_5678_u32, 4_u64, 6_u64),
    ];

    for (name, encoded, width, header_length) in cases {
        let input = hand_fixture(name);
        let file = inspect(&input).expect("Dictionary ID fixture must parse");
        let FrameKind::Standard(frame) = &file.frames[0].kind else {
            panic!("{name} did not produce a Standard frame");
        };
        let dictionary_id = frame
            .header
            .dictionary_id
            .as_ref()
            .expect("Dictionary ID field must be present");

        assert_eq!(dictionary_id.encoded, encoded, "{name}");
        assert_eq!(dictionary_id.span.offset, 6, "{name}");
        assert_eq!(dictionary_id.span.length, width, "{name}");
        assert_eq!(frame.header.span.offset, 4, "{name}");
        assert_eq!(frame.header.span.length, header_length, "{name}");
        assert_eq!(
            file.frames[0].span.length,
            u64::try_from(input.len()).expect("fixture length fits u64"),
            "{name}"
        );
    }

    let absent_input = hand_fixture("minimal_standard");
    let absent_file = inspect(&absent_input).expect("minimal Standard frame must parse");
    let FrameKind::Standard(absent) = &absent_file.frames[0].kind else {
        panic!("minimal fixture did not produce a Standard frame");
    };
    assert!(absent.header.dictionary_id.is_none());
}

#[test]
fn hand_built_frame_content_size_widths_cover_single_and_non_single_segment() {
    let absent_input = hand_fixture("minimal_standard");
    let absent_file = inspect(&absent_input).expect("FCS-absent fixture must parse");
    let FrameKind::Standard(absent) = &absent_file.frames[0].kind else {
        panic!("FCS-absent fixture did not produce a Standard frame");
    };
    assert!(!absent.header.single_segment);
    assert!(absent.header.frame_content_size.is_none());
    let window_span = absent
        .header
        .window_descriptor_span
        .as_ref()
        .expect("non-Single Segment frame must encode a Window Descriptor");
    assert_eq!(window_span.offset, 5);
    assert_eq!(window_span.length, 1);
    assert_eq!(absent.header.window_size, 1024);

    let cases = [
        ("fcs_1_single", 0_u64, 5_u64, 1_u64, 2_u64, true, 0_u64),
        ("fcs_2", 256_u64, 6_u64, 2_u64, 4_u64, false, 1024_u64),
        ("fcs_4", 256_u64, 6_u64, 4_u64, 6_u64, false, 1024_u64),
        ("fcs_8", 256_u64, 6_u64, 8_u64, 10_u64, false, 1024_u64),
    ];

    for (name, value, span_offset, span_length, header_length, single, window_size) in cases {
        let input = hand_fixture(name);
        let file = inspect(&input).expect("Frame Content Size fixture must parse");
        let FrameKind::Standard(frame) = &file.frames[0].kind else {
            panic!("{name} did not produce a Standard frame");
        };
        let frame_content_size = frame
            .header
            .frame_content_size
            .as_ref()
            .expect("Frame Content Size must be present");

        assert_eq!(frame.header.single_segment, single, "{name}");
        assert_eq!(frame.header.window_size, window_size, "{name}");
        assert_eq!(frame_content_size.value, value, "{name}");
        assert_eq!(frame_content_size.span.offset, span_offset, "{name}");
        assert_eq!(frame_content_size.span.length, span_length, "{name}");
        assert_eq!(frame.header.span.offset, 4, "{name}");
        assert_eq!(frame.header.span.length, header_length, "{name}");
        if single {
            assert!(frame.header.window_descriptor_span.is_none(), "{name}");
        } else {
            let span = frame
                .header
                .window_descriptor_span
                .as_ref()
                .expect("non-Single Segment frame must encode a Window Descriptor");
            assert_eq!(span.offset, 5, "{name}");
            assert_eq!(span.length, 1, "{name}");
        }
    }
}

#[test]
fn hand_built_block_cases_preserve_encoded_size_semantics_and_spans() {
    let cases = [
        ("raw_block", BlockType::Raw, 2_u32, 2_u32, 2_u64),
        ("rle_block", BlockType::Rle, 17_u32, 1_u32, 1_u64),
        (
            "compressed_block_opaque",
            BlockType::Compressed,
            3_u32,
            3_u32,
            3_u64,
        ),
    ];

    for (name, block_type, declared_size, encoded_content_size, content_length) in cases {
        let input = hand_fixture(name);
        let file = inspect(&input).expect("block fixture must parse structurally");
        let FrameKind::Standard(frame) = &file.frames[0].kind else {
            panic!("{name} did not produce a Standard frame");
        };
        let block = &frame.blocks[0];

        assert_eq!(block.block_type, block_type, "{name}");
        assert_eq!(block.declared_size, declared_size, "{name}");
        assert_eq!(block.encoded_content_size, encoded_content_size, "{name}");
        assert_eq!(block.header_span.offset, 6, "{name}");
        assert_eq!(block.header_span.length, 3, "{name}");
        assert_eq!(block.content_span.offset, 9, "{name}");
        assert_eq!(block.content_span.length, content_length, "{name}");
        assert!(block.is_last, "{name}");
    }

    let multiple_input = hand_fixture("multiple_blocks");
    let multiple_file = inspect(&multiple_input).expect("multiple-block fixture must parse");
    let FrameKind::Standard(multiple) = &multiple_file.frames[0].kind else {
        panic!("multiple-block fixture did not produce a Standard frame");
    };
    assert_eq!(multiple.blocks.len(), 2);
    assert_eq!(multiple.blocks[0].block_type, BlockType::Rle);
    assert_eq!(multiple.blocks[0].header_span.offset, 6);
    assert_eq!(multiple.blocks[0].content_span.offset, 9);
    assert_eq!(multiple.blocks[0].content_span.length, 1);
    assert!(!multiple.blocks[0].is_last);
    assert_eq!(multiple.blocks[1].block_type, BlockType::Raw);
    assert_eq!(multiple.blocks[1].header_span.offset, 10);
    assert_eq!(multiple.blocks[1].content_span.offset, 13);
    assert_eq!(multiple.blocks[1].content_span.length, 2);
    assert!(multiple.blocks[1].is_last);
    assert_eq!(multiple_file.frames[0].span.offset, 0);
    assert_eq!(multiple_file.frames[0].span.length, 15);
}

#[test]
fn checksum_and_mixed_stream_fixtures_preserve_exact_boundaries() {
    let checksum_input = hand_fixture("checksum");
    let checksum_file = inspect(&checksum_input).expect("checksum fixture must parse");
    let FrameKind::Standard(checksum_frame) = &checksum_file.frames[0].kind else {
        panic!("checksum fixture did not produce a Standard frame");
    };
    let checksum = checksum_frame
        .content_checksum
        .as_ref()
        .expect("stored checksum must be exposed");
    assert_eq!(checksum.value, 0xDEAD_BEEF);
    assert_eq!(checksum.span.offset, 9);
    assert_eq!(checksum.span.length, 4);
    assert_eq!(checksum_file.frames[0].span.offset, 0);
    assert_eq!(checksum_file.frames[0].span.length, 13);

    let mixed_input = hand_fixture("mixed_standard_skippable_standard");
    let mixed_file = inspect(&mixed_input).expect("mixed stream fixture must parse");
    assert_eq!(mixed_file.frames.len(), 3);
    assert_eq!(mixed_file.frames[0].index, 0);
    assert_eq!(mixed_file.frames[0].span.offset, 0);
    assert_eq!(mixed_file.frames[0].span.length, 9);
    assert_eq!(mixed_file.frames[1].index, 1);
    assert_eq!(mixed_file.frames[1].span.offset, 9);
    assert_eq!(mixed_file.frames[1].span.length, 10);
    let FrameKind::Skippable(skippable) = &mixed_file.frames[1].kind else {
        panic!("middle mixed-stream frame was not Skippable");
    };
    assert_eq!(skippable.magic_span.offset, 9);
    assert_eq!(skippable.magic_span.length, 4);
    assert_eq!(skippable.size_field_span.offset, 13);
    assert_eq!(skippable.size_field_span.length, 4);
    assert_eq!(skippable.payload_span.offset, 17);
    assert_eq!(skippable.payload_span.length, 2);
    assert_eq!(mixed_file.frames[2].index, 2);
    assert_eq!(mixed_file.frames[2].span.offset, 19);
    assert_eq!(mixed_file.frames[2].span.length, 9);
}

#[test]
fn malformed_fixture_matrix_returns_typed_location_aware_errors() {
    let max_payload_needed = usize::try_from(u32::MAX).expect("u32 must fit usize on supported targets");
    let cases = vec![
        (
            "empty",
            ZstdError::UnexpectedEof {
                offset: 0,
                needed: 4,
                remaining: 0,
            },
        ),
        (
            "invalid_magic",
            ZstdError::InvalidMagic {
                offset: 0,
                magic: 0x1234_5678,
            },
        ),
        (
            "truncated_frame_header",
            ZstdError::UnexpectedEof {
                offset: 4,
                needed: 1,
                remaining: 0,
            },
        ),
        (
            "reserved_frame_header_bit",
            ZstdError::ReservedFrameHeaderBit { offset: 4 },
        ),
        (
            "truncated_block_header",
            ZstdError::UnexpectedEof {
                offset: 6,
                needed: 3,
                remaining: 2,
            },
        ),
        (
            "truncated_block_content",
            ZstdError::UnexpectedEof {
                offset: 9,
                needed: 2,
                remaining: 1,
            },
        ),
        (
            "reserved_block_type",
            ZstdError::ReservedBlockType { offset: 6 },
        ),
        (
            "truncated_checksum",
            ZstdError::UnexpectedEof {
                offset: 9,
                needed: 4,
                remaining: 2,
            },
        ),
        (
            "truncated_skippable_size",
            ZstdError::UnexpectedEof {
                offset: 4,
                needed: 4,
                remaining: 2,
            },
        ),
        (
            "truncated_skippable_payload",
            ZstdError::UnexpectedEof {
                offset: 8,
                needed: 4,
                remaining: 2,
            },
        ),
        (
            "trailing_partial_magic",
            ZstdError::UnexpectedEof {
                offset: 9,
                needed: 4,
                remaining: 2,
            },
        ),
        (
            "invalid_block_size",
            ZstdError::InvalidBlockSize {
                offset: 6,
                size: 1025,
                maximum: 1024,
            },
        ),
        (
            "max_skippable_declared_size",
            ZstdError::UnexpectedEof {
                offset: 8,
                needed: max_payload_needed,
                remaining: 0,
            },
        ),
    ];

    for (name, expected) in cases {
        let input = hand_fixture(name);
        assert_eq!(inspect(&input).unwrap_err(), expected, "{name}");
    }
}

fn hand_fixture(name: &str) -> Vec<u8> {
    let prefix = format!("{name}=");
    let hex = HAND_BUILT
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("missing hand-built fixture: {name}"));
    decode_hex(hex)
}

fn decode_hex(hex: &str) -> Vec<u8> {
    let compact = hex
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    let mut chunks = compact.chunks_exact(2);
    let mut bytes = Vec::with_capacity(compact.len() / 2);

    for pair in &mut chunks {
        let pair = std::str::from_utf8(pair).expect("fixture hex must be ASCII");
        bytes.push(u8::from_str_radix(pair, 16).expect("fixture must contain valid hexadecimal"));
    }

    assert!(
        chunks.remainder().is_empty(),
        "fixture hex must contain an even number of digits"
    );
    bytes
}
