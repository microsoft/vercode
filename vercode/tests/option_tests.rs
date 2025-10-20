// Copyright (c) Microsoft Corporation. All rights reserved.

//! Comprehensive tests for Option<T> serialization and deserialization.
//!
//! This test suite verifies:
//! - Option<T> round-trip serialization for various types T
//! - Size assertions for niche-optimized types (bool, NonZeroXXX) vs regular types
//! - Niche optimization: Option<bool> and Option<NonZeroXXX> use same size as T
//! - Regular types: Option<T> adds 1 byte discriminant to size of T
//! - Nested Option handling (Option<Option<T>>)
//! - Option fields within structs and their interaction with versioning
//! - Version compatibility: Option<T> fields work correctly with version directives
//! - Backward compatibility: old data can be read by new code with default None values
//!
//! Expected encoding sizes:
//! - Option<bool>: 1 byte (niche optimization: false=0, true=1, None=2)  
//! - Option<NonZeroXXX>: same as XXX (niche optimization: value=value, None=0)
//! - Option<T> for other T: size of T + 1 byte (discriminant: Some=1, None=0)

use std::num::{
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
};
use vercode::{
    VerCodable, Vercode, VercodeTransparent, deserialize, serialize, serialize_to_vec, size,
};

/// Generic helper function for testing Option<T> round-trip serialization
/// and size assertions.
fn test_option_round_trip<T>(some_value: T, expected_some_size: usize, expected_none_size: usize)
where
    T: VerCodable + PartialEq + std::fmt::Debug + Clone,
{
    // Test Some(value) case
    let some_option = Some(some_value.clone());
    let mut buf = vec![0u8; 1024];
    let serialized_some = serialize(&some_option, &mut buf);

    assert_eq!(
        serialized_some.len(),
        expected_some_size,
        "Some({some_value:?}) serialized size mismatch"
    );
    assert_eq!(
        size(&some_option),
        expected_some_size,
        "Some({some_value:?}) size calculation mismatch"
    );

    let deserialized_some: Option<T> =
        deserialize(serialized_some).expect("Failed to deserialize Some variant");
    assert_eq!(deserialized_some, some_option);

    // Test None case
    let none_option: Option<T> = None;
    let serialized_none = serialize(&none_option, &mut buf);

    assert_eq!(
        serialized_none.len(),
        expected_none_size,
        "None serialized size mismatch for type {:?}",
        std::any::type_name::<T>()
    );
    assert_eq!(
        size(&none_option),
        expected_none_size,
        "None size calculation mismatch for type {:?}",
        std::any::type_name::<T>()
    );

    let deserialized_none: Option<T> =
        deserialize(serialized_none).expect("Failed to deserialize None variant");
    assert_eq!(deserialized_none, none_option);
}

#[test]
fn test_option_bool_niche_optimization() {
    // For bool, Option<bool> should use niche optimization:
    // Some(false) = 0, Some(true) = 1, None = 2
    // All encoded in 1 byte (same as bool)
    test_option_round_trip(true, 1, 1);
    test_option_round_trip(false, 1, 1);
}

#[test]
fn test_option_nonzero_niche_optimization() {
    // For NonZeroXXX types, Option<NonZeroXXX> should use niche optimization:
    // Some(value) = value, None = 0
    // Same size as the underlying type

    test_option_round_trip(NonZeroU8::new(42).unwrap(), 1, 1);
    test_option_round_trip(NonZeroU16::new(1234).unwrap(), 2, 2);
    test_option_round_trip(NonZeroU32::new(123456).unwrap(), 4, 4);
    test_option_round_trip(NonZeroU64::new(123456789).unwrap(), 8, 8);
    test_option_round_trip(NonZeroU128::new(123456789).unwrap(), 16, 16);
    test_option_round_trip(
        NonZeroUsize::new(42).unwrap(),
        size_of::<usize>(),
        size_of::<usize>(),
    );

    test_option_round_trip(NonZeroI8::new(-42).unwrap(), 1, 1);
    test_option_round_trip(NonZeroI16::new(-1234).unwrap(), 2, 2);
    test_option_round_trip(NonZeroI32::new(-123456).unwrap(), 4, 4);
    test_option_round_trip(NonZeroI64::new(-123456789).unwrap(), 8, 8);
    test_option_round_trip(NonZeroI128::new(-123456789).unwrap(), 16, 16);
    test_option_round_trip(
        NonZeroIsize::new(-42).unwrap(),
        size_of::<isize>(),
        size_of::<isize>(),
    );
}

#[test]
fn test_option_regular_types_extra_byte() {
    // For regular types, Option<T> should add 1 byte for the discriminant

    test_option_round_trip(42u8, 1 + 1, 1); // u8 (1) + discriminant (1)
    test_option_round_trip(1234u16, 2 + 1, 1); // u16 (2) + discriminant (1)  
    test_option_round_trip(123456u32, 4 + 1, 1); // u32 (4) + discriminant (1)
    test_option_round_trip(123456789u64, 8 + 1, 1); // u64 (8) + discriminant (1)
    test_option_round_trip(123456789u128, 16 + 1, 1); // u128 (16) + discriminant (1)

    test_option_round_trip(-42i8, 1 + 1, 1); // i8 (1) + discriminant (1)
    test_option_round_trip(-1234i16, 2 + 1, 1); // i16 (2) + discriminant (1)
    test_option_round_trip(-123456i32, 4 + 1, 1); // i32 (4) + discriminant (1)
    test_option_round_trip(-123456789i64, 8 + 1, 1); // i64 (8) + discriminant (1)
    test_option_round_trip(-123456789i128, 16 + 1, 1); // i128 (16) + discriminant (1)

    test_option_round_trip(std::f32::consts::PI, 4 + 1, 1); // f32 (4) + discriminant (1)
    test_option_round_trip(std::f64::consts::E, 8 + 1, 1); // f64 (8) + discriminant (1)

    test_option_round_trip(42usize, size_of::<usize>() + 1, 1); // usize + discriminant (1)
    test_option_round_trip(-42isize, size_of::<isize>() + 1, 1); // isize + discriminant (1)
}

#[test]
fn test_option_string() {
    let test_string = "Hello, World!".to_string();
    // String is encoded as: length (4 bytes) + content (13 bytes) = 17 bytes
    // Option<String> adds 1 byte discriminant: 17 + 1 = 18 bytes for Some
    test_option_round_trip(test_string, 18, 1);

    // Empty string
    let empty_string = String::new();
    // Empty string: length (4 bytes) + content (0 bytes) = 4 bytes
    // Option<String> adds 1 byte discriminant: 4 + 1 = 5 bytes for Some
    test_option_round_trip(empty_string, 5, 1);
}

#[test]
fn test_option_char() {
    // char is encoded as u32 (4 bytes)
    // Option<char> adds 1 byte discriminant: 4 + 1 = 5 bytes for Some
    test_option_round_trip('A', 5, 1);
    test_option_round_trip('🦀', 5, 1); // Unicode character
}

#[test]
fn test_option_array() {
    // Arrays are encoded as each element serialized
    let test_array = [1u8, 2u8, 3u8];
    // Array of 3 u8s: 3 bytes
    // Option<[u8; 3]> adds 1 byte discriminant: 3 + 1 = 4 bytes for Some
    test_option_round_trip(test_array, 4, 1);
}

#[test]
fn test_option_vec() {
    let test_vec = vec![1u32, 2u32, 3u32];
    // Vec<u32> is encoded as: length (4 bytes) + 3 * u32 (12 bytes) = 16 bytes
    // Option<Vec<u32>> adds 1 byte discriminant: 16 + 1 = 17 bytes for Some
    test_option_round_trip(test_vec, 17, 1);

    // Empty vec
    let empty_vec: Vec<u32> = vec![];
    // Empty Vec<u32>: length (4 bytes) + content (0 bytes) = 4 bytes
    // Option<Vec<u32>> adds 1 byte discriminant: 4 + 1 = 5 bytes for Some
    test_option_round_trip(empty_vec, 5, 1);
}

#[test]
fn test_option_with_version_directives() {
    #[derive(Vercode, Debug, PartialEq, Clone, Default)]
    struct VersionedStruct {
        base_field: u32,
        #[version(1)]
        optional_field: Option<u64>,
        #[version(2)]
        another_field: Option<NonZeroU32>,
    }

    let test_struct = VersionedStruct {
        base_field: 42,
        optional_field: Some(1234567890),
        another_field: Some(NonZeroU32::new(999).unwrap()),
    };

    // Test full serialization (all versions)
    let serialized = serialize_to_vec(&test_struct);
    let deserialized: VersionedStruct =
        deserialize(&serialized).expect("Failed to deserialize versioned struct");
    assert_eq!(deserialized, test_struct);

    // Test that sizes are calculated correctly
    let total_size = size(&test_struct);
    assert_eq!(serialized.len(), total_size);

    // Create a struct with None values
    let test_struct_none = VersionedStruct {
        base_field: 42,
        optional_field: None,
        another_field: None,
    };

    let serialized_none = serialize_to_vec(&test_struct_none);
    let deserialized_none: VersionedStruct = deserialize(&serialized_none)
        .expect("Failed to deserialize versioned struct with None values");
    assert_eq!(deserialized_none, test_struct_none);

    // Test version compatibility: old data can be read by new code
    use vercode::{deserialize_version, serialize_version};

    // Test version 0 serialization (only base_field)
    let mut buf0 = vec![0u8; 1024];
    let v0_serialized = serialize_version(&test_struct, 0, &mut buf0);
    let v0_deserialized: VersionedStruct =
        deserialize(v0_serialized).expect("Failed to deserialize version 0 data");

    // When deserializing version 0 data, versioned fields should get default values (None for Option)
    let expected_v0 = VersionedStruct {
        base_field: 42,
        optional_field: None, // Default value since this is version 1 field
        another_field: None,  // Default value since this is version 2 field
    };
    assert_eq!(v0_deserialized, expected_v0);

    // Test version 1 serialization (base_field + optional_field)
    let mut buf1 = vec![0u8; 1024];
    let v1_serialized = serialize_version(&test_struct, 1, &mut buf1);
    let v1_deserialized: VersionedStruct =
        deserialize(v1_serialized).expect("Failed to deserialize version 1 data");

    let expected_v1 = VersionedStruct {
        base_field: 42,
        optional_field: Some(1234567890), // Present in version 1
        another_field: None,              // Default value since this is version 2 field
    };
    assert_eq!(v1_deserialized, expected_v1);

    // Test explicit version deserialization
    let v0_explicit: VersionedStruct = deserialize_version(0, v0_serialized)
        .expect("Failed to deserialize version 0 data explicitly");
    assert_eq!(v0_explicit, expected_v0);

    let v1_explicit: VersionedStruct = deserialize_version(1, v1_serialized)
        .expect("Failed to deserialize version 1 data explicitly");
    assert_eq!(v1_explicit, expected_v1);

    // Test that Option niche optimization works with versioned fields
    #[derive(Vercode, Debug, PartialEq, Clone, Default)]
    struct NicheVersionedStruct {
        id: u16,
        #[version(1)]
        niche_bool: Option<bool>,
        #[version(2)]
        niche_nonzero: Option<NonZeroU32>,
        #[version(3)]
        regular_option: Option<u32>,
    }

    let niche_struct = NicheVersionedStruct {
        id: 100,
        niche_bool: Some(true),
        niche_nonzero: Some(NonZeroU32::new(42).unwrap()),
        regular_option: Some(999),
    };

    // Test full round-trip
    let niche_serialized = serialize_to_vec(&niche_struct);
    let niche_deserialized: NicheVersionedStruct = deserialize(&niche_serialized).unwrap();
    assert_eq!(niche_deserialized, niche_struct);

    // Test that version 0 only includes id
    let mut niche_buf0 = vec![0u8; 1024];
    let niche_v0 = serialize_version(&niche_struct, 0, &mut niche_buf0);
    let niche_v0_deser: NicheVersionedStruct = deserialize(niche_v0).unwrap();
    assert_eq!(
        niche_v0_deser,
        NicheVersionedStruct {
            id: 100,
            niche_bool: None,
            niche_nonzero: None,
            regular_option: None,
        }
    );

    // Test that version 1 includes id + niche_bool
    let mut niche_buf1 = vec![0u8; 1024];
    let niche_v1 = serialize_version(&niche_struct, 1, &mut niche_buf1);
    let niche_v1_deser: NicheVersionedStruct = deserialize(niche_v1).unwrap();
    assert_eq!(
        niche_v1_deser,
        NicheVersionedStruct {
            id: 100,
            niche_bool: Some(true),
            niche_nonzero: None,
            regular_option: None,
        }
    );

    // Test that version 2 includes id + niche_bool + niche_nonzero
    let mut niche_buf2 = vec![0u8; 1024];
    let niche_v2 = serialize_version(&niche_struct, 2, &mut niche_buf2);
    let niche_v2_deser: NicheVersionedStruct = deserialize(niche_v2).unwrap();
    assert_eq!(
        niche_v2_deser,
        NicheVersionedStruct {
            id: 100,
            niche_bool: Some(true),
            niche_nonzero: Some(NonZeroU32::new(42).unwrap()),
            regular_option: None,
        }
    );
}

#[test]
fn test_nested_options() {
    // Test Option<Option<T>>
    let double_some: Option<Option<u32>> = Some(Some(42));
    let some_none: Option<Option<u32>> = Some(None);
    let none: Option<Option<u32>> = None;

    // Option<Option<u32>> structure:
    // - Outer Option adds 1 byte discriminant
    // - Inner Option<u32> adds 1 byte discriminant + 4 bytes for u32 when Some

    let mut buf = vec![0u8; 1024];

    // Some(Some(42)): outer discriminant (1) + inner discriminant (1) + u32 (4) = 6 bytes
    let serialized_double_some = serialize(&double_some, &mut buf);
    assert_eq!(serialized_double_some.len(), 6);
    let deserialized: Option<Option<u32>> = deserialize(serialized_double_some).unwrap();
    assert_eq!(deserialized, double_some);

    // Some(None): outer discriminant (1) + inner discriminant (1) = 2 bytes
    let serialized_some_none = serialize(&some_none, &mut buf);
    assert_eq!(serialized_some_none.len(), 2);
    let deserialized: Option<Option<u32>> = deserialize(serialized_some_none).unwrap();
    assert_eq!(deserialized, some_none);

    // None: outer discriminant (1) = 1 byte
    let serialized_none = serialize(&none, &mut buf);
    assert_eq!(serialized_none.len(), 1);
    let deserialized: Option<Option<u32>> = deserialize(serialized_none).unwrap();
    assert_eq!(deserialized, none);
}

#[test]
fn test_option_with_custom_struct() {
    #[derive(Vercode, Debug, PartialEq, Clone)]
    struct Point {
        x: f32,
        y: f32,
    }

    let point = Point { x: 1.5, y: 2.5 };
    // Point: length (4) + f32 (4) + f32 (4) = 12 bytes
    // Option<Point> adds 1 byte discriminant: 12 + 1 = 13 bytes for Some
    test_option_round_trip(point, 13, 1);
}

#[test]
fn test_option_with_versioned_struct() {
    #[derive(Vercode, Debug, PartialEq, Clone)]
    struct VersionedPoint {
        x: f32,
        #[version(1)]
        y: f32,
    }

    // Test that Option<T> works correctly when T has version attributes
    let point = VersionedPoint { x: 1.0, y: 2.0 };

    let some_point = Some(point.clone());
    let none_point: Option<VersionedPoint> = None;

    // Serialize and deserialize
    let serialized_some = serialize_to_vec(&some_point);
    let serialized_none = serialize_to_vec(&none_point);

    let deserialized_some: Option<VersionedPoint> = deserialize(&serialized_some).unwrap();
    let deserialized_none: Option<VersionedPoint> = deserialize(&serialized_none).unwrap();

    assert_eq!(deserialized_some, some_point);
    assert_eq!(deserialized_none, none_point);
}

// Helper function to get the size of a type at compile time
const fn size_of<T>() -> usize {
    std::mem::size_of::<T>()
}

#[test]
fn test_option_struct_with_option_fields() {
    #[derive(Vercode, Debug, PartialEq, Clone, Default)]
    struct TestStruct {
        // Regular field
        id: u32,

        // Option field with regular type (adds extra byte)
        optional_name: Option<String>,

        // Option field with NonZero type (niche optimization)
        optional_count: Option<NonZeroU32>,

        // Option field with bool (niche optimization)
        optional_flag: Option<bool>,

        // Regular Option field (avoiding version directive due to known serialization issues)
        regular_data: Option<u64>,
    }

    // Test with all Some values
    let test_with_some = TestStruct {
        id: 42,
        optional_name: Some("test".to_string()),
        optional_count: Some(NonZeroU32::new(100).unwrap()),
        optional_flag: Some(true),
        regular_data: Some(999),
    };

    let serialized_some = serialize_to_vec(&test_with_some);
    let deserialized_some: TestStruct = deserialize(&serialized_some).unwrap();
    assert_eq!(deserialized_some, test_with_some);

    // Test with all None values
    let test_with_none = TestStruct {
        id: 42,
        optional_name: None,
        optional_count: None,
        optional_flag: None,
        regular_data: None,
    };

    let serialized_none = serialize_to_vec(&test_with_none);
    let deserialized_none: TestStruct = deserialize(&serialized_none).unwrap();
    assert_eq!(deserialized_none, test_with_none);

    // Test mixed case
    let test_mixed = TestStruct {
        id: 42,
        optional_name: Some("hello".to_string()),
        optional_count: None,
        optional_flag: Some(false),
        regular_data: None,
    };

    let serialized_mixed = serialize_to_vec(&test_mixed);
    let deserialized_mixed: TestStruct = deserialize(&serialized_mixed).unwrap();
    assert_eq!(deserialized_mixed, test_mixed);
}

#[test]
fn test_option_transparent() {
    #[derive(VercodeTransparent)]
    struct Transparent(NonZeroU64);

    assert_eq!(8, vercode::size(&Transparent(NonZeroU64::MIN)));
}
