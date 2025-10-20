// Copyright (c) Microsoft Corporation. All rights reserved.
use vercode::{VerCodable, Vercode, deserialize, serialize};

// A type that does NOT implement Default
#[derive(Debug, PartialEq, Clone, Copy)]
struct NoDefault {
    value: u32,
}

impl NoDefault {
    fn new(value: u32) -> Self {
        NoDefault { value }
    }
}

// Implement VerCodable manually for NoDefault
impl VerCodable for NoDefault {
    const MAX_VERSION: u32 = 0;

    fn write_version(&self, _version: u32, buf: &mut [u8]) -> usize {
        buf[0..4].copy_from_slice(&self.value.to_le_bytes());
        4
    }

    fn read_version(_version: u32, buf: &[u8]) -> Result<(Self, usize), vercode::InvalidEncoding> {
        if buf.len() < 4 {
            return Err(vercode::InvalidEncoding);
        }
        let value = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        Ok((NoDefault { value }, 4))
    }

    fn size_version(&self, _version: u32) -> usize {
        4
    }
}

#[test]
fn version_zero_no_default_struct() {
    // Version 0 fields don't need Default!
    #[derive(Debug, PartialEq, Vercode, Clone)]
    struct TestStruct {
        // Version 0 field - no Default required
        base: NoDefault,
        // Version 1 field - DOES require Default
        #[version(1)]
        extra: u32,
    }

    let original = TestStruct {
        base: NoDefault::new(42),
        extra: 100,
    };

    let mut buf = [0u8; 128];
    let serialized = serialize(&original, &mut buf);

    let decoded = deserialize::<TestStruct>(serialized).unwrap();
    assert_eq!(decoded.base.value, 42);
    assert_eq!(decoded.extra, 100);
}

#[test]
fn version_zero_no_default_enum() {
    // Enum variant fields at version 0 don't need Default!
    #[derive(Debug, PartialEq, Vercode, Clone)]
    enum TestEnum {
        Variant {
            // Version 0 field - no Default required
            base: NoDefault,
            // Version 1 field - DOES require Default
            #[version(1)]
            extra: u32,
        },
    }

    let original = TestEnum::Variant {
        base: NoDefault::new(99),
        extra: 200,
    };

    let mut buf = [0u8; 128];
    let serialized = serialize(&original, &mut buf);

    let decoded = deserialize::<TestEnum>(serialized).unwrap();

    let TestEnum::Variant { base, extra } = decoded;
    assert_eq!(base.value, 99);
    assert_eq!(extra, 200);
}

#[test]
fn version_zero_fields_always_read() {
    // Test that version 0 fields are ALWAYS read without length checks
    #[derive(Debug, PartialEq, Vercode, Clone)]
    struct AlwaysPresent {
        id: NoDefault,
        count: u32,
    }

    let original = AlwaysPresent {
        id: NoDefault::new(123),
        count: 456,
    };

    let mut buf = [0u8; 128];
    let serialized = serialize(&original, &mut buf);

    // This should succeed because version 0 fields are always read
    let decoded = deserialize::<AlwaysPresent>(serialized).unwrap();
    assert_eq!(decoded.id.value, 123);
    assert_eq!(decoded.count, 456);
}

#[test]
fn multiple_version_zero_fields_no_default() {
    // Demonstrate that multiple version 0 fields can all skip Default
    #[derive(Debug, PartialEq, Vercode, Clone)]
    struct MultiBase {
        first: NoDefault,
        second: NoDefault,
        third: NoDefault,
        // Only this needs Default since it's version 1
        #[version(1)]
        optional: u16,
    }

    let original = MultiBase {
        first: NoDefault::new(111),
        second: NoDefault::new(222),
        third: NoDefault::new(333),
        optional: 444,
    };

    let mut buf = [0u8; 256];
    let serialized = serialize(&original, &mut buf);
    let decoded = deserialize::<MultiBase>(serialized).unwrap();

    assert_eq!(decoded.first.value, 111);
    assert_eq!(decoded.second.value, 222);
    assert_eq!(decoded.third.value, 333);
    assert_eq!(decoded.optional, 444);
}
