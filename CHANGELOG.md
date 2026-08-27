# Changelog

All notable ZstdScope project releases are documented here.

The reusable `zstdscope` library and the `zstdscope-cli` package use independent versions from the v0.3 project release onward. Each entry states the affected package versions explicitly.

## [v0.3.0] - 2026-08-27

### Package versions

- `zstdscope`: remains at `0.2.0`; no parser or public Rust API release is required.
- `zstdscope-cli`: `0.3.0`; first crates.io release of the CLI package.

### Breaking changes

- `zstdscope inspect --json` now emits an explicit schema-version-1 DTO instead of serializing the public Rust model directly.
- The top-level document includes `"schema_version": 1`.
- Values originating from Rust `u64` fields are decimal strings rather than JSON numbers. This includes input size, byte offsets and lengths, derived window size, and Frame Content Size values.
- Existing JSON consumers must parse those decimal strings explicitly, for example with `BigInt` in JavaScript when exact arithmetic is required.

### Added

- Dedicated private CLI JSON DTOs, decoupled from future Rust model refactors.
- Explicit CLI JSON compatibility and breaking-change policy.
- crates.io packaging metadata for `zstdscope-cli`; the package installs the `zstdscope` binary.
- Package-boundary publish dry-run, install, `--version`, text-output, and JSON smoke tests.
- Immutable GitHub Actions revisions, least-privilege permissions, superseded-PR cancellation, and job timeouts.
- `cargo-deny` advisory, license, dependency-ban, and source checks.
- Weekly Dependabot updates for Cargo and GitHub Actions.
- `wasm32-unknown-unknown` compile-only coverage for the dependency-light core crate.

### Unchanged

- Zstandard parsing and validation behavior.
- Public Rust API of the `zstdscope` library.
- Human-readable CLI output.
- Bounds checking, resource-limit semantics, and malformed-input handling.

## [v0.2.0] - 2026-08-27

- Added configurable frame and block metadata limits through `inspect_with_limits()`.
- Added manual `cargo-fuzz` coverage and successful-parse model-invariant checks.
- Added a default 256 MiB CLI encoded-input guard with `--max-input-bytes` override.
- Defined and verified Rust 1.85.0 as the workspace MSRV.

## [v0.1.0] - 2026-08-18

- Released the initial Pure Rust structural Zstandard inspector library and CLI.
- Added Standard and Skippable Frame parsing, frame-header and block metadata, source spans, typed errors, human-readable output, and the original pre-versioned JSON output.

[v0.3.0]: https://github.com/tappe9/zstdscope/compare/v0.2.0...v0.3.0
[v0.2.0]: https://github.com/tappe9/zstdscope/compare/v0.1.0...v0.2.0
[v0.1.0]: https://github.com/tappe9/zstdscope/releases/tag/v0.1.0
