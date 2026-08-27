# ZstdScope

[![Crates.io](https://img.shields.io/crates/v/zstdscope.svg)](https://crates.io/crates/zstdscope)
[![docs.rs](https://docs.rs/zstdscope/badge.svg)](https://docs.rs/zstdscope)
[![CI](https://github.com/tappe9/zstdscope/actions/workflows/ci.yml/badge.svg)](https://github.com/tappe9/zstdscope/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/zstdscope.svg)](https://github.com/tappe9/zstdscope#license)

ZstdScope is a pure-Rust parser and structural inspection toolkit for the Zstandard compressed data format.

The project is intentionally focused on **inspection**, not compression or decompression. It exposes encoded structure and source-byte metadata through a reusable Rust API and a CLI, while leaving compressed block payloads opaque.

> **v0.2.0 is available on [crates.io](https://crates.io/crates/zstdscope).** ZstdScope remains pre-1.0, so the Rust API and JSON representation may still evolve intentionally.

**Links:** [crates.io](https://crates.io/crates/zstdscope) · [docs.rs](https://docs.rs/zstdscope) · [v0.2.0 release](https://github.com/tappe9/zstdscope/releases/tag/v0.2.0)

## Installation

Add ZstdScope to a Rust project:

```bash
cargo add zstdscope
```

Or add it manually to `Cargo.toml`:

```toml
[dependencies]
zstdscope = "0.2"
```

For optional Serde serialization support:

```bash
cargo add zstdscope --features serde
```

## What ZstdScope inspects

The current v0.1 scope includes:

- Standard and all 16 Skippable Frame magic values;
- concatenated frames with exact frame boundaries;
- Frame Header descriptor fields and derived window size;
- Frame Content Size, including all encoded widths and contradictions detectable from block-level decoded-size bounds;
- Dictionary ID, preserving an explicitly encoded zero separately from an absent field;
- Raw, RLE, and Compressed block headers and encoded content spans;
- the distinction between RLE declared size and its one-byte encoded content;
- structural rejection of Compressed blocks too small to contain their mandatory outer section headers;
- stored content checksum value and span, without claiming checksum verification;
- zero-based source spans for major encoded fields;
- typed, location-aware parse errors for malformed and truncated input.

Parsing literals, sequences, Huffman tables, FSE tables, and other compressed-block internals is intentionally deferred beyond v0.1.

The parser is strict for the structural envelope implemented by v0.1: it requires at least one complete frame and consumes the entire input. Empty input, malformed structures that v0.1 can validate, unknown top-level magic, reserved encodings, impossible structural sizes, and trailing partial frames are errors. Compressed-block internals and content-checksum validity are intentionally not validated in v0.1.

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

### Configurable resource limits

`inspect()` intentionally preserves the simple, unlimited frame/block-count behavior. Applications that inspect untrusted or externally supplied inputs can instead apply explicit metadata budgets with `inspect_with_limits()`:

```rust
use zstdscope::{InspectionLimits, inspect_with_limits};

let limits = InspectionLimits {
    max_frames: 1_024,
    max_blocks_per_frame: 2_048,
    max_total_blocks: 100_000,
};

let file = inspect_with_limits(&bytes, limits)?;
```

The values above are examples, not universal safe defaults; choose limits for the application's expected workload. A count equal to the configured maximum is accepted. Attempting to parse one more affected frame or block returns the typed `ZstdError::ResourceLimitExceeded` at the offset where that structure would begin.

These limits bound metadata counts only. They do not cap the size of the caller-owned input slice and do not make the in-memory API streaming. Block and Skippable payloads continue to be skipped without payload-sized copies.

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

### Large input files

The CLI still uses the in-memory library API, so an accepted input file is resident in memory while it is inspected. To keep the default CLI behavior bounded, `zstdscope inspect` rejects encoded input larger than **268,435,456 bytes (256 MiB)** before parsing.

Raise or lower that boundary explicitly with `--max-input-bytes`:

```text
zstdscope inspect large.zst --max-input-bytes 1073741824
```

Raising the limit also raises the maximum memory commitment for the encoded input buffer. The CLI checks the file size before the full read when possible and also bounds the actual read, so a file that grows while being read cannot silently bypass the configured limit.

This CLI byte limit is separate from `inspect_with_limits()`, which only limits frame/block metadata counts in the library. Neither mechanism makes inspection streaming. A future streaming/file-backed library API remains the path for files that should not be held fully in memory.

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

I/O and parse failures return a non-zero exit status, write diagnostics to stderr, and do not emit partial-success JSON. An input that exceeds the CLI byte limit also returns a non-zero exit status with a structured CLI error. Output write failures are handled without panicking; a downstream process closing a pipe normally is treated as a normal CLI termination.

### Release distribution

The reusable `zstdscope` Rust library is published on [crates.io](https://crates.io/crates/zstdscope), with API documentation on [docs.rs](https://docs.rs/zstdscope).

The `zstdscope-cli` package currently has `publish = false`; CLI binary distribution is intentionally separate and may be added through GitHub Releases or another documented channel later.

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
- remain safe on untrusted byte input within the documented in-memory/resource model;
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
- [Fuzzing guide](FUZZING.md)
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

The public parser APIs remain intentionally in-memory. `inspect_with_limits()` can bound frame/block metadata counts for untrusted inputs, while `inspect()` retains the original unlimited count behavior. The CLI adds a separate default 256 MiB encoded-input guard with `--max-input-bytes` as an explicit override, but any accepted CLI input is still held fully in memory. Streaming/file-backed inspection remains later hardening work.

Parser fuzzing is available through `cargo-fuzz`; successful fuzz parses are also checked against structural model invariants. See [FUZZING.md](FUZZING.md) for setup, execution, and regression-handling instructions. Fuzzing is manual initially and is not part of normal pull-request CI.

See [SECURITY.md](SECURITY.md) for the project security policy.

## Contributing

The project is developed in public. See [CONTRIBUTING.md](CONTRIBUTING.md) for the expected workflow.

## License

ZstdScope is dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)); or
- MIT license ([LICENSE-MIT](LICENSE-MIT)).

You may choose either license when using or redistributing ZstdScope.
