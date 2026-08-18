# ZstdScope Architecture

Status: **Draft**

This document describes the proposed architecture for ZstdScope before implementation begins. The architecture is intentionally small for v0.1 and keeps room for future CLI, WebAssembly, streaming, and visualization use cases.

## 1. Architectural goals

The architecture prioritizes:

1. **Correctness against the Zstandard format specification.**
2. **Safety on untrusted bytes.**
3. **Clear separation between parsing and presentation.**
4. **Stable structural metadata suitable for tools.**
5. **Low dependency and platform coupling.**
6. **Future WebAssembly compatibility.**

Performance matters, but v0.1 should not trade parser clarity or safety for micro-optimizations.

## 2. System boundary

ZstdScope inspects encoded Zstandard structure. It does not decode compressed payloads.

```text
encoded bytes
    │
    ▼
┌───────────────────────┐
│   zstdscope library   │
│                       │
│  safe reader/cursor   │
│          │            │
│       parser          │
│          │            │
│       model           │
└──────────┬────────────┘
           │
           ▼
      ZstdFile model
           │
     ┌─────┴─────┐
     ▼           ▼
 CLI text     CLI JSON

Future consumers:
- WebAssembly/Web UI
- editor extensions
- forensic tooling
- benchmark/analysis tooling
```

No filesystem, terminal, JSON formatting, or command-line behavior belongs in the parser library's core parsing path.

## 3. Workspace layout

Proposed Cargo workspace:

```text
zstdscope/
├── Cargo.toml
├── crates/
│   ├── zstdscope/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── cursor.rs
│   │       ├── error.rs
│   │       ├── model.rs
│   │       └── parser/
│   │           ├── mod.rs
│   │           ├── frame.rs
│   │           └── block.rs
│   └── zstdscope-cli/
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
├── docs/
└── tests/
```

The exact module names are not API commitments. They describe intended responsibilities.

## 4. Crate responsibilities

### `zstdscope`

The library owns:

- bounds-checked byte reading;
- Zstandard top-level frame detection;
- standard frame parsing;
- skippable frame parsing;
- frame-header parsing and derived values;
- block-header parsing;
- structural validation possible without decompression;
- offsets and encoded-size accounting;
- typed parse errors;
- public inspection model.

The library does **not** own:

- filesystem reads;
- terminal output;
- process exit codes;
- CLI argument parsing;
- decompression;
- compressed block internals in v0.1.

### `zstdscope-cli`

The CLI owns:

- command-line argument parsing;
- reading an input file;
- selecting text or JSON output;
- rendering diagnostics;
- mapping failures to process exit codes.

The CLI must call the same public library API available to third-party users. It must not duplicate parsing logic.

## 5. Parsing pipeline

The top-level parsing flow is proposed as:

```text
inspect(&[u8])
    │
    ▼
Cursor
    │
    ▼
read 4-byte magic
    │
    ├── standard magic ──► parse_standard_frame()
    │
    ├── skippable magic ─► parse_skippable_frame()
    │
    └── other ───────────► InvalidMagic
    │
    ▼
append Frame
    │
    ▼
input remaining?
    │
    ├── yes: parse next frame
    └── no: return ZstdFile
```

### Standard frame flow

```text
magic
  │
  ▼
frame header descriptor
  │
  ├─ determine optional field widths
  ▼
window descriptor (when present)
  ▼
dictionary ID (when present)
  ▼
frame content size (when present)
  ▼
block header ──► opaque block content
  │                  │
  └──── repeat until Last_Block
  ▼
content checksum (when present)
  ▼
StandardFrame
```

Compressed block contents are skipped by encoded length in v0.1. They are not interpreted.

## 6. Safe cursor abstraction

Untrusted bytes should be accessed through one small abstraction rather than by scattered indexing.

Conceptual API:

```rust
struct Cursor<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn position(&self) -> usize;
    fn remaining(&self) -> usize;
    fn read_u8(&mut self) -> Result<u8, ZstdError>;
    fn read_u16_le(&mut self) -> Result<u16, ZstdError>;
    fn read_u24_le(&mut self) -> Result<u32, ZstdError>;
    fn read_u32_le(&mut self) -> Result<u32, ZstdError>;
    fn read_u64_le(&mut self) -> Result<u64, ZstdError>;
    fn skip(&mut self, len: usize) -> Result<(), ZstdError>;
}
```

Requirements for the cursor:

- every read is bounds checked;
- every position increment is checked;
- errors report the position at which the read failed;
- skipping does not allocate;
- parser code does not directly index attacker-controlled offsets into the input.

The exact API may be adjusted during implementation, but the single-responsibility boundary should remain.

## 7. Model design

The public model is inspection-oriented rather than decoder-oriented.

### Spans

A reusable byte-span type is proposed:

```rust
pub struct ByteSpan {
    pub offset: u64,
    pub length: u64,
}
```

Offsets are zero-based from the beginning of the inspected input.

The parser can use `usize` internally for slice indexing while converting to public offset types with checked conversions. Centralizing conversion avoids accidental unchecked casts.

### File model

```rust
pub struct ZstdFile {
    pub input_size: u64,
    pub frames: Vec<Frame>,
}
```

### Frame model

A frame should carry common location information and a typed variant:

```rust
pub struct Frame {
    pub index: usize,
    pub span: ByteSpan,
    pub kind: FrameKind,
}

pub enum FrameKind {
    Standard(StandardFrame),
    Skippable(SkippableFrame),
}
```

### Standard frame

Conceptually:

```rust
pub struct StandardFrame {
    pub header: FrameHeader,
    pub blocks: Vec<Block>,
    pub content_checksum: Option<ContentChecksum>,
}
```

The header should retain both decoded/derived values useful to callers and raw information useful to inspection. For example, retaining the raw frame-header descriptor byte makes debugging easier.

### Block model

The model must not treat Zstandard `Block_Size` as synonymous with encoded content length.

```rust
pub struct Block {
    pub index: usize,
    pub header_span: ByteSpan,
    pub content_span: ByteSpan,
    pub block_type: BlockType,
    pub declared_size: u32,
    pub encoded_content_size: u32,
    pub is_last: bool,
}
```

For Raw and Compressed blocks, `declared_size` and `encoded_content_size` are equal. For RLE blocks, the encoded content length is one byte while the declared size represents the repetition count in decompressed output.

This distinction is important for accurate offsets and future visualization.

### Skippable frame

Conceptually:

```rust
pub struct SkippableFrame {
    pub magic: u32,
    pub variant: u8,
    pub payload_span: ByteSpan,
    pub declared_payload_size: u32,
}
```

Payload bytes are not copied into the result in v0.1. Consumers can map the span back to the original input if they need them.

## 8. Ownership and allocation strategy

The initial `inspect(&[u8])` API is eager with respect to metadata but not payload data.

The parser should allocate only metadata structures such as the frame and block vectors. It should not copy block contents or skippable payloads into the returned model.

Benefits:

- lower memory amplification;
- no attacker-controlled payload-sized allocation;
- simpler JSON serialization;
- result types do not need lifetimes tied to the source byte slice;
- future WebAssembly integration is easier.

A future streaming API may produce events or incremental frame models, but it is intentionally outside v0.1.

## 9. Error architecture

Errors should be machine-readable and location-aware.

A proposed shape is:

```rust
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

This list is illustrative, not final.

Human-readable `Display` messages are important, but callers must be able to match variants without parsing strings.

## 10. Validation policy

ZstdScope v0.1 should be strict about structural rules it understands.

Examples:

- reject an unknown top-level magic number;
- reject the reserved frame-header bit when set;
- reject reserved block type `3`;
- reject truncation rather than returning a partial successful model;
- validate lengths before skipping content;
- validate size constraints available from frame metadata.

It should **not** claim validation it cannot perform without decompression. In particular, storing the content checksum does not mean validating it against the original content.

Future recovery or forensic modes should be explicit alternatives rather than weakening the default parser contract.

## 11. Serialization boundary

The core data model should be JSON-friendly, but JSON is not a parser concern.

A likely approach is an optional core feature:

```text
zstdscope features:
  serde   # derives Serialize/Deserialize where appropriate
```

The CLI can enable that feature and use a JSON library. This keeps users who only need parsing from paying for serialization dependencies.

The exact dependency choice should be made during implementation planning.

## 12. CLI architecture

Proposed command:

```text
zstdscope inspect <FILE> [--json]
```

v0.1 may read the file into memory and pass a slice to the library. This is simple and consistent with the initial API, but it is a known limitation for very large files.

A streaming/file-backed inspection API is a candidate for a later milestone and should not be simulated inside the CLI with duplicate parsing code.

## 13. Dependency policy

The library should remain small and pure Rust.

For v0.1:

- no `libzstd` or `zstd-sys`;
- no C/C++ FFI;
- no platform-specific system dependency;
- no `unsafe` code without a separately approved architecture decision;
- keep required dependencies minimal;
- serialization should preferably be optional.

"Pure Rust" in this project means the Zstandard structure parser itself is implemented in Rust rather than delegating parsing to the C reference implementation. It does not mean every transitive crate must contain no `unsafe` internally; dependency review remains a separate supply-chain concern.

## 14. Specification governance

Authoritative sources are recorded in `docs/ZSTD-FORMAT.md`.

Implementation rules:

1. Do not infer format behavior solely from the reference implementation.
2. Prefer explicit documented format rules.
3. When RFC 8878 and the current reference format document materially differ, record the difference before choosing behavior.
4. Parser tests should link edge cases to the relevant specification rule where practical.

At the time of this architecture draft, the Zstandard repository's format document identifies itself as version 0.4.5 dated 2026-05-14. The repository documentation should record the specification version used when parser behavior changes.

## 15. Security model

The attacker controls every input byte.

Threats include:

- truncated fields;
- malicious declared sizes;
- integer overflow during offset arithmetic;
- excessive metadata counts;
- attempts to force very large allocations;
- malformed bit fields;
- very long concatenated streams.

Primary controls:

- centralized bounds-checked cursor;
- checked integer arithmetic;
- no copies of opaque payload data;
- no `unsafe` in the initial parser;
- typed errors;
- targeted boundary tests;
- fuzz testing.

A future parser limit/configuration API may be needed to cap frame/block counts for hostile environments. v0.1 design should avoid making such limits impossible to add compatibly.

## 16. Testing architecture

Tests should exist at multiple levels:

### Unit tests

For bit-field calculations, cursor reads, field-width rules, size derivation, and error locations.

### Parser integration tests

For complete standard frames, skippable frames, concatenation, and malformed streams.

### Reference-generated fixtures

Use the official `zstd` implementation to generate representative valid samples. Generated fixture provenance should be documented.

### Hand-built fixtures

Use exact byte arrays for reserved bits, truncation boundaries, width variants, and other cases that are difficult to force through a normal compressor.

### Fuzzing

A fuzz target should repeatedly call the public parser on arbitrary bytes. The minimum invariant is:

```text
arbitrary bytes -> Ok(...) or Err(...), never panic
```

Differential checks against the reference implementation may be explored later, while remembering that ZstdScope is an inspector rather than a decompressor.

## 17. Future extension points

The architecture intentionally leaves room for:

- streaming/event-based inspection;
- WebAssembly bindings;
- a browser hex viewer;
- literals/sequence metadata parsing;
- Huffman/FSE inspection;
- dictionary-format inspection;
- benchmark tooling;
- tolerant corruption scanning;
- editor integrations.

These should build on the library model instead of bypassing it.
