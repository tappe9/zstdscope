use std::io::{self, Write};

use zstdscope::{BlockType, FrameKind, ZstdFile};

pub fn render<W: Write>(writer: &mut W, file: &ZstdFile) -> io::Result<()> {
    for (position, frame) in file.frames.iter().enumerate() {
        if position != 0 {
            writeln!(writer)?;
        }

        match &frame.kind {
            FrameKind::Standard(standard) => {
                writeln!(
                    writer,
                    "Frame #{} Standard  offset={}  size={}",
                    frame.index, frame.span.offset, frame.span.length
                )?;
                writeln!(
                    writer,
                    "  Header  offset={}  size={}  descriptor=0x{:02X}  window_size={}  single_segment={}  checksum={}",
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
                )?;

                if let Some(frame_content_size) = &standard.header.frame_content_size {
                    writeln!(
                        writer,
                        "    Frame content size  value={}  offset={}  size={}",
                        frame_content_size.value,
                        frame_content_size.span.offset,
                        frame_content_size.span.length
                    )?;
                }

                if let Some(dictionary_id) = &standard.header.dictionary_id {
                    writeln!(
                        writer,
                        "    Dictionary ID  encoded={}  offset={}  size={}",
                        dictionary_id.encoded, dictionary_id.span.offset, dictionary_id.span.length
                    )?;
                }

                for block in &standard.blocks {
                    writeln!(
                        writer,
                        "  Block #{} {}  header_offset={}  content_offset={}  declared_size={}  encoded_size={}  last={}",
                        block.index,
                        block_type_name(block.block_type),
                        block.header_span.offset,
                        block.content_span.offset,
                        block.declared_size,
                        block.encoded_content_size,
                        block.is_last
                    )?;
                }

                if let Some(checksum) = &standard.content_checksum {
                    writeln!(
                        writer,
                        "  Content checksum  stored=0x{:08X}  offset={}  size={}  (not verified)",
                        checksum.value, checksum.span.offset, checksum.span.length
                    )?;
                }
            }
            FrameKind::Skippable(skippable) => {
                writeln!(
                    writer,
                    "Frame #{} Skippable  offset={}  size={}",
                    frame.index, frame.span.offset, frame.span.length
                )?;
                writeln!(
                    writer,
                    "  Magic  value=0x{:08X}  offset={}  size={}  variant={}",
                    skippable.magic,
                    skippable.magic_span.offset,
                    skippable.magic_span.length,
                    skippable.variant
                )?;
                writeln!(
                    writer,
                    "  Size field  offset={}  size={}  declared_payload_size={}",
                    skippable.size_field_span.offset,
                    skippable.size_field_span.length,
                    skippable.declared_payload_size
                )?;
                writeln!(
                    writer,
                    "  Payload  offset={}  size={}  payload_size={}",
                    skippable.payload_span.offset,
                    skippable.payload_span.length,
                    skippable.declared_payload_size
                )?;
            }
        }
    }

    Ok(())
}

fn block_type_name(block_type: BlockType) -> &'static str {
    match block_type {
        BlockType::Raw => "raw",
        BlockType::Rle => "rle",
        BlockType::Compressed => "compressed",
    }
}
