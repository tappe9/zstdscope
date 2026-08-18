# ZstdScope Public API Design

Status: **Accepted for v0.1 implementation**

This document defines the initial public Rust API direction for ZstdScope. The API remains pre-1.0 and may evolve, but the design decisions below are the implementation contract unless superseded by a new ADR.

See [ADR 0004](adr/0004-v0.1-public-api-policy.md) for the decisions that resolved the initial open questions.

## 1. Design principles

The public API should:

- describe encoded Zstandard structure rather than decompressed semantics;
- preserve byte-level distinctions that matter to an inspector;
- make source byte locations explicit;
- distinguish stored lengths from decoded/logical lengths;
- avoid copying opaque payload bytes into returned models;
- expose typed errors;
- remain convenient for CLI and JSON consumers;
- keep mandatory parser-core dependencies minimal;
- avoid promising unstable internals before v1.0.

## 2. Entry point

The initial entry point is:

```rust
pub fn inspect(input: &[u8]) -> Result<ZstdFile, ZstdError>;
```

Reasons:

- easy to understand;
- easy to test and fuzz;
- no filesystem coupling;
- compatible with browser/WASM byte buffers;
- leaves room for a separate streaming API later.

A file-path API such as `inspect_file()` belongs in the CLI or a future convenience layer rather than the parser core.

The v0.1 parser requires at least one complete frame. `inspect(&[])` returns a typed EOF/truncation-style error rather than an empty successful `ZstdFile`. Trailing bytes that cannot form another complete frame are also errors.

## 3. Common span type

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSpan {
    pub offset: u64,
    pub length: u64,
}
```

Expected convenience methods:

```rust
impl ByteSpan {
    pub fn end(&self) -> Option<u64>;
    pub fn is_empty(&self) -> bool;
}
```

`end()` must use checked arithmetic rather than silently wrap.

ZstdScope is an inspector, so source spans are part of the product rather than an implementation detail.

## 4. Top-level model

```rust
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
```

`Frame::span` represents the complete encoded frame, including its magic number and optional checksum.

## 5. Standard frame model

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardFrame {
    pub magic_span: ByteSpan,
    pub header: FrameHeader,
    pub blocks: Vec<Block>,
    pub content_checksum: Option<ContentChecksum>,
}
```

### 5.1 Frame header

The header exposes decoded inspection values while retaining the location of fields that physically occur in the input.

A representative direction is:

```rust
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
```

Exact type names may be refined during implementation, but v0.1 must retain field-specific spans for physically encoded optional header fields.

### 5.2 Dictionary ID fidelity

The encoded representation must be preserved.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DictionaryId {
    pub encoded: u32,
    pub span: ByteSpan,
}
```

`FrameHeader::dictionary_id` uses `Option<DictionaryId>`:

- `None` means the Dictionary ID field was absent;
- `Some(DictionaryId { encoded: 0, .. })` means a zero value was explicitly encoded;
- a non-zero value preserves the encoded Dictionary ID.

Although encoded zero has the same Dictionary-ID meaning as an unspecified ID for decompression, an inspector must not erase the byte-level distinction.

A semantic convenience helper may later be added, but the stored representation remains observable.

## 6. Block model

```rust
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
```

The distinction between `declared_size` and `encoded_content_size` is required:

- Raw: both values are `Block_Size`;
- Compressed: both values are `Block_Size`;
- RLE: `declared_size` is the decompressed repetition count while `encoded_content_size` is `1`.

This prevents consumers from calculating incorrect source offsets for RLE blocks.

`BlockType` mirrors the supported valid block types in the current format and is not automatically marked `#[non_exhaustive]`.

## 7. Content checksum model

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentChecksum {
    pub span: ByteSpan,
    pub value: u32,
}
```

`value` is the checksum value stored in the encoded frame. ZstdScope v0.1 does not verify it against decoded content because the core does not decompress payload data.

## 8. Skippable frame model

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippableFrame {
    pub magic_span: ByteSpan,
    pub magic: u32,
    pub variant: u8,
    pub size_field_span: ByteSpan,
    pub declared_payload_size: u32,
    pub payload_span: ByteSpan,
}
```

The exact four-byte magic value is retained. `variant` identifies the low-nibble variant among the 16 valid skippable magic values.

The payload itself is not copied into the result model.

## 9. Error API

Initial direction:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ZstdError {
    UnexpectedEof {
        offset: u64,
        needed: usize,
        remaining: usize,
    },
    InvalidMagic {
        offset: u64,
        magic: u32,
    },
    ReservedFrameHeaderBit {
        offset: u64,
    },
    ReservedBlockType {
        offset: u64,
    },
    InvalidBlockSize {
        offset: u64,
        size: u32,
        maximum: u32,
    },
    ArithmeticOverflow {
        offset: u64,
    },
}
```

Requirements:

- `ZstdError` is `#[non_exhaustive]`;
- implement `std::error::Error`;
- implement useful `Display` messages;
- preserve structured fields so callers do not need to parse strings;
- include a zero-based source offset where meaningful.

The non-exhaustive policy allows future parser validation to add error categories without freezing the initial error taxonomy permanently.

Empty input and a trailing partial top-level magic can be represented with `UnexpectedEof`; a dedicated `EmptyInput` variant is not required for v0.1.

## 10. Serialization and JSON

Serialization is optional in the core crate.

Expected Cargo feature direction:

```toml
[features]
default = []
serde = ["dep:serde"]
```

When enabled, public inspection model types may derive `Serialize`. `Deserialize` is not required for v0.1 because ZstdScope parses encoded bytes rather than accepting JSON as source truth.

The CLI enables serialization support for `--json`.

### JSON naming and representation

Machine-readable output uses **`snake_case`** for both field names and serialized enum values.

For example, `BlockType::Compressed` must serialize as `"compressed"`, not the Serde default `"Compressed"`.

The representation of enums such as `FrameKind` must be selected explicitly with serialization attributes or an explicit CLI DTO. The implementation must not accidentally make Serde defaults part of the external format. JSON output must have focused fixture/snapshot-style tests.

Example field naming:

```json
{
  "frame_content_size": 12345,
  "content_checksum_flag": true,
  "block_type": "compressed"
}
```

Before v1.0 the JSON schema may evolve. Once releases begin, intentional JSON changes should be recorded in the changelog.

## 11. Dependency policy

The `zstdscope` parser core keeps mandatory dependencies to a minimum.

In particular:

- no `libzstd`;
- no `zstd-sys`;
- no FFI-based decompression dependency;
- no CLI framework in the core crate;
- `serde` remains optional if used.

The CLI crate may use ergonomic dependencies for argument parsing, JSON output, and presentation because those concerns are separate from parsing.

## 12. Offset types

Public byte offsets and lengths use `u64`.

Internal indexing into a byte slice uses `usize`, with checked conversion when crossing the public/internal boundary.

This avoids exposing platform-sized offsets in the public inspection model while preserving safe indexing in Rust.

## 13. Versioning policy

Before v1.0:

- the public API is considered evolving;
- breaking changes should still be intentional and documented once releases begin;
- internal parser types should not become public solely for implementation convenience;
- accepted architecture decisions should be changed through a new ADR rather than silently diverging from documentation.

At v1.0, the crate should document which model fields and JSON representations are stability commitments.

## 14. Deferred APIs

The following are intentionally deferred:

```rust
inspect_reader<R: Read>(reader: R)
inspect_stream(...)
recover_corrupt_stream(...)
parse_compressed_block_internals(...)
verify_checksum(...)
```

Adding these later is easier than removing a prematurely generalized abstraction.

## 15. Resolved v0.1 policy

The initial design review resolved the following:

1. **Dictionary ID fidelity:** preserve an explicitly encoded zero separately from an absent field.
2. **Field spans:** expose byte spans for physically encoded frame-header fields in v0.1.
3. **Error extensibility:** `ZstdError` is `#[non_exhaustive]`; other enums are considered individually.
4. **JSON naming:** use `snake_case` for field names and enum values, with explicit serialization behavior.
5. **Offset types:** public offsets use `u64`; parser indexing uses `usize` with checked conversions.
6. **Dependencies:** keep the core dependency-light; serialization is optional and CLI dependencies stay outside the core.
7. **Licensing:** the project is `MIT OR Apache-2.0`.
8. **Empty input:** strict v0.1 inspection requires at least one complete frame and rejects empty input.

## 16. Configurable metadata resource limits

Issue #34 adds an opt-in resource-budget API without changing the original `inspect(&[u8])` contract:

```rust
pub struct InspectionLimits {
    pub max_frames: usize,
    pub max_blocks_per_frame: usize,
    pub max_total_blocks: usize,
}

pub fn inspect_with_limits(
    input: &[u8],
    limits: InspectionLimits,
) -> Result<ZstdFile, ZstdError>;
```

`inspect(&[u8])` remains the convenience entry point and does not impose frame/block count limits. Callers that accept untrusted or externally supplied inputs should prefer `inspect_with_limits` with an application-appropriate metadata budget.

Limit semantics are intentionally count based:

- `max_frames` counts Standard and Skippable Frames;
- `max_blocks_per_frame` limits blocks within each Standard Frame;
- `max_total_blocks` limits blocks across all Standard Frames in the complete input;
- reaching a configured count is allowed; attempting to parse one additional affected structure fails;
- the error offset is the start of the frame magic or block header that would exceed the budget;
- when both block limits are exhausted at the same block, the more specific per-frame limit is reported first.

Limit exhaustion uses a typed error:

```rust
pub enum ResourceLimitKind {
    Frames,
    BlocksPerFrame,
    TotalBlocks,
}

ZstdError::ResourceLimitExceeded {
    offset: u64,
    resource: ResourceLimitKind,
    limit: usize,
}
```

The count limits do not impose an input-byte limit and do not make the in-memory API streaming. The caller still owns the complete input slice. They also do not introduce payload-sized allocations: opaque Block and Skippable payload bytes continue to be skipped and represented by spans. Streaming/file-backed inspection remains a separate future API concern.

A limit can be zero. In that case, the first attempt to parse the affected resource returns `ResourceLimitExceeded` at that resource's starting offset. The unlimited legacy `inspect(&[])` behavior is unchanged and still reports the existing typed EOF/truncation error for empty input.
