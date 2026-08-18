# Parser fixture corpus

This directory contains durable v0.1 parser fixtures for end-to-end integration tests.

Fixtures are classified by what they prove. A test must not treat an envelope-only hand-built sample as evidence that the complete compressed payload or checksum is decoder-valid.

## Reference-generated, fully valid fixtures

Files under `reference/` were generated with the official Zstandard CLI:

```text
*** Zstandard CLI (64-bit) v1.5.7, by Yann Collet ***
```

They are stored as lowercase hexadecimal text rather than binary so changes remain reviewable in Git. Tests decode the hex to bytes before calling `zstdscope::inspect()`; they do not decompress the payload.

These samples are intended to be fully valid Zstandard frames and may be revalidated with the official CLI using `zstd --test` after regeneration. The published format specification remains authoritative because the reference decoder can intentionally accept some non-conforming inputs.

### `reference/raw-no-checksum.zst.hex`

Source bytes are the UTF-8 string:

```text
ZstdScope reference fixture: no checksum\n
```

Regeneration:

```bash
printf 'ZstdScope reference fixture: no checksum\n' > /tmp/zstdscope-reference-raw.txt
zstd -q -f --no-check /tmp/zstdscope-reference-raw.txt \
  -o /tmp/reference-raw-no-checksum.zst
zstd --test /tmp/reference-raw-no-checksum.zst
python3 -c 'from pathlib import Path; print(Path("/tmp/reference-raw-no-checksum.zst").read_bytes().hex())'
```

This fixture is a Single Segment frame with one Raw block and no content checksum.

### `reference/compressed-checksum.zst.hex`

Source bytes are the ASCII phrase `ZstdScope reference compressed fixture. ` repeated 256 times (10,240 bytes).

Regeneration:

```bash
python3 - <<'PY'
from pathlib import Path
Path('/tmp/zstdscope-reference-compressed.txt').write_bytes(
    b'ZstdScope reference compressed fixture. ' * 256
)
PY
zstd -q -f /tmp/zstdscope-reference-compressed.txt \
  -o /tmp/reference-compressed-checksum.zst
zstd --test /tmp/reference-compressed-checksum.zst
python3 -c 'from pathlib import Path; print(Path("/tmp/reference-compressed-checksum.zst").read_bytes().hex())'
```

This fixture is a Single Segment frame with a Compressed block and a stored content checksum.

## Hand-built structural fixtures

`hand-built.hex` is a named catalog of exact byte sequences assembled from the documented frame/header/block bit layout. Each entry exists to exercise an Inspector-visible distinction or malformed-input boundary that is difficult to guarantee through compressor-generated data.

The catalog contains three kinds of entries:

- complete hand-built frames whose outer structure and known decoded-size invariants are internally consistent;
- structural-envelope samples such as `compressed_block_opaque` and `checksum`, where v0.1 intentionally does not claim compressed-payload validity or checksum validity;
- explicitly malformed/boundary samples expected to return typed errors.

The FCS-width complete-frame fixtures use an RLE block whose decoded repetition count matches the declared Frame Content Size, so field-width tests no longer rely on frames that contradict their own FCS.

The catalog covers:

- Dictionary ID field widths 0/1/2/4 bytes, including explicitly encoded zero;
- Frame Content Size widths 0/1/2/4/8 bytes and Single Segment vs non-Single Segment;
- Raw, RLE, structurally opaque Compressed, multiple-block, checksum, and mixed-frame cases;
- malformed header/block/checksum/Skippable/trailing-input cases;
- representative block-size and declared-payload boundary cases.

See `docs/ZSTD-FORMAT.md` sections 4–15 for the bit-level rules these fixtures encode.
