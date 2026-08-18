# ADR 0001: Implement the format parser in Pure Rust

- Status: Proposed
- Date: 2026-08-18

## Context

ZstdScope exists to inspect the encoded structure of Zstandard streams. The official Zstandard project already provides a mature C reference implementation and APIs that can expose some frame information.

Calling `libzstd` through FFI would reduce the amount of format parsing code ZstdScope needs to own, but it would also make the project primarily a wrapper around an existing implementation. That would limit its usefulness as a transparent structural inspector and complicate some portability and source-mapping goals.

## Decision

Implement the v0.1 Zstandard structural parser directly in Rust from the published format specification.

The core crate will not depend on:

- `libzstd`;
- `zstd-sys`;
- C or C++ FFI.

The initial parser should contain no project-authored `unsafe` Rust.

Reference implementation behavior may be used for comparison and fixture generation, but it is not the parsing engine.

## Consequences

### Benefits

- ZstdScope can expose fields and byte spans chosen for inspection rather than decoder API convenience.
- The core remains easier to target to WebAssembly.
- Parser behavior is explicit and testable at the bit/byte level.
- The project has independent technical value rather than being only a thin binding.
- Fuzzing can directly exercise the parser's own safety boundaries.

### Costs

- ZstdScope owns correctness for the format subset it implements.
- Specification changes and clarifications must be tracked.
- More tests are required than for a thin wrapper.
- The project must avoid accidentally becoming an incomplete decompressor without a clear reason.

## Alternatives considered

### Wrap `libzstd`

Useful for mature decompression and compression, but a poor fit for the project's primary goal of detailed structural inspection and independent byte-level source mapping.

### Fork the Zstandard reference implementation

Rejected because ZstdScope does not need to maintain a compressor/decompressor implementation and should not inherit unrelated complexity.

### Use an existing Rust decoder as the parser core

Potentially useful as a reference, but decoder implementations commonly consume internal parsing state rather than expose a stable inspection model. Depending directly on decoder internals would also couple ZstdScope's API to another project's implementation choices.

## Follow-up

Before implementation begins, confirm this ADR in review and add specification-linked tests for every bit-field rule implemented in v0.1.
