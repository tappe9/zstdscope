# Releasing ZstdScope

This document defines the manual release procedure. Pull-request CI never receives crates.io credentials.

## Package versioning

The library and CLI use independent versions:

- `zstdscope` versions the public Rust parser API and inspection model;
- `zstdscope-cli` versions command-line behavior and the CLI JSON wire contract.

For project release `v0.3.0`, publish only `zstdscope-cli 0.3.0`. The library remains `zstdscope 0.2.0` because its parser implementation and public Rust API are unchanged.

## Preconditions

1. Start from the latest `main` and confirm the worktree is clean.
2. Confirm the release-preparation PR is merged and `main` CI is green.
3. Confirm `CHANGELOG.md`, README files, package manifests, and `Cargo.lock` describe the intended package versions.
4. Confirm the target package version does not already exist on crates.io.
5. Authenticate locally with a least-privilege crates.io token using Cargo's supported credential mechanism.

Do not reuse or overwrite a published crate version or an existing Git tag.

## Required quality gates

Run from the repository root:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc -p zstdscope --all-features --no-deps --locked
cargo test -p zstdscope --no-default-features --locked
rustup target add wasm32-unknown-unknown
cargo check -p zstdscope --target wasm32-unknown-unknown --no-default-features --locked
cargo package -p zstdscope --list --locked
cargo publish -p zstdscope --dry-run --locked
cargo package -p zstdscope-cli --list --locked
cargo publish -p zstdscope-cli --dry-run --locked
```

The core dry-run remains a regression gate even when the release publishes only the CLI.

## Verify the generated CLI package

```bash
rm -rf target/package /tmp/zstdscope-install
cargo package -p zstdscope-cli --locked
package_dir="$(find target/package -maxdepth 1 -type d -name 'zstdscope-cli-*' -print -quit)"
test -n "$package_dir"
cargo install --path "$package_dir" --locked --root /tmp/zstdscope-install

/tmp/zstdscope-install/bin/zstdscope --version
printf '\x28\xb5\x2f\xfd\x00\x00\x01\x00\x00' > /tmp/minimal.zst
/tmp/zstdscope-install/bin/zstdscope inspect /tmp/minimal.zst
/tmp/zstdscope-install/bin/zstdscope inspect /tmp/minimal.zst --json
```

For `v0.3.0`, the version command must print:

```text
zstdscope 0.3.0
```

The JSON output must contain:

```json
{
  "schema_version": 1
}
```

## Publish `zstdscope-cli 0.3.0`

Reconfirm the exact package contents and then publish:

```bash
cargo publish -p zstdscope-cli --dry-run --locked
cargo publish -p zstdscope-cli --locked
```

Wait for crates.io index propagation, then verify the registry entry:

```bash
cargo info zstdscope-cli@0.3.0
```

Install from crates.io into a clean root and repeat the smoke checks:

```bash
rm -rf /tmp/zstdscope-crates-io-install
cargo install zstdscope-cli --version 0.3.0 --locked --root /tmp/zstdscope-crates-io-install
/tmp/zstdscope-crates-io-install/bin/zstdscope --version
/tmp/zstdscope-crates-io-install/bin/zstdscope inspect /tmp/minimal.zst
/tmp/zstdscope-crates-io-install/bin/zstdscope inspect /tmp/minimal.zst --json
```

## Tag and create the GitHub Release

Tag the exact verified `main` commit; do not create a release from an unmerged branch commit.

```bash
git switch main
git pull --ff-only origin main
git status --short
git tag -a v0.3.0 -m "ZstdScope v0.3.0"
git push origin v0.3.0
```

Create a non-prerelease GitHub Release named `ZstdScope v0.3.0`. Use the `v0.3.0` section of `CHANGELOG.md` as the release notes and explicitly state:

- `zstdscope-cli 0.3.0` is published on crates.io;
- `zstdscope` remains at `0.2.0` because its Rust API and parser behavior are unchanged;
- CLI JSON schema version 1 is a breaking change from the source-built v0.2 CLI output.

After creating the release, verify that the tag targets the expected `main` commit and that the repository's latest-release page resolves to `v0.3.0`.

## Failure handling

- If validation fails before publication, fix the repository and repeat all affected gates.
- If crates.io publication fails before acceptance, correct the local cause and retry the same version only when crates.io confirms it was not published.
- If a published crate is critically broken, do not overwrite it. Consider yanking it, prepare a new patch version, document the reason, and preserve the original tag and release history.
