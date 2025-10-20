// Copyright (c) Microsoft Corporation. All rights reserved.
use vercode::{VerCodable, Vercode, deserialize, serialize};

#[test]
fn string_serialization() {
    let text = String::from("Hello, World!");
    let mut buf = [0u8; 128];
    let serialized = serialize(&text, &mut buf);

    // 4 bytes length prefix + 13 bytes of UTF-8 data = 17
    assert_eq!(serialized.len(), 17);
    assert_eq!(&serialized[0..4], &13u32.to_le_bytes());
    assert_eq!(&serialized[4..17], b"Hello, World!");

    let decoded = deserialize::<String>(serialized).unwrap();
    assert_eq!(decoded, text);
}

#[test]
fn string_empty() {
    let text = String::new();
    let mut buf = [0u8; 128];
    let serialized = serialize(&text, &mut buf);

    // 4 bytes length prefix + 0 bytes of data = 4
    assert_eq!(serialized.len(), 4);
    assert_eq!(&serialized[0..4], &0u32.to_le_bytes());

    let decoded = deserialize::<String>(serialized).unwrap();
    assert_eq!(decoded, text);
}

#[test]
fn string_unicode() {
    let text = String::from("Hello 世界 🦀");
    let mut buf = [0u8; 128];
    let serialized = serialize(&text, &mut buf);

    let decoded = deserialize::<String>(serialized).unwrap();
    assert_eq!(decoded, text);
}

#[test]
fn struct_with_string() {
    #[derive(Vercode, Debug, PartialEq, Clone)]
    struct Person {
        name: String,
        age: u32,
    }

    let person = Person {
        name: String::from("Alice"),
        age: 30,
    };

    let mut buf = [0u8; 256];
    let serialized = serialize(&person, &mut buf);
    let decoded = deserialize::<Person>(serialized).unwrap();

    assert_eq!(decoded, person);
    assert_eq!(decoded.name, "Alice");
    assert_eq!(decoded.age, 30);
}

#[test]
fn string_invalid_utf8() {
    // Create a buffer with invalid UTF-8
    let mut buf = [0u8; 128];
    // Length = 3
    buf[0..4].copy_from_slice(&3u32.to_le_bytes());
    // Invalid UTF-8 sequence
    buf[4] = 0xFF;
    buf[5] = 0xFE;
    buf[6] = 0xFD;

    let result = String::read_version(0, &buf);
    assert!(result.is_err());
}

#[test]
fn string_buffer_too_short() {
    // Create a buffer that claims to have more data than available
    let mut buf = [0u8; 10];
    // Length = 100 (but we only have 6 bytes left)
    buf[0..4].copy_from_slice(&100u32.to_le_bytes());

    let result = String::read_version(0, &buf);
    assert!(result.is_err());
}
