# ZstdScope

ZstdScope is a proposed pure-Rust parser and inspection toolkit for the Zstandard compressed data format.

The project is intentionally focused on **inspection**, not compression or decompression. Its goal is to expose the structure and metadata of Zstandard streams in a safe, reusable API that can power a CLI, future WebAssembly tooling, and other developer tools.

> Status: design phase. No parser implementation is included yet.

## Goals

ZstdScope aims to:

- parse Zstandard streams without decompressing their payloads;
- expose standard frames, skippable frames, frame headers, block headers, byte offsets, and sizes;
- provide precise diagnostics for malformed or unsupported input;
- remain safe on untrusted byte input and avoid panics from malformed data;
- keep the parsing core independent from the CLI and future user interfaces;
- remain suitable for a future `wasm32` target.

## Non-goals

ZstdScope is not intended to be:

- a compressor;
- a decompressor;
- a replacement for the official `zstd` CLI or `libzstd`;
- a complete decoder for compressed block internals in the first release.

## Planned v0.1 scope

The first implementation milestone is expected to inspect:

- Zstandard standard-frame magic numbers;
- skippable-frame magic numbers;
- frame header descriptor fields;
- window size;
- frame content size;
- dictionary ID;
- content checksum presence and stored checksum value;
- block headers;
- raw, RLE, and compressed block types;
- last-block markers;
- frame and block byte offsets and encoded sizes;
- concatenated frames.

Parsing literals, sequences, Huffman tables, and FSE tables inside compressed blocks is explicitly deferred beyond v0.1.

## Proposed workspace

```text
zstdscope/
├── crates/
│   ├── zstdscope/       # Pure Rust parsing library
│   └── zstdscope-cli/   # CLI built on the library
├── docs/
└── ARCHITECTURE.md
```

The initial public API is expected to center on a function similar to:

```rust
pub fn inspect(data: &[u8]) -> Result<ZstdFile, ZstdError>;
```

The API is still under design and is not yet stable.

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

## Contributing

The project is being designed in public. See [CONTRIBUTING.md](CONTRIBUTING.md) for the expected workflow.

## License

A project license has not been selected yet. A license must be chosen before the first distributable release.
