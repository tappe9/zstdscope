# ADR 0006: Distribute the CLI as a crates.io package

- Status: Accepted
- Date: 2026-08-27

## Context

The reusable `zstdscope` library is published on crates.io, while the `zstdscope-cli` workspace package originally used `publish = false`. Users therefore needed a repository checkout to run the command.

The evaluated channels were:

1. publish the CLI crate and install it with Cargo;
2. publish prebuilt GitHub Release binaries;
3. support both.

Prebuilt binaries would require a maintained target matrix, reproducible release automation, checksums, signing/provenance policy, and asset lifecycle management. Those controls are not yet present.

## Decision

### Primary release channel

Publish `zstdscope-cli` on crates.io. The package name remains `zstdscope-cli`, while the installed executable remains `zstdscope`:

```bash
cargo install zstdscope-cli --locked
```

Before the first CLI crate release is published, users may install the current repository revision without a manual checkout:

```bash
cargo install --git https://github.com/tappe9/zstdscope zstdscope-cli --locked
```

### Package boundary

The library and CLI remain separate crates:

- `zstdscope` owns parsing, validation, and the public inspection model;
- `zstdscope-cli` owns filesystem input policy, CLI arguments, presentation, JSON DTOs, diagnostics, and process exit behavior.

The CLI package depends on a released `zstdscope` version as well as the workspace path, allowing workspace development and a self-contained crates.io package.

### Supported platforms

The supported source-build matrix is the set continuously exercised by CI:

- Ubuntu x86_64;
- Windows x86_64 using MSVC;
- macOS arm64.

Other Rust-supported targets are best effort. This decision does not promise prebuilt artifacts for any target.

### Prebuilt binaries

GitHub Release binaries are deferred. They may be added by a later ADR only after automation defines:

- supported target triples;
- reproducible build inputs;
- checksums for every downloadable asset;
- signing or provenance policy;
- release failure and revocation handling.

## Consequences

### Positive

- Users get a conventional Rust installation path without manually cloning the repository.
- The release mechanism reuses Cargo's source-build and dependency-resolution model.
- The project avoids prematurely maintaining unsigned, manually assembled binaries.
- Library and CLI responsibilities remain separated.

### Trade-offs

- Users need a Rust toolchain and compile the CLI locally.
- Installation time is longer than downloading a prebuilt executable.
- The CLI crate must be published after its exact `zstdscope` dependency version exists on crates.io.

## Verification

CI performs all of the following from the package boundary:

```bash
cargo package -p zstdscope-cli --list --locked
cargo publish -p zstdscope-cli --dry-run --locked
cargo package -p zstdscope-cli --locked
cargo install --path target/package/zstdscope-cli-<version> --locked
```

The installed artifact must pass:

- `zstdscope --version`;
- human-readable `zstdscope inspect`;
- `zstdscope inspect --json` with `schema_version` 1.

Publishing credentials and the actual crates.io release remain an explicit release operation; pull-request CI never receives publish credentials.
