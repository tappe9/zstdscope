/// A half-open span of bytes in the original encoded input.
///
/// `offset` is zero-based from the beginning of the slice passed to
/// [`crate::inspect`]. The covered range is `[offset, offset + length)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct ByteSpan {
    /// Zero-based byte offset from the beginning of the inspected input.
    pub offset: u64,
    /// Number of encoded bytes covered by the span.
    pub length: u64,
}

impl ByteSpan {
    /// Returns the exclusive end offset, or `None` if the addition overflows.
    pub fn end(&self) -> Option<u64> {
        self.offset.checked_add(self.length)
    }

    /// Returns `true` when the span contains no bytes.
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

/// Structural inspection result for the complete input slice.
///
/// A successful value contains every top-level frame in input order. The
/// strict parser requires at least one complete frame and consumes the entire
/// input.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct ZstdFile {
    /// Total encoded input size in bytes.
    pub input_size: u64,
    /// Top-level frames in encoded order.
    pub frames: Vec<Frame>,
}

/// One top-level frame in a Zstandard stream.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct Frame {
    /// Zero-based frame index within [`ZstdFile::frames`].
    pub index: usize,
    /// Span of the complete encoded frame, including magic and any checksum.
    pub span: ByteSpan,
    /// Parsed Standard- or Skippable-Frame metadata.
    pub kind: FrameKind,
}

/// Supported top-level Zstandard frame kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "type", content = "data", rename_all = "snake_case")
)]
pub enum FrameKind {
    /// A standard Zstandard frame.
    Standard(StandardFrame),
    /// A skippable frame in the standard 16-value magic range.
    Skippable(SkippableFrame),
}

/// Structural metadata for a standard Zstandard frame.
///
/// Block payloads remain opaque; compressed block internals are not decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct StandardFrame {
    /// Span of the four-byte Standard Frame magic number.
    pub magic_span: ByteSpan,
    /// Parsed Frame Header metadata.
    pub header: FrameHeader,
    /// Blocks in encoded order, through the block with `is_last == true`.
    pub blocks: Vec<Block>,
    /// Stored content checksum metadata when the header checksum flag is set.
    ///
    /// ZstdScope does not decompress content in v0.1, so this checksum is not
    /// verified against decoded data.
    pub content_checksum: Option<ContentChecksum>,
}

/// Parsed fields and derived values from a Standard Frame Header.
///
/// The aggregate [`FrameHeader::span`] starts at the descriptor byte and does
/// not include the four-byte frame magic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct FrameHeader {
    /// Span of the complete encoded Frame Header after the magic number.
    pub span: ByteSpan,
    /// Raw Frame Header Descriptor byte.
    pub descriptor: u8,
    /// Span of the one-byte Frame Header Descriptor.
    pub descriptor_span: ByteSpan,
    /// Span of the Window Descriptor when physically encoded.
    ///
    /// This is `None` for Single Segment frames, where the Window Descriptor is
    /// absent.
    pub window_descriptor_span: Option<ByteSpan>,
    /// Decoded Frame Content Size and its source span when encoded.
    ///
    /// `None` means the size field is absent and the decompressed content size
    /// is unknown; it does not mean zero.
    pub frame_content_size: Option<FrameContentSize>,
    /// Encoded Dictionary ID field when physically present.
    ///
    /// `None` means the field is absent. `Some(DictionaryId { encoded: 0, .. })`
    /// means zero was explicitly encoded. ZstdScope preserves this byte-level
    /// distinction for inspection purposes.
    pub dictionary_id: Option<DictionaryId>,
    /// Effective window size in bytes derived from the encoded header.
    ///
    /// For Single Segment frames this is derived from Frame Content Size;
    /// otherwise it is derived from the Window Descriptor.
    pub window_size: u64,
    /// Whether the descriptor declares a stored 32-bit content checksum.
    pub content_checksum_flag: bool,
    /// Whether the frame uses the Single Segment header form.
    pub single_segment: bool,
    /// Value of the descriptor bit currently marked unused by the format.
    ///
    /// The bit is preserved as encoded and is not assigned new semantics.
    pub unused_bit: bool,
}

/// A decoded Frame Content Size together with the bytes that encoded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct FrameContentSize {
    /// Decoded decompressed-content size in bytes.
    ///
    /// The Zstandard two-byte encoding's `+256` rule has already been applied.
    pub value: u64,
    /// Span of the physically encoded Frame Content Size field.
    pub span: ByteSpan,
}

/// A physically encoded Dictionary ID and its source location.
///
/// Presence is intentionally separate from numeric meaning: an explicitly
/// encoded zero is represented by `Some(DictionaryId { encoded: 0, .. })`,
/// while an absent field is represented by [`FrameHeader::dictionary_id`] being
/// `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct DictionaryId {
    /// Numeric value stored in the Dictionary ID field, including explicit zero.
    pub encoded: u32,
    /// Span of the physically encoded Dictionary ID field.
    pub span: ByteSpan,
}

/// Structural metadata for one Standard Frame block.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct Block {
    /// Zero-based block index within its Standard Frame.
    pub index: usize,
    /// Span of the three-byte block header.
    pub header_span: ByteSpan,
    /// Span of the encoded block-content bytes.
    ///
    /// The bytes are not copied into this model.
    pub content_span: ByteSpan,
    /// Parsed block type.
    pub block_type: BlockType,
    /// The 21-bit `Block_Size` value declared in the block header.
    ///
    /// For Raw and Compressed blocks this is also the encoded content length.
    /// For RLE blocks it is the decompressed repetition count and therefore is
    /// deliberately distinct from [`Block::encoded_content_size`].
    pub declared_size: u32,
    /// Number of encoded content bytes occupied by this block.
    ///
    /// This equals `declared_size` for Raw and Compressed blocks. RLE blocks
    /// always occupy one encoded content byte even when `declared_size` is
    /// larger.
    pub encoded_content_size: u32,
    /// Whether this block has the `Last_Block` bit set.
    pub is_last: bool,
}

/// Valid Standard Frame block types supported by the structural parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BlockType {
    /// Raw block whose encoded content length is `Block_Size`.
    Raw,
    /// Run-length encoded block with exactly one encoded content byte.
    Rle,
    /// Compressed block whose payload is treated as opaque encoded bytes.
    Compressed,
}

/// Stored 32-bit content checksum metadata.
///
/// The value is exposed exactly as stored after the final block. It is not
/// verified because v0.1 does not decompress the frame content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct ContentChecksum {
    /// Span of the four encoded checksum bytes.
    pub span: ByteSpan,
    /// Stored little-endian 32-bit checksum value.
    pub value: u32,
}

/// Structural metadata for a Zstandard Skippable Frame.
///
/// The user-defined payload is skipped without being copied or interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct SkippableFrame {
    /// Span of the four-byte skippable magic number.
    pub magic_span: ByteSpan,
    /// Exact skippable magic value in the range `0x184D2A50..=0x184D2A5F`.
    pub magic: u32,
    /// Low-nibble skippable-frame variant, in the inclusive range `0..=15`.
    pub variant: u8,
    /// Span of the four-byte little-endian payload-size field.
    pub size_field_span: ByteSpan,
    /// Payload length declared by the encoded size field.
    pub declared_payload_size: u32,
    /// Span of the opaque user-defined payload bytes.
    pub payload_span: ByteSpan,
}
