# Supply-chain and CI policy

Status: **Active**

This document defines the dependency and workflow policy enforced for ZstdScope pull requests and `main`.

## Goals

The policy should detect actionable risk without turning CI into an undocumented or noisy gate. Every required check must have a clear owner, configuration, and exception process.

## GitHub Actions

- Workflow permissions default to `contents: read`.
- Third-party actions are pinned to immutable commit SHAs where practical. A human-readable release tag is retained as an inline comment.
- Superseded pull-request runs are cancelled through workflow concurrency controls; `main` runs are not cancelled.
- Every job has a bounded `timeout-minutes` value.
- Dependabot checks Cargo and GitHub Actions dependencies weekly.

A dependency-update PR must still pass the complete quality and platform matrix. An automated update is not trusted merely because it was opened by Dependabot.

## Cargo dependency policy

`cargo-deny` is the required consolidated gate for:

- RustSec advisories;
- allowed licenses;
- duplicate/wildcard dependency policy;
- registry and Git source policy.

The configuration lives in `deny.toml` and is reviewed like production code.

### Advisories

No advisory is ignored by default. A temporary ignore requires:

1. a linked issue describing reachability and impact;
2. a reason an upgrade or removal is not currently possible;
3. a bounded removal condition or date;
4. explicit review in the pull request that changes `deny.toml`.

`cargo-audit` is not run as a second mandatory gate because `cargo-deny` already consumes the RustSec advisory database. Adding duplicate tooling without a distinct policy would increase noise rather than coverage.

### Licenses

The current allowlist is:

- `Apache-2.0`;
- `MIT`;
- `Unicode-3.0`.

The allowlist reflects the minimum license set needed by the locked dependency graph. A dependency with an `Unlicense OR MIT` expression is accepted through its MIT option; `Unlicense` is not broadly allowlisted. Adding a license requires an explicit `deny.toml` change and review, and CI must not silently broaden acceptance.

### Bans and sources

- Wildcard dependency versions are denied.
- Multiple versions are reported as warnings so they remain visible without blocking justified transitive duplication.
- Unknown registries and unknown Git sources are denied.
- crates.io is the only allowed registry.
- No Git dependency is allowlisted by default.

## Compatibility checks

### MSRV

The workspace declares Rust 1.85.0 as its minimum supported Rust version. CI checks and tests the complete workspace with that toolchain.

### Platform matrix

CI retains stable-Rust tests on:

- Ubuntu;
- Windows;
- macOS.

### WebAssembly

The core parser is compile-checked with:

```bash
cargo check -p zstdscope --target wasm32-unknown-unknown --no-default-features --locked
```

This gate protects the dependency and platform-neutral parser boundary. It does not claim that JavaScript bindings or a browser UI already exist.

### Semantic-version checks

`cargo-semver-checks` was evaluated but is not a mandatory per-PR gate at this stage. ZstdScope is pre-1.0, and not every intentional public-model evolution should be blocked by an automated compatibility verdict.

It should be run during release/API review when a release claims compatibility with an earlier published library version, and it should be reconsidered as a mandatory gate before 1.0. Intentional breaking changes still require documentation and normal review.

## Packaging and distribution checks

CI verifies both workspace packages with locked dependencies:

```bash
cargo package -p zstdscope --list --locked
cargo publish -p zstdscope --dry-run --locked
cargo package -p zstdscope-cli --list --locked
cargo publish -p zstdscope-cli --dry-run --locked
```

The generated CLI package is installed from `target/package` into a temporary root. The resulting binary must report the generated package version and successfully inspect a minimal Standard Frame in text and JSON modes.

Pull-request CI never receives crates.io publish credentials. Actual publication is a deliberate release operation after the exact commit and package contents have been reviewed.

## Required quality gates

The required workflow keeps the existing gates and adds the controls above:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc -p zstdscope --all-features --no-deps --locked
cargo test -p zstdscope --no-default-features --locked
```

A change is not complete merely because one operating system or one feature set passes.

## Exceptions

Security, license, source, or CI exceptions must be narrow and repository-visible. The change must explain:

- the exact rule being relaxed;
- the affected package/action and version;
- the risk assessment;
- why a safer alternative is unavailable;
- how and when the exception will be removed.

Permanent broad suppressions are not an acceptable substitute for resolving the underlying dependency or workflow issue.
