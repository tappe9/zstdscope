# ZstdScope Requirements

Status: **Draft**

This document defines the intended scope and quality requirements for the first implementation milestone of ZstdScope. It is a design contract, not a description of existing behavior.

## 1. Product definition

ZstdScope is a parser and inspection toolkit for the Zstandard binary format. It reads encoded Zstandard data and returns structural metadata without attempting to reconstruct the original uncompressed payload.

The primary product is a reusable Rust library. A CLI is a consumer of that library.

## 2. Goals

### G-001 — Structural inspection

Given a byte slice containing one or more valid Zstandard or skippable frames, ZstdScope should return a structured representation of the stream, including locations and encoded sizes.

### G-002 — Safe parsing of untrusted input

Malformed input must result in a typed error rather than a panic, out-of-bounds access, integer overflow, or uncontrolled allocation.

### G-003 — Useful diagnostics

When parsing fails, the error should identify the byte offset and enough context to understand which field or invariant failed.

### G-004 — Reusable core

Parsing behavior must not depend on terminal output, filesystem access, or CLI-specific types.

### G-005 — Future portability

The core design should avoid unnecessary platform-specific dependencies so that a future `wasm32` target remains practical.

## 3. Non-goals for v0.1

ZstdScope v0.1 will not:

- compress data;
- decompress data;
- replace `zstd` or `libzstd`;
- decode literals or sequences inside compressed blocks;
- decode Huffman or FSE tables;
- verify the content checksum against decompressed content;
- train or consume Zstandard dictionaries for decompression;
- provide random access to decompressed content;
- silently recover from arbitrary corruption.

A tolerant/recovery parser may be considered later, but v0.1 is strict by default.

## 4. Functional requirements

### FR-001 — Inspect a byte slice

The library must expose a simple entry point equivalent in purpose to:

```rust
pub fn inspect(data: &[u8]) -> Result<ZstdFile, ZstdError>;
```

The exact API may change before implementation is finalized.

### FR-002 — Parse concatenated frames

The parser must process a stream containing multiple consecutive frames until the input is exhausted.

Supported top-level frame kinds:

- standard Zstandard frame;
- skippable frame.

An unexpected top-level magic number is an error.

### FR-003 — Parse standard-frame magic number

The parser must recognize the standard Zstandard magic number defined by the specification and record the frame's start offset.

### FR-004 — Parse frame header descriptor

The parser must interpret the descriptor fields needed to determine the rest of the frame header:

- frame content size flag;
- single segment flag;
- unused bit;
- reserved bit;
- content checksum flag;
- dictionary ID flag.

A set reserved bit must be rejected as malformed input. The currently unused bit must be preserved or reported but must not be assigned semantics not present in the specification.

### FR-005 — Parse window information

For non-single-segment frames, the parser must read the window descriptor and derive `window_size` according to the Zstandard format specification.

For single-segment frames, the window descriptor is absent and the effective window size is derived from the frame content size.

All arithmetic must be checked.

### FR-006 — Parse dictionary ID

The parser must support dictionary ID field widths specified by the dictionary ID flag and expose the decoded ID.

A decoded dictionary ID of zero must be represented consistently with the specification's meaning that no dictionary ID is specified. The API design must avoid implying that a zero ID proves that no dictionary is required for decompression.

### FR-007 — Parse frame content size

The parser must support all encoded frame-content-size field widths defined by the format, including the special offset rule for the two-byte representation.

If frame content size is absent, the model must represent it as unknown rather than zero.

### FR-008 — Parse block headers

Each standard frame must be parsed through its final block.

For every block the result must expose at least:

- block index;
- byte offset of the block header;
- block type;
- last-block flag;
- the 21-bit declared `Block_Size` value;
- encoded block-content byte length;
- total encoded block byte length including the three-byte header.

Supported block types:

- Raw;
- RLE;
- Compressed.

The reserved block type must be rejected.

The model must distinguish the declared `Block_Size` from encoded content length because RLE blocks encode exactly one content byte while their `Block_Size` represents the decompressed repetition count.

### FR-009 — Respect block-size invariants

The parser should validate block-size constraints that can be checked without decompression, including the frame's maximum block size where the required information is available.

### FR-010 — Parse optional content checksum field

If the frame header indicates a content checksum, the parser must consume and expose the stored 32-bit checksum value and its offset.

ZstdScope v0.1 does not verify the checksum because verification requires the decoded content.

### FR-011 — Parse skippable frames

The parser must recognize all 16 skippable magic-number values in the range defined by the Zstandard specification.

For a skippable frame the model must expose:

- frame start offset;
- exact magic number;
- variant/tag nibble derivable from the magic number;
- declared payload length;
- payload offset;
- total encoded frame length.

The parser does not need to interpret user-defined payload contents.

### FR-012 — Preserve offsets

All major structural elements must include byte offsets sufficient for a future hex viewer to map parsed metadata back to source bytes.

At minimum:

- frame start;
- frame header start/end or encoded length;
- each block header start;
- block content start;
- checksum offset when present;
- skippable payload start.

Offsets are zero-based from the beginning of the inspected input.

### FR-013 — JSON-capable data model

The public model should be designed so that the CLI can serialize inspection results to JSON without duplicating parser logic.

Serialization support may be feature-gated if that keeps the core lightweight.

## 5. CLI requirements

### CLI-001 — Inspect command

The initial CLI shape is:

```text
zstdscope inspect <FILE>
```

It must read the file and print a human-readable structural summary.

### CLI-002 — JSON output

The CLI must support machine-readable output:

```text
zstdscope inspect <FILE> --json
```

The JSON schema is not stable before v1.0 and must be documented as such.

### CLI-003 — Exit behavior

The CLI must return a non-zero exit status for I/O errors or parse failures and print diagnostics to stderr.

## 6. Error requirements

Errors must be typed and must not require consumers to parse human-readable strings to determine the category.

Expected categories include, but are not limited to:

- unexpected end of input;
- invalid top-level magic number;
- reserved frame-header bit set;
- reserved block type;
- invalid or impossible frame-header value;
- invalid block size;
- checked-arithmetic overflow.

Where meaningful, errors must contain the failing zero-based byte offset.

## 7. Safety and resource requirements

### SAFE-001 — No panic on arbitrary input

For any byte sequence, the public parser entry point must return `Ok` or `Err`; malformed input must not cause a panic.

### SAFE-002 — Bounds-checked reads

Parsing code must not perform unchecked indexing into untrusted input.

### SAFE-003 — Checked arithmetic

Offset and size arithmetic must use checked operations whenever attacker-controlled values participate in the calculation.

### SAFE-004 — No payload-sized allocation for inspection

Inspecting a skippable frame or block must not allocate a buffer proportional to its declared payload size merely to skip or describe it.

### SAFE-005 — `unsafe` policy

The initial implementation should use no `unsafe` Rust. Introducing `unsafe` later requires an explicit architecture decision explaining why it is necessary and how its invariants are tested.

## 8. Compatibility requirements

The implementation must be derived from authoritative format sources rather than behavior guessed from sample files.

Primary references:

1. RFC 8878: https://www.rfc-editor.org/rfc/rfc8878.html
2. Current Zstandard format specification: https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md
3. Zstandard reference implementation: https://github.com/facebook/zstd

Any known difference between the RFC and the current reference specification that affects parsing must be documented before the implementation chooses behavior.

## 9. Testing requirements

The parser test suite should cover at least:

- minimal valid standard frame;
- raw block;
- RLE block;
- compressed block treated as opaque content;
- multiple blocks;
- content checksum present;
- each dictionary ID width;
- each frame content size width;
- single-segment and non-single-segment frames;
- concatenated standard frames;
- skippable frames and each valid skippable magic pattern;
- standard and skippable frames mixed in one stream;
- invalid magic;
- truncated frame header;
- truncated block header;
- truncated block content;
- truncated checksum;
- reserved frame-header bit;
- reserved block type;
- malicious size values and overflow boundaries.

Test fixtures generated by the official `zstd` implementation are desirable, but hand-constructed byte fixtures should also be used for edge cases where exact bit-level control matters.

A fuzz target should be added before the first stable release. The core invariant is that arbitrary bytes never cause a parser panic.

## 10. Definition of done for v0.1

v0.1 is ready when:

- the library parses the complete v0.1 frame/header/block scope above;
- the CLI offers text and JSON inspection;
- malformed-input tests cover all parser branches considered security-sensitive;
- supported platforms pass CI;
- public APIs have rustdoc documentation;
- README and architecture documentation match implemented behavior;
- no known parser panic exists for malformed input;
- project licensing has been explicitly selected and added to the repository.
