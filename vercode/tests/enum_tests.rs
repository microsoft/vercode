// Copyright (c) Microsoft Corporation. All rights reserved.
use vercode::{Vercode, deserialize, serialize, serialize_version};

#[test]
fn enum_basic() {
    #[derive(Debug, Default, PartialEq, Vercode, Clone)]
    enum SimpleEnum {
        #[default]
        A,
        B,
        C,
    }

    let a = SimpleEnum::A;
    let mut buf = [0u8; 32];
    let serialized = serialize(&a, &mut buf);
    // 4 bytes length prefix + 2 bytes discriminant = 6
    assert_eq!(serialized.len(), 6);
    assert_eq!(&serialized[0..4], &(6u32.to_le_bytes()));
    assert_eq!(&serialized[4..6], &(0u16.to_le_bytes()));

    let decoded = deserialize::<SimpleEnum>(serialized).unwrap();
    assert_eq!(decoded, SimpleEnum::A);

    let b = SimpleEnum::B;
    let serialized = serialize(&b, &mut buf);
    assert_eq!(serialized.len(), 6);
    assert_eq!(&serialized[4..6], &(1u16.to_le_bytes()));

    let decoded = deserialize::<SimpleEnum>(serialized).unwrap();
    assert_eq!(decoded, SimpleEnum::B);
}

#[test]
fn enum_with_fields() {
    #[derive(Debug, PartialEq, Vercode, Default, Clone)]
    enum Message {
        #[default]
        Quit,
        Move {
            x: u32,
            y: u32,
        },
        Write(u8, u8),
    }

    // Test Quit variant
    let quit = Message::Quit;
    let mut buf = [0u8; 64];
    let serialized = serialize(&quit, &mut buf);
    assert_eq!(serialized.len(), 6); // 4 + 2
    let decoded = deserialize::<Message>(serialized).unwrap();
    assert_eq!(decoded, Message::Quit);

    // Test Move variant
    let move_msg = Message::Move { x: 100, y: 200 };
    let serialized = serialize(&move_msg, &mut buf);
    // 4 (length) + 2 (discriminant) + 4 (x) + 4 (y) = 14
    assert_eq!(serialized.len(), 14);
    assert_eq!(&serialized[0..4], &(14u32.to_le_bytes()));
    assert_eq!(&serialized[4..6], &(1u16.to_le_bytes())); // discriminant 1

    let decoded = deserialize::<Message>(serialized).unwrap();
    assert_eq!(decoded, Message::Move { x: 100, y: 200 });

    // Test Write variant
    let write_msg = Message::Write(10, 20);
    let serialized = serialize(&write_msg, &mut buf);
    // 4 + 2 + 1 + 1 = 8
    assert_eq!(serialized.len(), 8);
    assert_eq!(&serialized[4..6], &(2u16.to_le_bytes())); // discriminant 2

    let decoded = deserialize::<Message>(serialized).unwrap();
    assert_eq!(decoded, Message::Write(10, 20));
}

#[test]
fn enum_variant_with_field_versions() {
    #[derive(Debug, PartialEq, Vercode, Clone)]
    enum Data {
        Record {
            id: u32,
            #[version(1)]
            metadata: u16,
        },
    }

    impl Default for Data {
        fn default() -> Self {
            Data::Record { id: 0, metadata: 0 }
        }
    }

    let record = Data::Record {
        id: 100,
        metadata: 200,
    };
    let mut buf0 = [0u8; 64];

    // Version 0: variant not available, can't write it meaningfully
    let serialized_v0 = serialize_version(&record, 0, &mut buf0);
    assert_eq!(serialized_v0.len(), 10);

    // Test round-trip at version 2
    let decoded = deserialize::<Data>(serialized_v0).unwrap();
    assert_eq!(
        decoded,
        Data::Record {
            id: 100,
            metadata: 0, // default value
        }
    );

    // Version 1: variant available but metadata field not yet
    let mut buf1 = [0u8; 64];
    let serialized_v1 = serialize_version(&record, 1, &mut buf1);
    assert_eq!(serialized_v1.len(), 12);

    // Test round-trip at version 1 (can't test v0 since variant doesn't exist there)
    eprintln!(
        "About to deserialize version 1 with buf len {}",
        serialized_v1.len()
    );
    let result = deserialize::<Data>(serialized_v1);
    eprintln!("deserialize result: {result:?}");
    let decoded = result.unwrap();
    assert_eq!(
        decoded,
        Data::Record {
            id: 100,
            metadata: 200
        }
    ); // metadata gets default
}

#[test]
fn enum_field_name_conflicts() {
    // Test that field names don't conflict with macro internal variables
    #[derive(Debug, PartialEq, Vercode, Clone)]
    enum ConflictingNames {
        Data {
            length: u32,
            offset: u16,
            total: u8,
            version: u64,
            buf: u32,
            more: u16,
        },
    }

    impl Default for ConflictingNames {
        fn default() -> Self {
            ConflictingNames::Data {
                length: 0,
                offset: 0,
                total: 0,
                version: 0,
                buf: 0,
                more: 0,
            }
        }
    }

    let data = ConflictingNames::Data {
        length: 100,
        offset: 200,
        total: 50,
        version: 12345,
        buf: 999,
        more: 777,
    };

    let mut buf_array = [0u8; 128];
    let serialized = serialize(&data, &mut buf_array);

    // 4 (length prefix) + 2 (discriminant) + 4 (length field) + 2 (offset field) +
    // 1 (total field) + 8 (version field) + 4 (buf field) + 2 (more field) = 27
    assert_eq!(serialized.len(), 27);

    let decoded = deserialize::<ConflictingNames>(serialized).unwrap();
    assert_eq!(decoded, data);
}
