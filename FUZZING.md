# Fuzzing ZstdScope

ZstdScope uses `cargo-fuzz` to exercise the public `zstdscope::inspect` parser entry point with arbitrary byte input.

The `inspect` fuzz target also validates structural invariants whenever parsing succeeds, including frame/block indexes, source-span bounds and ordering, complete frame coverage, and final-block semantics.

## Prerequisites

`cargo-fuzz` requires a Rust nightly toolchain and LLVM's libFuzzer support.

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
```

## Run locally

From the repository root:

```bash
cargo +nightly fuzz run inspect
```

For a bounded local run:

```bash
cargo +nightly fuzz run inspect -- -max_total_time=60
```

To reproduce one saved input:

```bash
cargo +nightly fuzz run inspect fuzz/artifacts/inspect/<artifact>
```

## Regression policy

If fuzzing finds a parser panic or model-invariant failure:

1. minimize the reproducer with `cargo +nightly fuzz tmin inspect <artifact>`;
2. add the minimized bytes to the normal Rust test suite as a permanent regression test;
3. fix the parser or invariant violation with that regression test in place;
4. rerun the fuzz target to confirm the original crash no longer reproduces.

Do not rely only on the fuzz corpus for known regressions; permanent regressions belong in the normal test suite so standard CI covers them.

## Automation policy

Fuzzing starts as a **manual** developer workflow. It is intentionally not part of the normal pull-request CI because fuzz duration is open-ended and the existing quality/platform jobs should remain fast and deterministic.

A scheduled or continuous fuzzing service can be added separately once corpus growth, runtime, and operational value are understood.
