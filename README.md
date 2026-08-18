# ZstdScope

ZstdScope is a pure-Rust parser and inspection toolkit for the Zstandard compressed data format.

The project is intentionally focused on **inspection**, not compression or decompression. It exposes encoded structure and source-byte metadata through a reusable Rust API and a CLI, while leaving compressed block payloads opaque.

> Status: the v0.1 structural parser, human-readable CLI, and JSON CLI are implemented on `main`. The project remains pre-1.0 and the public API may still evolve intentionally.

## What ZstdScope inspects

The current v0.1 scope includes:

- Standard and all 16 Skippable Frame magic values;
- concatenated frames with exact frame boundaries;
- Frame Header descriptor fields and derived window size;
- Frame Content Size, including all encoded widths;
- Dictionary ID, preserving an explicitly encoded zero separately from an absent field;
- Raw, RLE, and Compressed block headers and encoded content spans;
- the distinction between RLE declared size and its one-byte encoded content;
- stored content checksum value and span, without claiming checksum verification;
- zero-based source spans for major encoded fields;
- typed, location-aware parse errors for malformed and truncated input.

Parsing literals, sequences, Huffman tables, FSE tables, and other compressed-block internals is intentionally deferred beyond v0.1.

The parser is strict: it requires at least one complete frame and consumes the entire input. Empty input, malformed structures, unknown top-level magic, and trailing partial frames are errors.

## Rust library

The primary API is:

```rust
pub fn inspect(data: &[u8]) -> Result<ZstdFile, ZstdError>;
```

A simple consumer can read bytes however it chooses and pass the slice to the parser:

```rust
use zstdscope::{FrameKind, inspect};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("sample.zst")?;
    let file = inspect(&bytes)?;

    for frame in &file.frames {
        match &frame.kind {
            FrameKind::Standard(standard) => {
                println!(
                    "frame #{}: standard, {} blocks, offset={}, size={}",
                    frame.index,
                    standard.blocks.len(),
                    frame.span.offset,
                    frame.span.length
                );
            }
            FrameKind::Skippable(skippable) => {
                println!(
                    "frame #{}: skippable variant {}, payload={} bytes",
                    frame.index,
                    skippable.variant,
                    skippable.declared_payload_size
                );
            }
        }
    }

    Ok(())
}
```

All public offsets are zero-based byte offsets into the encoded input. Opaque block and Skippable payload bytes are represented by spans rather than copied into the returned model.

### Optional serialization

The core crate keeps serialization optional:

```toml
[features]
default = []
serde = ["dep:serde"]
```

Enabling the `serde` feature adds `Serialize` support to the public inspection model. Parsing-only users do not require Serde.

## CLI

Inspect a file with the human-readable renderer:

```text
zstdscope inspect sample.zst
```

From a repository checkout:

```text
cargo run -p zstdscope-cli -- inspect sample.zst
```

The output reports frame type and boundaries, header metadata, block types and sizes, Skippable payload metadata, and stored checksum metadata when present.

### JSON output

Use `--json` for machine-readable output:

```text
zstdscope inspect sample.zst --json
```

or from the workspace:

```text
cargo run -p zstdscope-cli -- inspect sample.zst --json
```

JSON field names and serialized enum values use explicit `snake_case` representations. `FrameKind` uses a tagged `type` / `data` shape, and Inspector-specific distinctions such as absent versus explicitly encoded zero Dictionary IDs and RLE declared versus encoded sizes are preserved.

I/O and parse failures return a non-zero exit status, write diagnostics to stderr, and do not emit partial-success JSON.

## Workspace

```text
zstdscope/
├── crates/
│   ├── zstdscope/       # Pure Rust parsing library
│   └── zstdscope-cli/   # CLI built on the public library API
├── docs/
└── ARCHITECTURE.md
```

The accepted v0.1 API direction is documented in [Public API design](docs/API-DESIGN.md).

## Design principles

ZstdScope aims to:

- inspect Zstandard structure without decompressing payloads;
- preserve byte-level distinctions useful to inspection and hex-viewer tooling;
- provide precise diagnostics for malformed or unsupported input;
- remain safe on untrusted byte input;
- keep parsing independent from filesystem, terminal, and CLI concerns;
- keep mandatory core dependencies small;
- remain suitable for a future `wasm32` target.

## Non-goals

ZstdScope is not intended to be:

- a compressor;
- a decompressor;
- a replacement for the official `zstd` CLI or `libzstd`;
- a decoder for compressed block internals in v0.1;
- a content-checksum verifier in v0.1.

## Documentation

- [Requirements](docs/REQUIREMENTS.md)
- [Architecture](ARCHITECTURE.md)
- [Zstandard format notes](docs/ZSTD-FORMAT.md)
- [Public API design](docs/API-DESIGN.md)
- [Roadmap](ROADMAP.md)
- [Architecture decision records](docs/adr/)

## Specification sources

ZstdScope is designed against authoritative Zstandard format documentation:

- [RFC 8878 — Zstandard Compression and the `application/zstd` Media Type](https://www.rfc-editor.org/rfc/rfc8878.html)
- [Zstandard reference format specification](https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md)
- [Zstandard reference implementation](https://github.com/facebook/zstd)

Where the current reference specification and the RFC differ, the difference must be documented before implementation behavior is chosen.

## Safety

ZstdScope treats every input byte as untrusted. Parser reads and skips are bounds-checked, offset/size arithmetic is checked, opaque payloads are not copied into the inspection model, and the project forbids authored `unsafe` Rust in the core crate.

See [SECURITY.md](SECURITY.md) for the project security policy.

## Contributing

The project is developed in public. See [CONTRIBUTING.md](CONTRIBUTING.md) for the expected workflow.

## License

ZstdScope is dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)); or
- MIT license ([LICENSE-MIT](LICENSE-MIT)).

You may choose either license when using or redistributing ZstdScope.
