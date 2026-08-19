use zstdscope::{FrameKind, ZstdFile, inspect};

const STANDARD_MAGIC: u32 = 0xFD2F_B528;

#[test]
fn successful_parse_satisfies_structural_model_invariants() {
    let input = [minimal_standard_frame(), minimal_standard_frame()].concat();
    let file = inspect(&input).expect("fixture must parse");

    assert_model_invariants(&input, &file);
}

fn assert_model_invariants(input: &[u8], file: &ZstdFile) {
    assert_eq!(file.input_size, input.len() as u64);

    let mut expected_frame_offset = 0_u64;
    for (frame_index, frame) in file.frames.iter().enumerate() {
        assert_eq!(frame.index, frame_index);
        assert_eq!(frame.span.offset, expected_frame_offset);
        let frame_end = frame.span.end().expect("frame span must not overflow");
        assert!(frame_end <= file.input_size);

        if let FrameKind::Standard(standard) = &frame.kind {
            assert!(!standard.blocks.is_empty());
            assert!(standard.blocks.last().is_some_and(|block| block.is_last));

            for (block_index, block) in standard.blocks.iter().enumerate() {
                assert_eq!(block.index, block_index);
                let header_end = block
                    .header_span
                    .end()
                    .expect("block header span must not overflow");
                let content_end = block
                    .content_span
                    .end()
                    .expect("block content span must not overflow");
                assert_eq!(header_end, block.content_span.offset);
                assert!(block.header_span.offset >= frame.span.offset);
                assert!(content_end <= frame_end);
            }
        }

        expected_frame_offset = frame_end;
    }

    assert_eq!(expected_frame_offset, file.input_size);
}

fn minimal_standard_frame() -> Vec<u8> {
    let mut frame = STANDARD_MAGIC.to_le_bytes().to_vec();
    frame.extend_from_slice(&[0x00, 0x00]);
    frame.extend_from_slice(&[0x01, 0x00, 0x00]);
    frame
}
