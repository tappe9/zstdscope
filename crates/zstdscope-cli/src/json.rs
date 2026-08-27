use std::io::Write;

use serde::Serialize;
use zstdscope::{
    Block, BlockType, ByteSpan, ContentChecksum, DictionaryId, Frame, FrameContentSize,
    FrameHeader, FrameKind, SkippableFrame, StandardFrame, ZstdFile,
};

const SCHEMA_VERSION: u32 = 1;

pub(super) fn write<W: Write>(writer: &mut W, file: &ZstdFile) -> Result<(), serde_json::Error> {
    serde_json::to_writer_pretty(writer, &JsonZstdFileV1::from(file))
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct JsonZstdFileV1 {
    schema_version: u32,
    input_size: String,
    frames: Vec<JsonFrameV1>,
}

impl From<&ZstdFile> for JsonZstdFileV1 {
    fn from(source: &ZstdFile) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            input_size: source.input_size.to_string(),
            frames: source.frames.iter().map(JsonFrameV1::from).collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct JsonFrameV1 {
    index: usize,
    span: JsonByteSpanV1,
    kind: JsonFrameKindV1,
}

impl From<&Frame> for JsonFrameV1 {
    fn from(source: &Frame) -> Self {
        Self {
            index: source.index,
            span: source.span.into(),
            kind: (&source.kind).into(),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum JsonFrameKindV1 {
    Standard(JsonStandardFrameV1),
    Skippable(JsonSkippableFrameV1),
}

impl From<&FrameKind> for JsonFrameKindV1 {
    fn from(source: &FrameKind) -> Self {
        match source {
            FrameKind::Standard(frame) => Self::Standard(frame.into()),
            FrameKind::Skippable(frame) => Self::Skippable(frame.into()),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct JsonStandardFrameV1 {
    magic_span: JsonByteSpanV1,
    header: JsonFrameHeaderV1,
    blocks: Vec<JsonBlockV1>,
    content_checksum: Option<JsonContentChecksumV1>,
}

impl From<&StandardFrame> for JsonStandardFrameV1 {
    fn from(source: &StandardFrame) -> Self {
        Self {
            magic_span: source.magic_span.into(),
            header: (&source.header).into(),
            blocks: source.blocks.iter().map(JsonBlockV1::from).collect(),
            content_checksum: source
                .content_checksum
                .as_ref()
                .map(JsonContentChecksumV1::from),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct JsonFrameHeaderV1 {
    span: JsonByteSpanV1,
    descriptor: u8,
    descriptor_span: JsonByteSpanV1,
    window_descriptor_span: Option<JsonByteSpanV1>,
    frame_content_size: Option<JsonFrameContentSizeV1>,
    dictionary_id: Option<JsonDictionaryIdV1>,
    window_size: String,
    content_checksum_flag: bool,
    single_segment: bool,
    unused_bit: bool,
}

impl From<&FrameHeader> for JsonFrameHeaderV1 {
    fn from(source: &FrameHeader) -> Self {
        Self {
            span: source.span.into(),
            descriptor: source.descriptor,
            descriptor_span: source.descriptor_span.into(),
            window_descriptor_span: source.window_descriptor_span.map(JsonByteSpanV1::from),
            frame_content_size: source
                .frame_content_size
                .as_ref()
                .map(JsonFrameContentSizeV1::from),
            dictionary_id: source
                .dictionary_id
                .as_ref()
                .map(JsonDictionaryIdV1::from),
            window_size: source.window_size.to_string(),
            content_checksum_flag: source.content_checksum_flag,
            single_segment: source.single_segment,
            unused_bit: source.unused_bit,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct JsonFrameContentSizeV1 {
    value: String,
    span: JsonByteSpanV1,
}

impl From<&FrameContentSize> for JsonFrameContentSizeV1 {
    fn from(source: &FrameContentSize) -> Self {
        Self {
            value: source.value.to_string(),
            span: source.span.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct JsonDictionaryIdV1 {
    encoded: u32,
    span: JsonByteSpanV1,
}

impl From<&DictionaryId> for JsonDictionaryIdV1 {
    fn from(source: &DictionaryId) -> Self {
        Self {
            encoded: source.encoded,
            span: source.span.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct JsonBlockV1 {
    index: usize,
    header_span: JsonByteSpanV1,
    content_span: JsonByteSpanV1,
    block_type: JsonBlockTypeV1,
    declared_size: u32,
    encoded_content_size: u32,
    is_last: bool,
}

impl From<&Block> for JsonBlockV1 {
    fn from(source: &Block) -> Self {
        Self {
            index: source.index,
            header_span: source.header_span.into(),
            content_span: source.content_span.into(),
            block_type: source.block_type.into(),
            declared_size: source.declared_size,
            encoded_content_size: source.encoded_content_size,
            is_last: source.is_last,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum JsonBlockTypeV1 {
    Raw,
    Rle,
    Compressed,
}

impl From<BlockType> for JsonBlockTypeV1 {
    fn from(source: BlockType) -> Self {
        match source {
            BlockType::Raw => Self::Raw,
            BlockType::Rle => Self::Rle,
            BlockType::Compressed => Self::Compressed,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct JsonContentChecksumV1 {
    span: JsonByteSpanV1,
    value: u32,
}

impl From<&ContentChecksum> for JsonContentChecksumV1 {
    fn from(source: &ContentChecksum) -> Self {
        Self {
            span: source.span.into(),
            value: source.value,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct JsonSkippableFrameV1 {
    magic_span: JsonByteSpanV1,
    magic: u32,
    variant: u8,
    size_field_span: JsonByteSpanV1,
    declared_payload_size: u32,
    payload_span: JsonByteSpanV1,
}

impl From<&SkippableFrame> for JsonSkippableFrameV1 {
    fn from(source: &SkippableFrame) -> Self {
        Self {
            magic_span: source.magic_span.into(),
            magic: source.magic,
            variant: source.variant,
            size_field_span: source.size_field_span.into(),
            declared_payload_size: source.declared_payload_size,
            payload_span: source.payload_span.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct JsonByteSpanV1 {
    offset: String,
    length: String,
}

impl From<ByteSpan> for JsonByteSpanV1 {
    fn from(source: ByteSpan) -> Self {
        Self {
            offset: source.offset.to_string(),
            length: source.length.to_string(),
        }
    }
}
