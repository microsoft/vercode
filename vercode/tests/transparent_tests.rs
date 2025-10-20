// Copyright (c) Microsoft Corporation. All rights reserved.
use vercode::{
    VerCodable, Vercode, VercodeTransparent, deserialize, deserialize_version, serialize,
    serialize_version,
};

#[test]
fn transparent_newtype() {
    #[derive(Debug, PartialEq, VercodeTransparent, Default, Clone, Copy)]
    struct UserId(u32);

    let user_id = UserId(12345);
    let mut buf = [0u8; 32];
    let serialized = serialize(&user_id, &mut buf);

    // Should be just 4 bytes for u32, no length prefix or wrapper overhead
    assert_eq!(serialized.len(), 4);
    assert_eq!(&serialized[0..4], &12345u32.to_le_bytes());

    let decoded = deserialize::<UserId>(serialized).unwrap();
    assert_eq!(decoded, user_id);
}

#[test]
fn transparent_newtype_with_versioned_inner() {
    #[derive(Default, Vercode, Debug, PartialEq, Clone, Copy)]
    struct Metadata {
        id: u16,
        #[version(1)]
        timestamp: u64,
    }

    #[derive(Debug, PartialEq, VercodeTransparent, Default, Clone, Copy)]
    struct WrappedMetadata(Metadata);

    let wrapped = WrappedMetadata(Metadata {
        id: 100,
        timestamp: 9999,
    });

    let mut buf0 = [0u8; 64];

    // Version 0: should write only id (2 bytes) + 4 byte length prefix = 6 bytes
    let serialized_v0 = serialize_version(&wrapped, 0, &mut buf0);
    assert_eq!(serialized_v0.len(), 6);

    // Version 1: should write id (2 bytes) + timestamp (8 bytes) + 4 byte length prefix = 14 bytes
    let mut buf1 = [0u8; 64];
    let serialized_v1 = serialize_version(&wrapped, 1, &mut buf1);
    assert_eq!(serialized_v1.len(), 14);

    // Test round-trip
    let decoded = deserialize_version::<WrappedMetadata>(1, serialized_v1).unwrap();
    assert_eq!(decoded, wrapped);

    // Verify MAX_VERSION is inherited from inner type
    assert_eq!(WrappedMetadata::MAX_VERSION, Metadata::MAX_VERSION);
    assert_eq!(WrappedMetadata::MAX_VERSION, 1);
}

#[test]
fn transparent_named_field() {
    #[derive(Debug, PartialEq, VercodeTransparent, Default, Clone, Copy)]
    struct Score {
        value: u32,
    }

    let score = Score { value: 42 };
    let mut buf = [0u8; 32];
    let serialized = serialize(&score, &mut buf);

    // Should be just 4 bytes for u32, no overhead
    assert_eq!(serialized.len(), 4);
    assert_eq!(&serialized[0..4], &42u32.to_le_bytes());

    let decoded = deserialize::<Score>(serialized).unwrap();
    assert_eq!(decoded.value, 42);
}

#[test]
fn size_of_newtype_should_be_same_as_inner() {
    // Use VercodeTransparent for zero-overhead newtype wrappers
    #[derive(Default, VercodeTransparent, Debug, PartialEq)]
    struct NewType(u32);

    let new_type = NewType(42u32);

    let size_struct = vercode::size(&42u32);
    let size_newtype = vercode::size(&new_type);

    // With VercodeTransparent, sizes should be identical (no length prefix overhead)
    assert_eq!(size_struct, size_newtype);
    assert_eq!(size_newtype, 4); // Just the u32
}
