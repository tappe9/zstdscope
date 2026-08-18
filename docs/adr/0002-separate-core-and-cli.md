# ADR 0002: Separate the parser library from the CLI

- Status: Accepted
- Date: 2026-08-18

## Context

ZstdScope is expected to serve several kinds of consumers over time: terminal users, Rust applications, a future WebAssembly interface, browser tooling, and possibly editor integrations.

If parsing, file I/O, terminal formatting, and command-line behavior are implemented in one crate, future consumers either depend on unnecessary CLI concerns or duplicate parser logic.

## Decision

Use a Cargo workspace with separate crates:

```text
crates/zstdscope
crates/zstdscope-cli
```

`zstdscope` is the reusable parser and model library.

`zstdscope-cli` is a thin application layer that:

- reads files;
- parses CLI arguments;
- invokes the public `zstdscope` API;
- renders text or JSON;
- maps errors to process exit behavior.

The CLI must not contain an independent Zstandard parser.

## Consequences

### Benefits

- third-party Rust users depend only on the parser library;
- the parser can be fuzzed independently;
- a future WASM layer can reuse the same model;
- parser correctness and presentation logic remain independently testable;
- CLI redesign does not require parser API redesign.

### Costs

- workspace/release configuration is slightly more involved;
- CLI-only features may require optional library features such as serialization;
- care is required to keep presentation helpers from leaking into the core crate.

## Alternatives considered

### Single binary crate

Simpler initially, but makes the core parser difficult to reuse and encourages future duplication.

### Single library crate with a binary target

Viable for small projects, but separate packages provide a clearer dependency boundary if the library is later published independently or consumed from WASM tooling.

## Follow-up

The initial implementation must create both crates before parser work begins and establish CI that checks the whole workspace.
