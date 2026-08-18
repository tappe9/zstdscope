# ZstdScope Public API Design

Status: **Draft**

This document proposes the first public Rust API for discussion. It is intentionally conservative because API names and semantics become costly to change once the crate is published.

## 1. Design principles

The public API should:

- describe encoded Zstandard structure rather than decompressed semantics;
- make byte locations explicit;
- distinguish stored lengths from decoded/logical lengths;
- avoid borrowing payload slices in returned models;
- expose typed errors;
- remain convenient for CLI and JSON consumers;
- avoid promising unstable internals before v1.0.

## 2. Entry point

Proposed initial entry point:

```rust
pub fn inspect(input: &[u8]) -> Result<ZstdFile, ZstdError>;
```

Reasons:

- easy to understand;
- easy to test and fuzz;
- no filesystem coupling;
- compatible with browser/WASM byte buffers;
- leaves room for a separate streaming API later.

A file-path API such as `inspect_file()` should live in the CLI or a future convenience layer rather than the parser core.

## 3. Common span type

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSpan {
    pub offset: u64,
    pub length: u64,
}
```

Possible convenience methods:

```rust
impl ByteSpan {
    pub fn end(&self) -> Option<u64>;
    pub fn is_empty(&self) -> bool;
}
```

`end()` should use checked arithmetic rather than silently wrap.

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

`Frame::span` represents the complete encoded frame, including magic number and optional checksum.

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

### Frame header

The header should expose values useful for inspection while retaining raw descriptor information:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    pub span: ByteSpan,
    pub descriptor: u8,
    pub frame_content_size: Option<u64>,
    pub window_size: u64,
    pub dictionary_id: Option<u32>,
    pub content_checksum_flag: bool,
    pub single_segment: bool,
    pub unused_bit: bool,
}
```

Questions to resolve during implementation review:

1. Should field-specific spans be exposed in v0.1, or added later when the hex viewer is implemented?
2. Should `dictionary_id` map encoded zero to `None`, or should the API preserve both `encoded_dictionary_id: Option<u32>` and a semantic helper?

The preferred direction is to avoid losing encoded information. One option is:

```rust
pub struct DictionaryId {
    pub encoded: u32,
    pub span: ByteSpan,
}
```

with a helper describing whether the ID is semantically specified. This decision should be made before code is published.

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

- Raw: both values are `Block_Size`.
- Compressed: both values are `Block_Size`.
- RLE: `declared_size` is the decompressed repetition count, while `encoded_content_size` is `1`.

This naming prevents consumers from making incorrect offset calculations for RLE blocks.

A convenience method may be useful:

```rust
impl Block {
    pub fn encoded_span(&self) -> Option<ByteSpan>;
}
```

but v0.1 should avoid unnecessary convenience surface until use cases are proven.

## 7. Content checksum model

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentChecksum {
    pub span: ByteSpan,
    pub value: u32,
}
```

The API must document that `value` is the stored checksum field. ZstdScope does not verify it against decoded content in v0.1.

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

The exact 4-byte magic number should be retained rather than represented only as a boolean "skippable" marker. This preserves information useful to tooling.

`variant` represents the low nibble distinguishing the 16 valid skippable magic values.

The payload itself is not copied into the model.

## 9. Error API

Proposed direction:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
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

- implement `std::error::Error`;
- implement useful `Display` messages;
- preserve structured fields;
- keep the error enum `#[non_exhaustive]` under consideration so future parser validation can add variants without forcing a major version bump.

Before choosing `#[non_exhaustive]`, document the ergonomics trade-off for downstream exhaustive matching.

## 10. Serialization

Proposed optional Cargo feature:

```toml
[features]
default = []
serde = ["dep:serde"]
```

When enabled, public inspection model types can derive `Serialize`. Deserialization is not automatically required; ZstdScope's primary job is parsing encoded bytes, not accepting arbitrary JSON as trusted model state.

Therefore the preferred v0.1 direction is:

```text
Serialize: yes, behind optional feature
Deserialize: only if a concrete use case appears
```

The CLI enables `serde` and performs JSON rendering in the CLI crate.

## 11. Versioning policy

Before v1.0:

- public API is considered evolving;
- breaking changes should still be explained in `CHANGELOG.md` once releases begin;
- avoid publishing internal parser types solely because they happen to exist;
- prefer a small stable inspection model over a large one-to-one mapping of every internal function.

At v1.0, the crate should document which model fields and JSON representations are stability commitments.

## 12. Deferred APIs

The following are intentionally deferred:

```rust
inspect_reader<R: Read>(reader: R)
inspect_stream(...)
recover_corrupt_stream(...)
parse_compressed_block_internals(...)
verify_checksum(...)
```

Adding these later is easier than removing a prematurely generalized abstraction.

## 13. Open API questions

The first implementation PR should resolve these explicitly:

1. **Dictionary ID fidelity:** preserve encoded zero distinctly, or normalize it?
2. **Field spans:** expose a span for every optional frame-header field in v0.1 or only aggregate header span?
3. **Error extensibility:** use `#[non_exhaustive]` on `ZstdError` and/or public enums?
4. **JSON field naming:** Rust `snake_case` or a stable external naming convention such as `camelCase`?
5. **Input size type:** keep public offsets as `u64` while parser indexing uses `usize`?

No code should harden these unresolved choices before they are reviewed.
