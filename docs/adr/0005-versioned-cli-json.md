# ADR 0005: Versioned CLI JSON DTO boundary

- Status: Accepted
- Date: 2026-08-27

## Context

The original `zstdscope inspect --json` implementation serialized the public Rust inspection model directly. Explicit Serde attributes and black-box tests made that representation predictable, but they did not separate two independently evolving contracts:

```text
public Rust model ──► CLI JSON wire format
```

A Rust API refactor could therefore become an accidental JSON breaking change. The public model also contains `u64` offsets, lengths, sizes, and Frame Content Size values that cannot always be represented exactly by a JavaScript `Number`.

ZstdScope needs an intentional machine-readable boundary before browser, editor, and scripting consumers become more numerous.

## Decision

### Dedicated private DTOs

The CLI owns private `Json*V1` DTOs and maps the public `ZstdFile` model into them. The parser core does not own CLI JSON formatting.

The core crate may continue to expose optional Serde support for consumers that deliberately serialize the Rust model. That representation is not the CLI JSON compatibility contract.

### Version discriminator

Every successful CLI JSON document includes:

```json
{
  "schema_version": 1
}
```

### Integer representation

All values originating from Rust `u64` fields are encoded as decimal strings. This includes input size, byte-span offsets and lengths, derived window size, and Frame Content Size.

Values whose source types are bounded `u8`, `u32`, or `usize` remain JSON numbers in schema version 1.

### Structural fidelity

The DTO must preserve inspector-specific distinctions:

- absent Dictionary ID versus an explicitly encoded zero;
- RLE declared size versus its one-byte encoded content size;
- explicit Standard versus Skippable frame tags;
- source spans for encoded fields.

### Compatibility policy

Before 1.0, backward-compatible additive fields may be introduced without changing `schema_version` when existing consumers that ignore unknown fields remain valid.

The following changes are breaking and require a new schema version plus release-note documentation:

- removing or renaming a field;
- changing a field's JSON type;
- changing enum names, tags, or tagged-union shape;
- changing the meaning or units of an existing field;
- changing the decimal-string convention for `u64` values;
- erasing an inspector-specific distinction represented by schema version 1.

The human-readable renderer is a separate presentation contract and must not change merely because DTO internals are refactored.

## Consequences

### Positive

- Rust API evolution no longer changes CLI JSON automatically.
- JavaScript consumers can preserve every `u64` value exactly.
- Schema changes become deliberate, reviewable, and testable.
- The parser core remains independent from CLI wire-format policy.

### Trade-offs

- Mapping code duplicates the shape of selected public model fields.
- Decimal strings require numeric parsing for consumers that perform arithmetic.
- New public model fields are not exposed in CLI JSON until the DTO is updated intentionally.

## Verification

Black-box CLI tests cover:

- a complete schema-version-1 fixture;
- Standard and Skippable frame tagged shapes;
- decimal-string `u64` values and `snake_case` names;
- absent versus explicitly encoded zero Dictionary IDs;
- RLE declared versus encoded sizes;
- non-zero failure with no partial JSON output.

Any future schema version must add equivalent contract-focused tests before release.
