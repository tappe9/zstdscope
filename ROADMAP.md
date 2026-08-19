# ZstdScope Roadmap

This roadmap is directional. Milestones may change as implementation and specification review uncover better boundaries.

## v0.1 — Structural inspector MVP

Status: **completed and released as v0.1.0.**

Goal: provide a safe Pure Rust parser for top-level Zstandard structure plus a usable CLI.

Implemented scope:

- Cargo workspace with `zstdscope` and `zstdscope-cli` crates;
- bounds-checked cursor/reader abstraction;
- standard Zstandard frame recognition;
- frame-header descriptor parsing;
- window-size derivation;
- dictionary ID parsing;
- frame content size parsing;
- Raw/RLE/Compressed block-header parsing;
- last-block handling;
- stored content-checksum field inspection;
- skippable-frame support;
- concatenated-frame support;
- byte spans and offsets in the public model;
- typed location-aware errors;
- `zstdscope inspect <FILE>`;
- `zstdscope inspect <FILE> --json`;
- malformed-input tests;
- reference-generated valid fixtures;
- GitHub Actions for format, lint, test, and documentation checks;
- `MIT OR Apache-2.0` project licensing.

Not included:

- decompression;
- literals/sequence parsing;
- Huffman/FSE inspection;
- checksum verification;
- streaming file inspection;
- a continuous fuzz target in v0.1.

The parser's malformed-input safety invariant is covered by targeted boundary and integration tests in v0.1. Continuous fuzzing remains a hardening item before a stable release.

## v0.2 — Inspector ergonomics and robustness

Status: **in progress on `main`; not yet released.**

Already implemented on `main` after v0.1.0:

- configurable parser resource limits;
- a manual `cargo-fuzz` target with structural model-invariant checks on successful parses;
- bounded CLI file reads with a default 256 MiB encoded-input guard and explicit `--max-input-bytes` override.

Remaining candidate scope:

- richer field-level spans for hex-view mapping;
- more detailed diagnostics;
- streaming or file-backed library inspection API for very large files;
- scheduled or continuous fuzzing automation;
- JSON schema documentation;
- CLI output refinements.

## v0.3 — Compressed-block structural metadata

Candidate scope:

- Literals Section inspection;
- Sequences Section inspection;
- nested spans within Compressed blocks;
- structural validation that does not require full decompression.

This milestone should not be started until the v0.1 frame/block model proves extensible enough to represent nested structures cleanly.

## v0.4 — Entropy metadata inspection

Candidate scope:

- Huffman table metadata;
- FSE table metadata;
- visualization-oriented models for entropy structures.

## v0.5 — WebAssembly and browser inspector

Candidate scope:

- WASM bindings for the parser;
- browser-local file inspection;
- no server upload required;
- tree view of frames and blocks;
- hex/source mapping using byte spans;
- malformed-field highlighting.

The Web UI should remain a consumer of the same `zstdscope` core rather than a separate parser.

## v0.6 — Analysis and benchmark tooling

Candidate scope:

- integrate inspection metadata with Zstd benchmark results;
- compare encoded structure across compression levels;
- machine-readable benchmark result format;
- recommendation experiments for compression settings.

Benchmark execution may require a native component to avoid presenting browser/WASM timing as equivalent to native `libzstd` performance.

## v1.0 — Stable inspection API

Potential criteria:

- stable documented Rust public API;
- documented compatibility policy;
- stable or explicitly versioned JSON representation;
- robust fuzz coverage;
- large corpus of valid and malformed tests;
- mature diagnostics;
- clear support policy for Zstandard format revisions;
- published crate and release artifacts.

## Guiding rule

Prefer small, independently useful milestones. Do not turn ZstdScope into a decompressor merely because deeper inspection eventually requires understanding decoder structures.
