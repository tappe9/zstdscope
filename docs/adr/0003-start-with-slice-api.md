# ADR 0003: Start with an in-memory byte-slice inspection API

- Status: Proposed
- Date: 2026-08-18

## Context

Zstandard is frequently used for large files and streams, so a streaming inspector is desirable long term. However, implementing streaming correctly introduces additional concerns around buffering, partial fields, frame boundaries, lifetimes, error offsets, and API shape.

The first milestone needs a small surface that is easy to test, fuzz, and use from both native Rust and future WebAssembly callers.

## Decision

Start v0.1 with an in-memory API equivalent to:

```rust
pub fn inspect(input: &[u8]) -> Result<ZstdFile, ZstdError>;
```

The parser will eagerly build structural metadata but will not copy opaque block or skippable payload data into the result.

A streaming API is deferred until the structural model and error semantics have proven stable.

## Consequences

### Benefits

- simplest API for callers and fuzz targets;
- parser implementation can focus on format correctness;
- byte offsets are straightforward and deterministic;
- works naturally with browser/WASM byte arrays;
- avoids prematurely designing an abstraction around `Read`, async I/O, or custom sources.

### Costs

- the v0.1 CLI may need to load the complete input file into memory;
- very large files are not an ideal use case until a later streaming/file-backed API exists;
- future streaming support must be added as a new entry point rather than assumed to fall out automatically from v0.1 internals.

## Alternatives considered

### `Read`-based API from the start

Better for large files, but complicates random source-span access and browser use before the model is stable.

### Memory mapping in the CLI

Can reduce copying for large files but adds platform/dependency concerns and does not solve the core API design question. It can be evaluated independently later.

### Generic byte-source trait

Rejected for v0.1 as premature abstraction. A trait should be introduced only after at least two concrete source models demonstrate compatible requirements.

## Follow-up

Track streaming/file-backed inspection as a later roadmap item and ensure v0.1 public model types do not unnecessarily depend on borrowing the input slice.
