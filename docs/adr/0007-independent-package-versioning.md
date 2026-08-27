# ADR 0007: Version library and CLI packages independently

- Status: Accepted
- Date: 2026-08-27

## Context

The workspace originally inherited one package version for both `zstdscope` and `zstdscope-cli`. That was convenient while the CLI was not published, but the two packages now have different compatibility contracts and can change independently:

- `zstdscope` exposes the reusable Rust parsing API and inspection model;
- `zstdscope-cli` exposes command-line behavior and the versioned CLI JSON wire contract.

The post-v0.2 work changes the CLI JSON contract and makes the CLI publishable, but it does not change the parser, validation behavior, or public Rust API in the library crate. Publishing an unchanged `zstdscope` version solely to keep package numbers aligned would create an unnecessary library release.

A unified workspace version also complicates pre-publication validation. A CLI package pinned to an unreleased new library version cannot complete package-boundary installation and `cargo publish --dry-run` verification until that library version already exists on crates.io.

## Decision

### Independent package versions

`zstdscope` and `zstdscope-cli` use explicit, independent package versions.

For the v0.3 project release:

- `zstdscope` remains at `0.2.0` because its Rust API and parser implementation are unchanged;
- `zstdscope-cli` is released as `0.3.0` because its machine-readable JSON contract intentionally changes and it becomes available through crates.io for the first time.

The CLI continues to depend on the released `zstdscope = 0.2.0` API that it actually requires.

### Release tags and notes

Repository-level Git tags and GitHub Releases describe the project release milestone. Release notes must state the exact crate versions included or intentionally unchanged whenever package versions differ.

A later change may update one package or both. Versions should be aligned only when the compatibility impact and released content justify alignment; numerical alignment is not itself a release requirement.

### Compatibility signaling

Each package follows its own pre-1.0 compatibility boundary:

- Rust API compatibility is communicated by the `zstdscope` crate version;
- CLI behavior and JSON wire compatibility are communicated by the `zstdscope-cli` package and `zstdscope --version`;
- JSON breaking changes additionally require a new `schema_version` as defined by ADR 0005.

## Consequences

### Positive

- Published versions correspond to actual changes in each package.
- The unchanged library does not receive a meaningless release.
- CLI packaging, installation, and publish dry-runs can be validated before publication using an already released library dependency.
- Future library-only and CLI-only releases remain straightforward.

### Trade-offs

- A repository release may contain different library and CLI version numbers.
- Documentation and release notes must name package versions explicitly.
- Workspace manifests no longer inherit one shared version field.

## Verification

Release preparation must verify:

```bash
cargo metadata --format-version 1 --locked
cargo package -p zstdscope-cli --list --locked
cargo publish -p zstdscope-cli --dry-run --locked
```

The generated CLI package must install successfully and report the expected CLI package version through `zstdscope --version`.
