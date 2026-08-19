use zstdscope::{ByteSpan, FrameKind, StandardFrame, ZstdFile};

pub fn assert_model_invariants(input: &[u8], file: &ZstdFile) {
    assert_eq!(file.input_size, input.len() as u64);
    assert!(!file.frames.is_empty());

    let mut expected_frame_offset = 0_u64;
    for (frame_index, frame) in file.frames.iter().enumerate() {
        assert_eq!(frame.index, frame_index);
        assert_eq!(frame.span.offset, expected_frame_offset);
        let frame_end = assert_span_within(frame.span, file.input_size);

        match &frame.kind {
            FrameKind::Standard(standard) => {
                assert_standard_frame_invariants(standard, frame.span, file.input_size);
            }
            FrameKind::Skippable(skippable) => {
                assert_eq!(skippable.magic_span.offset, frame.span.offset);
                assert_eq!(skippable.magic_span.length, 4);
                assert_eq!(skippable.size_field_span.offset, frame.span.offset + 4);
                assert_eq!(skippable.size_field_span.length, 4);
                assert_eq!(skippable.payload_span.offset, frame.span.offset + 8);
                assert_eq!(
                    skippable.payload_span.length,
                    u64::from(skippable.declared_payload_size)
                );
                assert_eq!(assert_span_within(skippable.payload_span, file.input_size), frame_end);
                assert!(skippable.variant <= 15);
            }
        }

        expected_frame_offset = frame_end;
    }

    assert_eq!(expected_frame_offset, file.input_size);
}

fn assert_standard_frame_invariants(
    standard: &StandardFrame,
    frame_span: ByteSpan,
    input_size: u64,
) {
    let frame_end = frame_span.end().expect("frame span must not overflow");
    assert_eq!(standard.magic_span.offset, frame_span.offset);
    assert_eq!(standard.magic_span.length, 4);
    assert_eq!(standard.header.span.offset, frame_span.offset + 4);
    assert_eq!(standard.header.descriptor_span.offset, standard.header.span.offset);
    assert_eq!(standard.header.descriptor_span.length, 1);
    assert_span_within(standard.header.span, frame_end);
    assert_span_within(standard.header.descriptor_span, frame_end);

    if let Some(span) = standard.header.window_descriptor_span {
        assert_span_within(span, frame_end);
    }
    if let Some(field) = standard.header.frame_content_size {
        assert_span_within(field.span, frame_end);
    }
    if let Some(field) = standard.header.dictionary_id {
        assert_span_within(field.span, frame_end);
    }

    assert!(!standard.blocks.is_empty());
    assert!(standard.blocks.last().is_some_and(|block| block.is_last));
    assert!(standard.blocks[..standard.blocks.len() - 1]
        .iter()
        .all(|block| !block.is_last));

    let mut expected_block_offset = standard
        .header
        .span
        .end()
        .expect("header span must not overflow");
    for (block_index, block) in standard.blocks.iter().enumerate() {
        assert_eq!(block.index, block_index);
        assert_eq!(block.header_span.offset, expected_block_offset);
        assert_eq!(block.header_span.length, 3);
        let header_end = assert_span_within(block.header_span, frame_end);
        assert_eq!(header_end, block.content_span.offset);
        let content_end = assert_span_within(block.content_span, frame_end);
        assert_eq!(block.content_span.length, u64::from(block.encoded_content_size));
        expected_block_offset = content_end;
    }

    if let Some(checksum) = standard.content_checksum {
        assert_eq!(checksum.span.offset, expected_block_offset);
        assert_eq!(checksum.span.length, 4);
        expected_block_offset = assert_span_within(checksum.span, input_size);
    }

    assert_eq!(expected_block_offset, frame_end);
}

fn assert_span_within(span: ByteSpan, maximum_end: u64) -> u64 {
    let end = span.end().expect("span must not overflow");
    assert!(end <= maximum_end);
    end
}
