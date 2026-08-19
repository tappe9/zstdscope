# ZstdScope Architecture

Status: **Accepted for v0.1 implementation**

This document defines the accepted initial architecture for ZstdScope. The architecture is intentionally small for v0.1 and leaves room for future CLI, WebAssembly, streaming, and visualization use cases.

## 1. Architectural goals

The architecture prioritizes:

1. **Correctness against the Zstandard format specification.**
2. **Safety on untrusted bytes.**
3. **Clear separation between parsing and presentation.**
4. **Structural metadata suitable for inspection tooling.**
5. **Low dependency and platform coupling.**
6. **Future WebAssembly compatibility.**

Performance matters, but v0.1 must not trade parser clarity or safety for micro-optimizations.

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

Initial Cargo workspace:

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

The exact internal module names are not public API commitments. They describe intended responsibilities.

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
- the public inspection model.

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
- applying CLI-specific encoded-input size policy before parsing;
- selecting text or JSON output;
- rendering diagnostics;
- mapping failures to process exit codes.

The CLI must call the same public library API available to third-party users. It must not duplicate parsing logic.

## 5. Parsing pipeline

Top-level parsing flow:

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

The parser requires at least one complete frame. Empty input therefore fails while attempting to read the first top-level magic. Trailing bytes that cannot form another complete frame are also errors.

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

Untrusted bytes are accessed through one small abstraction rather than scattered indexing.

Conceptual internal API:

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

The exact private method names may be adjusted during implementation, but this responsibility boundary remains.

## 7. Public model design

The public model is inspection-oriented rather than decoder-oriented. The detailed accepted API direction lives in `docs/API-DESIGN.md`.

### Spans

```rust
pub struct ByteSpan {
    pub offset: u64,
    pub length: u64,
}
```

Offsets are zero-based from the beginning of the inspected input and refer to encoded bytes.

The parser uses `usize` internally for slice indexing while converting to public `u64` offsets and lengths with checked conversions.

Source spans are exposed for major structures and physically encoded optional frame-header fields so a future hex viewer can map model values back to input bytes.

### File and frame model

```rust
pub struct ZstdFile {
    pub input_size: u64,
    pub frames: Vec<Frame>,
}

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

### Dictionary ID fidelity

The model preserves the distinction between:

- no encoded Dictionary ID field;
- an explicitly encoded zero;
- a non-zero encoded ID.

This is an Inspector requirement even though encoded zero has the same Dictionary-ID meaning as an unspecified ID for decompression.

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

## 8. Ownership and allocation strategy

The initial `inspect(&[u8])` API is eager with respect to metadata but not payload data.

The parser allocates metadata structures such as frame and block vectors. It must not copy block contents or skippable payloads into the returned model.

Benefits:

- lower memory amplification;
- no allocation proportional to an untrusted declared payload merely to skip it;
- simpler optional JSON serialization;
- result types do not need lifetimes tied to the source slice;
- future WebAssembly integration is easier.

A malicious input can still contain a very large number of real frame/block headers, so metadata counts are a resource consideration. `inspect_with_limits()` addresses that concern with configurable frame/block count budgets without changing the fundamental result model.

The public library APIs still require the caller to own the complete input slice. Metadata-count limits therefore do not bound the caller's input buffer. File-size policy belongs to the file-reading application layer: the CLI applies a separate default 256 MiB encoded-input guard, while an accepted input remains fully resident in memory during inspection.

## 9. Error architecture

Errors are machine-readable and location-aware.

The accepted direction uses a typed `ZstdError` marked `#[non_exhaustive]`. Representative categories include:

- unexpected end of input;
- invalid top-level magic;
- reserved frame-header bit;
- reserved block type;
- invalid block size;
- arithmetic overflow.

Human-readable `Display` messages are important, but callers must be able to match error categories without parsing strings.

Empty input and trailing partial top-level magic may use the same typed EOF/truncation error category as other incomplete structures.

CLI-only resource policy failures, such as exceeding the configured encoded-input byte limit, remain structured CLI errors rather than `ZstdError` variants because they are not Zstandard format errors.

## 10. Validation policy

ZstdScope v0.1 is strict about structural rules it understands.

Examples:

- reject empty input because a Zstandard compressed stream contains at least one frame;
- reject an unknown top-level magic number;
- reject the reserved frame-header bit when set;
- reject reserved block type `3`;
- reject truncation rather than returning a partial successful model;
- validate lengths before skipping content;
- validate size constraints available from frame metadata.

It must **not** claim validation it cannot perform without decompression. In particular, exposing the stored content checksum does not mean validating it against the original content.

Future recovery or forensic modes must be explicit alternatives rather than weakening the v0.1 parser contract.

## 11. Serialization boundary

The core model is JSON-friendly, but JSON formatting is not a parser concern.

The core may expose optional `serde` serialization support:

```text
zstdscope features:
  serde   # Serialize public inspection model types where appropriate
```

`Deserialize` is not required for v0.1.

The CLI enables serialization support for `--json`. JSON field names and serialized enum values use `snake_case`. Enum representation must be chosen explicitly and covered by tests rather than relying accidentally on Serde defaults.

## 12. CLI architecture

Current command:

```text
zstdscope inspect <FILE> [--json] [--max-input-bytes <BYTES>]
```

The CLI keeps parsing in the library. It opens the input file, rejects files larger than the configured byte limit before the full read when file metadata already proves the limit would be exceeded, and also caps the actual read at one byte beyond the limit so file growth cannot silently bypass the configured boundary. The default limit is 268,435,456 bytes (256 MiB); callers can deliberately raise or lower it with `--max-input-bytes`.

After the bounded read succeeds, the CLI passes the complete byte slice to the existing public `inspect(&[u8])` API. The CLI does not contain an independent Zstandard parser, and the library API contract is unchanged.

This input-size guard bounds the encoded input buffer for the default CLI path but does not make inspection streaming: accepted input remains resident in memory, and metadata allocations remain governed separately by parser behavior or `inspect_with_limits()`. A streaming/file-backed inspection API is a later milestone and must not be simulated inside the CLI with duplicate parsing code.

## 13. Dependency policy

The library remains small and Pure Rust.

For v0.1:

- no `libzstd` or `zstd-sys`;
- no C/C++ FFI;
- no platform-specific system dependency;
- no project-authored `unsafe` code without a separately accepted architecture decision;
- keep mandatory dependencies minimal;
- serialization is optional.

"Pure Rust" means the Zstandard structure parser itself is implemented in Rust rather than delegating parsing to the C reference implementation. It does not mean every transitive crate must contain no `unsafe` internally; dependency review remains a separate supply-chain concern.

## 14. Specification governance

Authoritative sources are recorded in `docs/ZSTD-FORMAT.md`.

Implementation rules:

1. Do not infer format behavior solely from the reference implementation.
2. Prefer explicit documented format rules.
3. When RFC 8878 and the current reference format document materially differ, record the difference before choosing behavior.
4. Parser tests should link edge cases to the relevant specification rule where practical.

At the time this architecture was accepted, the Zstandard repository format document identifies itself as version 0.4.5 dated 2026-05-14.

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
- configurable metadata-count limits for callers that opt into them;
- a default bounded encoded-input read in the CLI;
- no project-authored `unsafe` in the initial parser;
- typed errors;
- targeted boundary tests;
- fuzz testing.

The CLI's default 256 MiB guard reduces the risk of an unexpectedly large whole-file allocation in normal CLI use. Raising `--max-input-bytes` is an explicit choice to accept a larger input buffer; it does not change the parser's in-memory architecture.

The "arbitrary input does not panic" invariant refers to malformed parser input; it does not claim recoverability from process-level failures such as global allocator exhaustion.

## 16. Testing architecture

Tests exist at multiple levels.

### Unit tests

For bit-field calculations, cursor reads, field-width rules, size derivation, checked conversions, and error locations.

### Parser integration tests

For complete standard frames, skippable frames, concatenation, empty/truncated input, and malformed streams.

### CLI boundary tests

For input-size limits, exact-boundary acceptance, rejection before parsing, diagnostic behavior, and CLI help/default-policy visibility.

### Reference-generated fixtures

Use the official `zstd` implementation to generate representative valid samples. Fixture provenance should be documented.

### Hand-built fixtures

Use exact byte arrays for reserved bits, truncation boundaries, width variants, explicit zero Dictionary IDs, and other cases requiring bit-level control.

### JSON tests

The CLI must test its JSON field names, enum values, and selected enum representation so output does not change merely because a serialization library default changes.

### Fuzzing

A fuzz target repeatedly calls the public parser on arbitrary bytes. The baseline parser invariant is:

```text
malformed arbitrary bytes -> Ok(...) or Err(...), never a parser panic
```

Differential checks against the reference implementation may be explored later, while remembering that ZstdScope is an inspector rather than a decompressor.

## 17. Future extension points

The architecture leaves room for:

- streaming/event-based inspection;
- WebAssembly bindings;
- a browser hex viewer;
- literals/sequence metadata parsing;
- Huffman/FSE inspection;
- dictionary-format inspection;
- benchmark tooling;
- tolerant corruption scanning;
- editor integrations.

These extensions should build on the library model instead of bypassing it.

## 18. Accepted ADRs

The v0.1 architecture is governed by:

- ADR 0001 — Pure Rust structural parser;
- ADR 0002 — separate parser core and CLI crates;
- ADR 0003 — start with an in-memory `&[u8]` inspection API;
- ADR 0004 — v0.1 public API and licensing policy.

Issue #37 adds a CLI-specific bounded-read policy without changing the public parser API, so it does not supersede ADR 0003 or require a new public parsing architecture decision.

A future implementation that intentionally diverges from these decisions should introduce a new ADR rather than silently changing the architecture.
