# ZstdScope

ZstdScope is a project for a pure-Rust parser and inspection toolkit for the Zstandard compressed data format.

The project is intentionally focused on **inspection**, not compression or decompression. Its goal is to expose the structure and metadata of Zstandard streams in a safe, reusable API that can power a CLI, future WebAssembly tooling, and other developer tools.

> Status: the initial v0.1 requirements, architecture, and public API direction are accepted. Parser implementation has not started yet.

## Goals

ZstdScope aims to:

- parse Zstandard streams without decompressing their payloads;
- expose standard frames, skippable frames, frame headers, block headers, byte offsets, and sizes;
- preserve byte-level distinctions useful to inspection tools;
- provide precise diagnostics for malformed or unsupported input;
- remain safe on untrusted byte input and avoid parser panics from malformed data;
- keep the parsing core independent from the CLI and future user interfaces;
- remain suitable for a future `wasm32` target.

## Non-goals

ZstdScope is not intended to be:

- a compressor;
- a decompressor;
- a replacement for the official `zstd` CLI or `libzstd`;
- a complete decoder for compressed block internals in the first release.

## Planned v0.1 scope

The first implementation milestone will inspect:

- Zstandard standard-frame magic numbers;
- skippable-frame magic numbers;
- frame header descriptor fields;
- window size;
- frame content size;
- Dictionary ID, preserving an explicitly encoded zero separately from an absent field;
- content checksum presence and stored checksum value;
- block headers;
- Raw, RLE, and Compressed block types;
- last-block markers;
- frame, header-field, and block source spans;
- concatenated frames.

Parsing literals, sequences, Huffman tables, and FSE tables inside compressed blocks is explicitly deferred beyond v0.1.

The strict v0.1 parser requires at least one complete frame. Empty input, truncated structures, unknown top-level magic, and trailing partial frames are parse errors.

## Workspace

```text
zstdscope/
├── crates/
│   ├── zstdscope/       # Pure Rust parsing library
│   └── zstdscope-cli/   # CLI built on the library
├── docs/
└── ARCHITECTURE.md
```

The initial public API centers on:

```rust
pub fn inspect(data: &[u8]) -> Result<ZstdFile, ZstdError>;
```

The accepted v0.1 API direction is documented in [Public API design](docs/API-DESIGN.md). The project remains pre-1.0, so future breaking changes may still occur intentionally and with documentation.

## JSON

The CLI will support machine-readable inspection output with `--json`.

JSON field names and serialized enum values use `snake_case`. The representation is explicitly defined and tested rather than accidentally depending on serialization-library defaults.

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

ZstdScope treats every input byte as untrusted. The initial parser design requires bounds-checked reads, checked offset/size arithmetic, no opaque payload copies, and no project-authored `unsafe` Rust.

See [SECURITY.md](SECURITY.md) for the project security policy.

## Contributing

The project is developed in public. See [CONTRIBUTING.md](CONTRIBUTING.md) for the expected workflow.

## License

ZstdScope is dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)); or
- MIT license ([LICENSE-MIT](LICENSE-MIT)).

You may choose either license when using or redistributing ZstdScope.
