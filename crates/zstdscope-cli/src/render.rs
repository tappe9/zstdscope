use zstdscope::{BlockType, FrameKind, ZstdFile};

pub fn render(file: &ZstdFile) -> String {
    let mut output = String::new();

    for (position, frame) in file.frames.iter().enumerate() {
        if position != 0 {
            output.push('\n');
        }

        match &frame.kind {
            FrameKind::Standard(standard) => {
                output.push_str(&format!(
                    "Frame #{} Standard  offset={}  size={}\n",
                    frame.index, frame.span.offset, frame.span.length
                ));
                output.push_str(&format!(
                    "  Header  offset={}  size={}  descriptor=0x{:02X}  window_size={}  single_segment={}  checksum={}\n",
                    standard.header.span.offset,
                    standard.header.span.length,
                    standard.header.descriptor,
                    standard.header.window_size,
                    standard.header.single_segment,
                    if standard.header.content_checksum_flag {
                        "present"
                    } else {
                        "absent"
                    }
                ));

                if let Some(frame_content_size) = &standard.header.frame_content_size {
                    output.push_str(&format!(
                        "    Frame content size  value={}  offset={}  size={}\n",
                        frame_content_size.value,
                        frame_content_size.span.offset,
                        frame_content_size.span.length
                    ));
                }

                if let Some(dictionary_id) = &standard.header.dictionary_id {
                    output.push_str(&format!(
                        "    Dictionary ID  encoded={}  offset={}  size={}\n",
                        dictionary_id.encoded,
                        dictionary_id.span.offset,
                        dictionary_id.span.length
                    ));
                }

                for block in &standard.blocks {
                    output.push_str(&format!(
                        "  Block #{} {}  header_offset={}  content_offset={}  declared_size={}  encoded_size={}  last={}\n",
                        block.index,
                        block_type_name(block.block_type),
                        block.header_span.offset,
                        block.content_span.offset,
                        block.declared_size,
                        block.encoded_content_size,
                        block.is_last
                    ));
                }

                if let Some(checksum) = &standard.content_checksum {
                    output.push_str(&format!(
                        "  Content checksum  stored=0x{:08X}  offset={}  size={}  (not verified)\n",
                        checksum.value, checksum.span.offset, checksum.span.length
                    ));
                }
            }
            FrameKind::Skippable(skippable) => {
                output.push_str(&format!(
                    "Frame #{} Skippable  offset={}  size={}\n",
                    frame.index, frame.span.offset, frame.span.length
                ));
                output.push_str(&format!(
                    "  Magic  value=0x{:08X}  offset={}  size={}  variant={}\n",
                    skippable.magic,
                    skippable.magic_span.offset,
                    skippable.magic_span.length,
                    skippable.variant
                ));
                output.push_str(&format!(
                    "  Size field  offset={}  size={}  declared_payload_size={}\n",
                    skippable.size_field_span.offset,
                    skippable.size_field_span.length,
                    skippable.declared_payload_size
                ));
                output.push_str(&format!(
                    "  Payload  offset={}  size={}  payload_size={}\n",
                    skippable.payload_span.offset,
                    skippable.payload_span.length,
                    skippable.declared_payload_size
                ));
            }
        }
    }

    output
}

fn block_type_name(block_type: BlockType) -> &'static str {
    match block_type {
        BlockType::Raw => "raw",
        BlockType::Rle => "rle",
        BlockType::Compressed => "compressed",
    }
}
