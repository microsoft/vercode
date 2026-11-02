# Changelog

This changelog covers `vercode` and `vercode-macros`.

## 0.6.0

- **Breaking:** `serialize`, `serialize_version`, `serialize_to_vec`, `size`,
  and `size_version` now return `Result`, with `InvalidEncoding` as the error type.
  Custom `VerCodable` implementations must update `write_version` and
  `size_version`, plus any overrides of `write_option` and `size_option_version`,
  to return `Result<usize, InvalidEncoding>`.
- Reject string lengths, collection counts, and derived frame sizes greater
  than `u32::MAX` instead of silently truncating their length fields.
- Return `InvalidEncoding` for encoded-size arithmetic overflow and propagate
  nested serialization and size errors.
- Check the encoded size before `serialize_to_vec` allocates its output buffer.
- Reject derived `Vercode` enums with more than 65536 variants at compile time.
- Add regression tests for length boundaries, oversized values, arithmetic
  overflow, nested errors, and version filtering.

Valid values retain the same wire format.
Buffer-based serializers still require sufficient output space and can panic
if the buffer is too small. An error can leave the output buffer partially written.

## 0.5.5

- Return `InvalidEncoding` for truncated booleans and fixed-width primitives,
  including nonzero integers, instead of panicking.
- Reject invalid nested byte counts and offset overflow in options, strings,
  collections, arrays, tuples, and derived readers.
- Stop array decoding after the first error.
- Restrict derived struct and enum readers to each object's declared frame.
  Reject frames smaller than their headers or larger than the available input.
- Return the full frame length so older readers skip unknown trailing fields
  before the next value. This fixes decoding of nested values across versions.
- Add regression tests for truncation, invalid lengths, overflow, nested frame
  boundaries, and version compatibility.

The public API and valid wire encodings remain unchanged.

## 0.5.4

- Limit initial reservations for generic `Vec`, `HashMap`, and `HashSet`
  decoding to 4096 elements or 1 MiB of element storage, whichever is smaller.
  Hash-table overhead and allocator rounding can increase the actual allocation.
  These limits do not bound later growth or total decoded memory.
- Check integer-vector byte requirements before allocation and copying.
  Return `InvalidEncoding` for truncated payloads or byte-count overflow.
- Read, write, and calculate the encoded size of `Vec<()>` in constant time.
  Its wire format remains a four-byte count with no element payload.
- Enforce the serialization size assertion in release builds as well as debug builds.

## 0.5.3

- Update the `syn` dependency from version 2 to version 3.

## 0.5.2

- Add `VercodeEnumU8` and `VercodeEnumU16` derives, with corresponding
  `U8Enum` and `U16Enum` marker traits, for fieldless integer-backed enums.
- Decode unknown integer enum codes as `Default::default()`.
  Unknown codes are discarded. Truncated input returns `InvalidEncoding`.
- Add examples, documentation, and compatibility tests for both integer widths.
- Add package descriptions and repository metadata.

The integer enum encoding differs from the existing `Vercode` enum encoding.
Adopting these derives for an existing field requires a wire-format migration.

## 0.5.1

- Calculate encoded sizes for vectors of primitive integers without per-element
  `size_version` calls. The encoded sizes and wire format remain unchanged.

## 0.5.0

- Initial public release of `vercode` and `vercode-macros`.
- Provide versioned binary serialization and deserialization for Rust data
  structures, with derive macros for structs, enums, and transparent wrappers.
