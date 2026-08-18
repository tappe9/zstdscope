# Contributing to ZstdScope

Thank you for considering a contribution.

ZstdScope is currently in its design and early implementation phase. Correctness and safety matter more than feature count.

## Development principles

Contributions should preserve these project principles:

- derive parser behavior from authoritative Zstandard format documentation;
- keep the parser core independent from CLI and UI concerns;
- treat input bytes as untrusted;
- prefer explicit checked parsing over clever bit manipulation;
- avoid `unsafe` in project code unless an accepted ADR justifies it;
- preserve byte-level encoding details that are useful to inspection consumers;
- keep mandatory core dependencies minimal;
- do not add decompression behavior unless the project scope is explicitly expanded;
- add tests for every format edge case or bug fix.

## Before implementing a format change

For behavior that changes parsing semantics:

1. identify the relevant rule in RFC 8878 and/or the current Zstandard format specification;
2. check whether the reference specification and RFC materially differ;
3. describe the intended behavior in an issue or PR;
4. add a focused test that demonstrates the rule;
5. update architecture or format notes when the change affects documented behavior.

Primary references:

- https://www.rfc-editor.org/rfc/rfc8878.html
- https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md

## Pull requests

Keep pull requests focused. A good parser PR should explain:

- which part of the format is implemented or changed;
- the specification rule being followed;
- malformed-input behavior;
- tests added;
- whether the public model or error API changes.

Avoid mixing large formatting/refactoring changes with parser semantic changes unless necessary.

## Validation

Once the Rust workspace exists, PRs are expected to pass the project checks, anticipated to include:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

Additional fuzz or fixture validation commands will be documented when those targets are added.

## Public API changes

Before v1.0 the API may evolve, but breaking changes should still be intentional and explained.

Do not expose internal parser types merely to make implementation convenient. Public types should serve inspection consumers.

Accepted API policy decisions are recorded in the ADRs under `docs/adr/`. Changing an accepted policy should be proposed as a new ADR rather than silently diverging from the documented design.

## Contribution licensing

ZstdScope is licensed under **MIT OR Apache-2.0**. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in ZstdScope is provided under the same dual-license terms, without additional conditions.

See `LICENSE-MIT` and `LICENSE-APACHE` for the full license texts.

## Security issues

Do not open a public issue for a vulnerability that could materially affect users. Follow [SECURITY.md](SECURITY.md).
