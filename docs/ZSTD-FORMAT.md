# Zstandard Format Notes for ZstdScope

Status: **Accepted v0.1 design reference**

This document is a project-oriented summary of the Zstandard format rules needed by ZstdScope. It is not a replacement for the authoritative specification.

## 1. Authoritative references

ZstdScope should be implemented from primary sources:

- RFC 8878: https://www.rfc-editor.org/rfc/rfc8878.html
- Current Zstandard format document: https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md
- Reference implementation: https://github.com/facebook/zstd

At the time this document was accepted, the Zstandard repository format document identifies itself as **version 0.4.5 (2026-05-14)**.

RFC 8878 establishes the stable interoperable format. The repository format document may evolve with clarifications or compatible format work. If the two sources differ in a way that affects ZstdScope, do not silently choose one: record the difference in an issue or ADR and add a targeted test.

## 2. Stream model

Zstandard compressed data is made of one or more frames. A byte stream can contain multiple concatenated frames, and each top-level frame is independently delimited.

ZstdScope v0.1 recognizes two top-level frame classes:

1. standard Zstandard frames;
2. skippable frames.

The v0.1 parser is strict. Inspection continues frame by frame until the input is exhausted. Unknown top-level magic is a parse error. An empty input is also invalid because it contains no frame; the implementation should report a typed truncation/EOF-style parse error rather than return an empty successful model.

## 3. Standard Zstandard frame

A standard frame is structurally:

```text
Magic Number
Frame Header
Block
[Block ...]
[Content Checksum]
```

### Magic number

The standard magic number is:

```text
0xFD2FB528
```

It occupies four bytes and is encoded little-endian.

ZstdScope should retain its byte span even though the value itself is fixed, because source mapping is a core Inspector feature.

## 4. Frame header

The frame header is variable-length. Its first byte, the frame-header descriptor, determines which optional fields follow.

Conceptual layout:

```text
Frame Header Descriptor   1 byte
[Window Descriptor]       0 or 1 byte
[Dictionary ID]           0, 1, 2, or 4 bytes
[Frame Content Size]      0, 1, 2, 4, or 8 bytes
```

### Frame-header descriptor bits

```text
bits 7..6  Frame Content Size flag
bit  5     Single Segment flag
bit  4     Unused bit
bit  3     Reserved bit
bit  2     Content Checksum flag
bits 1..0  Dictionary ID flag
```

ZstdScope policy:

- preserve/report the unused bit without inventing semantics;
- reject the reserved bit when set;
- derive optional field widths before consuming them;
- expose the original descriptor byte and its source span for inspection/debugging;
- expose source spans for physically encoded optional header fields.

## 5. Frame Content Size

`Frame_Content_Size` is the decompressed size when the field is present.

The descriptor encodes field width. The possible widths are:

```text
0, 1, 2, 4, or 8 bytes
```

A missing field means **unknown size**, not zero.

Important special case: the two-byte representation stores a value that requires adding the format-defined offset of 256 when decoded.

ZstdScope preserves both the decoded value and the source byte span. All width and value arithmetic must be checked before being converted to public model types.

## 6. Single Segment and Window Size

When `Single_Segment_flag` is set:

- no Window Descriptor byte is present;
- Frame Content Size is present;
- effective Window Size is the Frame Content Size.

When it is not set, the Window Descriptor carries an exponent and mantissa used to derive the window size.

The current format specification defines the derivation conceptually as:

```text
window_log  = 10 + exponent
window_base = 1 << window_log
window_add  = (window_base / 8) * mantissa
window_size = window_base + window_add
```

ZstdScope should implement this with checked arithmetic and unit tests for minimum, maximum, and representative values.

When a Window Descriptor is physically present, its source span is exposed by the inspection model.

## 7. Dictionary ID

The dictionary ID flag selects an encoded field width of:

```text
0, 1, 2, or 4 bytes
```

The field is little-endian.

An encoded dictionary ID value of zero has a subtle meaning: it has the same Dictionary-ID meaning as an unspecified ID, but it does not prove that decompression requires no dictionary.

ZstdScope is an inspector, so it preserves the byte-level distinction between:

- no Dictionary ID field being encoded;
- a Dictionary ID field explicitly encoding zero;
- a Dictionary ID field encoding a non-zero value.

The accepted public model uses `Option<DictionaryId>`: `None` means the field is absent, while `Some(DictionaryId { encoded: 0, .. })` preserves an explicitly encoded zero. See `docs/API-DESIGN.md` and ADR 0004.

## 8. Content checksum

When the content-checksum flag is set, a four-byte checksum field appears after the last block.

The checksum is derived from the original decoded content. Therefore a structural inspector that does not decompress payloads can:

- locate the checksum;
- read its stored value;
- expose it in the model;

but cannot generally verify it.

ZstdScope v0.1 must not describe an unverified checksum as valid.

## 9. Blocks

Every standard frame contains at least one block.

A block is:

```text
Block Header      3 bytes
Block Content     variable
```

The 24-bit little-endian block header is partitioned as:

```text
bit 0       Last Block
bits 1..2   Block Type
bits 3..23  Block Size
```

### Block types

```text
0  Raw
1  RLE
2  Compressed
3  Reserved / invalid
```

Reserved block type `3` must be rejected.

### Block Size semantics

This is an important Inspector-specific detail.

For Raw and Compressed blocks:

```text
encoded Block Content length = Block_Size
```

For RLE blocks:

```text
encoded Block Content length = 1 byte
Block_Size = decompressed repetition count
```

Therefore ZstdScope must not use one ambiguous field to represent both concepts. The public model exposes both `declared_size` and `encoded_content_size`, and offset calculations must use encoded content length.

### Last Block

The parser continues reading block headers until the Last Block bit is set. Only after the final block may the optional content checksum appear.

## 10. Block maximum size

The format limits block size using the effective frame window and the format's 128 KiB block ceiling.

ZstdScope should validate size rules that can be determined from header metadata without decompressing the block.

Care is required for RLE because its `Block_Size` describes decoded length while its encoded content is one byte.

## 11. Compressed block internals

A Compressed block contains further Zstandard structures, including literals and sequences, with entropy coding such as Huffman and FSE involved deeper in the format.

These internals are **opaque in v0.1**.

ZstdScope v0.1 only needs to:

1. recognize the block type;
2. validate the encoded length against available input and structural constraints;
3. record offsets and sizes;
4. skip the encoded content safely.

Future versions may add nested inspection without changing the fundamental frame/block model.

## 12. Skippable frames

Skippable frames allow user-defined metadata or other payloads to appear between Zstandard frames.

Their conceptual layout is:

```text
Magic Number    4 bytes
Frame Size      4 bytes
User Data       Frame Size bytes
```

Valid skippable magic numbers cover all 16 values:

```text
0x184D2A50 .. 0x184D2A5F
```

The four-byte Frame Size is little-endian and declares the User Data length.

ZstdScope should:

- recognize the full 16-value range;
- preserve the exact magic value;
- expose the low-nibble variant/tag;
- expose the declared payload size;
- expose payload and total-frame spans;
- skip payload bytes without copying them into the result;
- reject truncation when the declared payload extends beyond the input.

ZstdScope v0.1 does not assign application semantics to skippable payload data.

## 13. Concatenation

A valid inspected input may contain combinations such as:

```text
Standard Frame
Standard Frame
```

or:

```text
Standard Frame
Skippable Frame
Standard Frame
```

The returned model preserves input order and provides a monotonically increasing frame index.

Trailing bytes that cannot form another complete valid frame are an error; they are not silently ignored.

## 14. Offset conventions

All ZstdScope public byte offsets are:

- zero-based;
- relative to the beginning of the inspected input;
- measured in encoded bytes.

Do not mix decoded positions with encoded source positions in the same field names.

## 15. Test traceability

For bit-level rules, tests should include a short comment pointing to the corresponding specification concept. This is especially useful for:

- descriptor flag widths;
- two-byte Frame Content Size offset behavior;
- Single Segment behavior;
- Dictionary ID absence versus explicitly encoded zero;
- skippable magic range;
- RLE Block Size semantics;
- reserved bits and block types;
- empty and truncated input behavior.

When the reference specification changes, this document and those tests should be reviewed together.
