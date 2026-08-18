use zstdscope::{BlockType, ByteSpan, FrameKind, StandardFrame, ZstdError, inspect};

const REFERENCE_RAW_NO_CHECKSUM: &str = include_str!("fixtures/reference/raw-no-checksum.zst.hex");
const REFERENCE_COMPRESSED_CHECKSUM: &str =
    include_str!("fixtures/reference/compressed-checksum.zst.hex");
const HAND_BUILT: &str = include_str!("fixtures/hand-built.hex");

#[test]
fn reference_generated_frames_parse_with_exact_structure_and_spans() {
    let raw_input = decode_hex(REFERENCE_RAW_NO_CHECKSUM);
    let (raw_span, raw) = inspect_single_standard(&raw_input);
    assert_span(&raw_span, 0, 50);
    assert_span(&raw.magic_span, 0, 4);
    assert_span(&raw.header.span, 4, 2);
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
    assert_span(&raw_fcs.span, 5, 1);
    assert_eq!(raw.blocks.len(), 1);
    assert_eq!(raw.blocks[0].block_type, BlockType::Raw);
    assert_eq!(raw.blocks[0].declared_size, 41);
    assert_eq!(raw.blocks[0].encoded_content_size, 41);
    assert_span(&raw.blocks[0].header_span, 6, 3);
    assert_span(&raw.blocks[0].content_span, 9, 41);
    assert!(raw.content_checksum.is_none());

    let compressed_input = decode_hex(REFERENCE_COMPRESSED_CHECKSUM);
    let (compressed_span, compressed) = inspect_single_standard(&compressed_input);
    assert_span(&compressed_span, 0, 63);
    assert_span(&compressed.header.span, 4, 3);
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
    assert_span(&compressed_fcs.span, 5, 2);
    assert_eq!(compressed.blocks.len(), 1);
    assert_eq!(compressed.blocks[0].block_type, BlockType::Compressed);
    assert_eq!(compressed.blocks[0].declared_size, 49);
    assert_eq!(compressed.blocks[0].encoded_content_size, 49);
    assert_span(&compressed.blocks[0].header_span, 7, 3);
    assert_span(&compressed.blocks[0].content_span, 10, 49);
    let checksum = compressed
        .content_checksum
        .as_ref()
        .expect("reference Compressed fixture must store a checksum");
    assert_eq!(checksum.value, 0x3BC2_6083);
    assert_span(&checksum.span, 59, 4);
}

#[test]
fn hand_built_header_widths_preserve_inspector_fidelity() {
    let dictionary_cases = [
        ("dict_id_1_explicit_zero", 0_u32, 1_u64, 3_u64),
        ("dict_id_2", 0x1234_u32, 2_u64, 4_u64),
        ("dict_id_4", 0x1234_5678_u32, 4_u64, 6_u64),
    ];

    for (name, encoded, width, header_length) in dictionary_cases {
        let (_, frame) = inspect_single_standard(&hand_fixture(name));
        let dictionary_id = frame
            .header
            .dictionary_id
            .as_ref()
            .expect("Dictionary ID field must be present");
        assert_eq!(dictionary_id.encoded, encoded, "{name}");
        assert_span(&dictionary_id.span, 6, width);
        assert_span(&frame.header.span, 4, header_length);
    }

    let (_, absent) = inspect_single_standard(&hand_fixture("minimal_standard"));
    assert!(absent.header.dictionary_id.is_none());
    assert!(absent.header.frame_content_size.is_none());
    assert!(!absent.header.single_segment);
    assert_eq!(absent.header.window_size, 1024);
    assert_span(
        absent
            .header
            .window_descriptor_span
            .as_ref()
            .expect("non-Single Segment frame must encode a Window Descriptor"),
        5,
        1,
    );

    let fcs_cases = [
        ("fcs_1_single", 0_u64, 5_u64, 1_u64, 2_u64, true, 0_u64),
        ("fcs_2", 256_u64, 6_u64, 2_u64, 4_u64, false, 1024_u64),
        ("fcs_4", 256_u64, 6_u64, 4_u64, 6_u64, false, 1024_u64),
        ("fcs_8", 256_u64, 6_u64, 8_u64, 10_u64, false, 1024_u64),
    ];

    for (name, value, offset, length, header_length, single, window_size) in fcs_cases {
        let (_, frame) = inspect_single_standard(&hand_fixture(name));
        let fcs = frame
            .header
            .frame_content_size
            .as_ref()
            .expect("Frame Content Size must be present");
        assert_eq!(frame.header.single_segment, single, "{name}");
        assert_eq!(frame.header.window_size, window_size, "{name}");
        assert_eq!(fcs.value, value, "{name}");
        assert_span(&fcs.span, offset, length);
        assert_span(&frame.header.span, 4, header_length);
        assert_eq!(frame.header.window_descriptor_span.is_none(), single, "{name}");
    }
}

#[test]
fn hand_built_blocks_preserve_encoded_size_semantics_and_spans() {
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

    for (name, block_type, declared_size, encoded_size, content_length) in cases {
        let (_, frame) = inspect_single_standard(&hand_fixture(name));
        let block = &frame.blocks[0];
        assert_eq!(block.block_type, block_type, "{name}");
        assert_eq!(block.declared_size, declared_size, "{name}");
        assert_eq!(block.encoded_content_size, encoded_size, "{name}");
        assert_span(&block.header_span, 6, 3);
        assert_span(&block.content_span, 9, content_length);
        assert!(block.is_last, "{name}");
    }

    let (span, frame) = inspect_single_standard(&hand_fixture("multiple_blocks"));
    assert_span(&span, 0, 15);
    assert_eq!(frame.blocks.len(), 2);
    assert_eq!(frame.blocks[0].block_type, BlockType::Rle);
    assert_span(&frame.blocks[0].header_span, 6, 3);
    assert_span(&frame.blocks[0].content_span, 9, 1);
    assert!(!frame.blocks[0].is_last);
    assert_eq!(frame.blocks[1].block_type, BlockType::Raw);
    assert_span(&frame.blocks[1].header_span, 10, 3);
    assert_span(&frame.blocks[1].content_span, 13, 2);
    assert!(frame.blocks[1].is_last);
}

#[test]
fn checksum_and_mixed_stream_fixtures_preserve_exact_boundaries() {
    let (checksum_span, checksum_frame) = inspect_single_standard(&hand_fixture("checksum"));
    assert_span(&checksum_span, 0, 13);
    let checksum = checksum_frame
        .content_checksum
        .as_ref()
        .expect("stored checksum must be exposed");
    assert_eq!(checksum.value, 0xDEAD_BEEF);
    assert_span(&checksum.span, 9, 4);

    let input = hand_fixture("mixed_standard_skippable_standard");
    let file = inspect(&input).expect("mixed stream fixture must parse");
    assert_eq!(file.frames.len(), 3);
    assert_eq!(file.frames[0].index, 0);
    assert_span(&file.frames[0].span, 0, 9);
    assert_eq!(file.frames[1].index, 1);
    assert_span(&file.frames[1].span, 9, 10);
    let FrameKind::Skippable(skippable) = &file.frames[1].kind else {
        panic!("middle mixed-stream frame was not Skippable");
    };
    assert_span(&skippable.magic_span, 9, 4);
    assert_span(&skippable.size_field_span, 13, 4);
    assert_span(&skippable.payload_span, 17, 2);
    assert_eq!(file.frames[2].index, 2);
    assert_span(&file.frames[2].span, 19, 9);
}

#[test]
fn malformed_fixture_matrix_returns_typed_location_aware_errors() {
    let max_payload_needed =
        usize::try_from(u32::MAX).expect("u32 must fit usize on supported targets");
    let cases = [
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

fn inspect_single_standard(input: &[u8]) -> (ByteSpan, StandardFrame) {
    let file = inspect(input).expect("single-frame fixture must parse");
    assert_eq!(
        file.input_size,
        u64::try_from(input.len()).expect("length fits u64")
    );
    assert_eq!(file.frames.len(), 1);
    let frame = file
        .frames
        .into_iter()
        .next()
        .expect("single-frame fixture must produce one frame");
    let FrameKind::Standard(standard) = frame.kind else {
        panic!("fixture did not produce a Standard frame");
    };
    (frame.span, standard)
}

fn assert_span(span: &ByteSpan, offset: u64, length: u64) {
    assert_eq!(*span, ByteSpan { offset, length });
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
