#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSpan {
    pub offset: u64,
    pub length: u64,
}

impl ByteSpan {
    pub fn end(&self) -> Option<u64> {
        self.offset.checked_add(self.length)
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZstdFile {
    pub input_size: u64,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub index: usize,
    pub span: ByteSpan,
    pub kind: FrameKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameKind {
    Standard(StandardFrame),
    Skippable(SkippableFrame),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardFrame {
    pub magic_span: ByteSpan,
    pub header: FrameHeader,
    pub blocks: Vec<Block>,
    pub content_checksum: Option<ContentChecksum>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    pub span: ByteSpan,
    pub descriptor: u8,
    pub descriptor_span: ByteSpan,
    pub window_descriptor_span: Option<ByteSpan>,
    pub frame_content_size: Option<FrameContentSize>,
    pub dictionary_id: Option<DictionaryId>,
    pub window_size: u64,
    pub content_checksum_flag: bool,
    pub single_segment: bool,
    pub unused_bit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameContentSize {
    pub value: u64,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DictionaryId {
    pub encoded: u32,
    pub span: ByteSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub index: usize,
    pub header_span: ByteSpan,
    pub content_span: ByteSpan,
    pub block_type: BlockType,
    pub declared_size: u32,
    pub encoded_content_size: u32,
    pub is_last: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Raw,
    Rle,
    Compressed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentChecksum {
    pub span: ByteSpan,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippableFrame {
    pub magic_span: ByteSpan,
    pub magic: u32,
    pub variant: u8,
    pub size_field_span: ByteSpan,
    pub declared_payload_size: u32,
    pub payload_span: ByteSpan,
}
