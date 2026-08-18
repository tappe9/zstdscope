# Issue #9 coverage matrix

This matrix maps the durable fixture corpus to the v0.1 parser paths required by Issue #9.

| Area | Fixture / test coverage |
|---|---|
| FR-001 inspect entry point | all integration fixture tests call `inspect()` |
| FR-002 concatenated frames | `mixed_standard_skippable_standard`, existing concatenation tests |
| FR-003 Standard magic | all Standard fixtures |
| FR-004 descriptor flags | FCS width cases, Single Segment, checksum, reserved-bit error |
| FR-005 window information | non-Single Segment fixtures use Window Descriptor `0x00`; Single Segment fixture omits it |
| FR-006 Dictionary ID fidelity | absent, 1-byte explicit zero, 2-byte, 4-byte fixtures |
| FR-007 Frame Content Size | absent plus 1/2/4/8-byte encoded widths |
| FR-008 block headers | Raw, RLE, Compressed, multiple-block fixtures |
| FR-009 block-size invariant | `invalid_block_size` |
| FR-010 content checksum | hand-built checksum plus reference-generated checksum frame |
| FR-011 Skippable frames | mixed stream and existing all-16-variant coverage |
| FR-012 source spans | integration tests assert frame/header/field/block/checksum/Skippable spans |
| malformed input | named malformed catalog cases assert typed errors and offsets |

Reference-generated fixtures prove interoperability with official `zstd` output. Hand-built fixtures provide deterministic bit-level coverage where compressor output is not suitable for controlling exact field widths or malformed boundaries.
