
# Vercode - Version based encoding and decoding

**Minimal overhead backward and forward compatible serialization for evolving data structures.**

Vercode is a Rust serialization library designed for systems that need to handle multiple versions of the same data structure simultaneously—like long-running processes, distributed systems, or protocols that must maintain backward compatibility.

## Why Vercode?

Traditional serialization libraries like `serde` force you to choose: either break compatibility when you add fields, maintain multiple struct definitions, or use versionable formats that add runtime overhead.

Alternatives like flatbuffers or protocol buffers require an external IDL and code generation. The resulting generated wrappers are not idiomatic Rust leading to additional hand wrapping, copying, or non-idiomatic Rust.

Vercode has lower runtime overhead than bincode (and slightly larger encoded size) while allowing for adding new fields to structs and enum variants while maintaining full backwards and forward compatibility. Specifically future code can deserialize messages from old serializers, getting `Default::default()` for the new fields. Past code can deserialize message from future serializers, skipping the new data it does not understand.

The [revision](https://docs.rs/revision/latest/revision/) crate addresses similar versioning needs but only supports new code reading older revisions. Vercode provides bidirectional compatibility, allowing old code to read new revisions.

## Quick Start

Add to your `Cargo.toml`:
```toml
[dependencies]
vercode = { version = "0.5" }
```

## Example: Evolving a Protocol

Here's a complete example showing structs, enums, and version evolution:

```rust
use vercode::{Vercode, serialize, deserialize};

// Enum with versioned fields in variants
#[derive(Vercode, Debug, PartialEq)]
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),

    // Added in version 1: authenticated messages
    #[version(1)]
    Auth { 
        token: String,

        // expanded with addition field in version 2
        #[version(2)]
        expires_at: u64,
    },
}

// Struct that evolved over three versions
#[derive(Vercode, Default, Debug, PartialEq)]
#[vercode(max_version = 2)]
struct User {
    // Version 0 fields (always present, no Default required)
    id: u32,
    name: String,
    
    // Version 1 field (added later, requires Default)
    #[version(1)]
    email: String,
    
    #[version(1)]
    last_message: Message,
    
    // Version 2 fields (even newer)
    #[version(2)]
    verified: bool,
}

// Create a fully versioned user
let user = User {
    id: 42,
    name: "Alice".into(),
    email: "alice@example.com".into(),
    verified: true,
    last_message: Message::Move { x: 100, y: 200 },
};

let mut buf = vec![0u8; 1024];
let serialized = serialize(&user, &mut buf);

// Deserialize - automatically handles version differences
let decoded: User = deserialize(serialized);
assert_eq!(decoded, user);

// Simulate old version (v0) reading new data
let old_data = vercode::serialize_version(&user, 0, &mut buf);
let old_user: User = deserialize(old_data);
// Fields from v1+ will be Default::default()
assert_eq!(old_user.id, 42);
assert_eq!(old_user.name, "Alice");
assert_eq!(old_user.email, "");  // Default for String
assert_eq!(old_user.verified, false);  // Default for bool
```

## Zero-Overhead Newtypes

For newtypes that should serialize identically to their inner type:

```rust
use vercode::VercodeTransparent;

#[derive(VercodeTransparent)]
struct UserId(u64);

// Serializes as just a u64, no wrapper overhead
```

## Integer enums with default fallback

For fieldless enums whose unknown wire codes should map to `Default::default()`,
derive `VercodeEnumU8` or `VercodeEnumU16` instead of `Vercode`. These implement
`VerCodable` and the `U8Enum` or `U16Enum` marker trait, respectively, delegating
to the existing integer codec.

```rust
use num_enum::{IntoPrimitive, TryFromPrimitive};
use vercode::{VercodeEnumU8, deserialize, serialize_to_vec};

#[derive(Copy, Clone, Debug, Default, PartialEq, IntoPrimitive, TryFromPrimitive, VercodeEnumU8)]
#[repr(u8)]
enum Mode {
    Normal = 0,
    #[default]
    Unknown = 255,
}

assert_eq!(serialize_to_vec(&Mode::Normal), vec![0]);
assert_eq!(deserialize::<Mode>(&[42]).unwrap(), Mode::Unknown);
```

Add `num_enum = "0.7"` to your dependencies for the conversion derives, or implement
`TryFrom<u8>` and `Into<u8>` yourself. For `VercodeEnumU16`, use `#[repr(u16)]` and the
corresponding `u16` conversions. Both require `Copy` and `Default`.

- Use **explicit, stable codes** and choose a safe default. `#[default]` selects
  the fallback; the variant does not have to be named `Unknown`.
- Unknown codes are **discarded**. Reserializing the fallback writes its own code.
  Use an integer newtype with `VercodeTransparent` when codes must be preserved.
- Truncated input returns `InvalidEncoding`, not the default.
- Encoding is one byte (`VercodeEnumU8`) or two little-endian bytes (`VercodeEnumU16`),
  without an enum length prefix. It is **not wire-compatible** with derived `Vercode` enums.
- Do not also derive `Vercode` on these enums. Implementing just the marker trait
  does not enable serialization; use the matching derive. Separate blanket
  implementations for the two markers would overlap under Rust's coherence rules.

See [`vercode/examples/num_enum.rs`](vercode/examples/num_enum.rs) for a parent
struct read by an older enum version. Run it with:

```sh
cargo run -p vercode --example num_enum
```

## Format

Vercode uses a **length-prefixed binary format**:

1. **Structs**: `[4-byte length][field₀][field₁]...[fieldₙ]`
2. **Enums**: `[4-byte length][2-byte discriminant][variant fields...]`
3. **Primitives**: Direct byte representation (little-endian)

Fields are ordered by version number (v0, then v1, then v2, etc.), and then source order.

## Supported Types

Native types implementing VerCodable:

- All integer types
- `NonZeroXXX` types
- `f32` and `f64`
- `bool`, `char`, and `()`
- `String` and `Uuid`

Containers where T, K, V are also supported types
  - `Option<T>`
  - `[T; N]`
  - `Vec<T>`
  - `HashMap<K, V>` and `HashSet<T>`

Nested structs (via Vercode derive attribute)

Enums with both unit and data-carrying variants (via Vercode derive attribute)

Tuples of size 1 to 10 consisting of elements that are in this list

## Limitations

- Maximum of 2^16 variants per enum
- Maximum of 2^32 bytes per struct or enum

## Breaking Changes

- Adding a field with a same or older version
- Rearrange orders of fields with the same version
- Changing a type
- Changing length of fixed size array
- Swapping Vercode attribute with VercodeTransparent, or vice versa

## Partially breaking changes

- Adding a new enum variant at the end will not break the format. Old deserializers will still be able to deserialize new serialized values, as long as there are no instances of the new variant(s). In case a new variant is encountered by an old deserializer, an error will be returned.

## Benchmark Comparison

The following benchmarks compare Vercode against bincode for a complex nested struct with enums, arrays, and multiple field types on a Standard_D32as_v5 VM.

| Operation       | Vercode   | Bincode   | Speedup     |
|-----------------|-----------|-----------|-------------|
| Serialize       | 16.3 ns   | 134.5 ns  | **8.2×**    |
| Deserialize     | 19.0 ns   | 79.8 ns   | **4.2×**    |
| Serialized Size | 199 bytes | 181 bytes | 1.1× larger |

## License

MIT

## Contributing

Contributions welcome! Please open issues for bugs or feature requests.
