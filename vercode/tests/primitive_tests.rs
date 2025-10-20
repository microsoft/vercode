// Copyright (c) Microsoft Corporation. All rights reserved.
use std::num::{
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
};
use uuid::Uuid;
use vercode::{VerCodable, Vercode, deserialize, serialize, serialize_version};

#[test]
fn test_uuid() {
    #[derive(Default, Vercode, Debug, PartialEq, Clone)]
    struct UuidTest {
        id: Uuid,
        #[version(1)]
        label: Uuid,
    }

    let ut = UuidTest {
        id: Uuid::new_v4(),
        label: Uuid::new_v4(),
    };
    let mut buf0 = [0u8; 40];
    let sv0 = serialize_version(&ut, 0, &mut buf0);
    assert_eq!(sv0.len(), 20); // 16 bytes for id + 4 bytes length prefix
    let mut buf1 = [0u8; 40];
    let sv1 = serialize_version(&ut, 1, &mut buf1);
    assert_eq!(sv1.len(), 36); // 16 bytes for id + 16 bytes for label + 4 bytes length prefix
    assert_eq!(ut.size_version(0), sv0.len());
    assert_eq!(ut.size_version(1), sv1.len());
    assert_eq!(vercode::size_version(&ut, 0), sv0.len());
    assert_eq!(vercode::size_version(&ut, 1), sv1.len());
}

#[test]
fn option_none_some_round_trip() {
    #[derive(Default, Vercode, Debug, PartialEq, Clone)]
    struct Inner {
        #[version(1)]
        a: u8,
        b: u8,
    }

    #[derive(Default, Vercode, Debug, PartialEq, Clone)]
    struct OptWrap {
        inner: Option<Inner>,
        tail: u8,
    }

    let none_case = OptWrap {
        inner: None,
        tail: 5,
    };
    let mut buf = [0u8; 128];
    let serialized_none = serialize(&none_case, &mut buf);
    let decoded_none = deserialize::<OptWrap>(serialized_none).unwrap();
    assert_eq!(decoded_none, none_case);

    let some_case = OptWrap {
        inner: Some(Inner { a: 9, b: 7 }),
        tail: 6,
    };
    let serialized_some = serialize(&some_case, &mut buf);
    let decoded_some = deserialize::<OptWrap>(serialized_some).unwrap();
    assert_eq!(decoded_some, some_case);
}

#[test]
fn types_round_trip_test() {
    #[derive(Clone, Debug, PartialEq, Vercode)]
    struct TestTypes {
        a: (u8,),
        b: (u16, u32),
        c: (u64, u128, i8),
        d: (i16, i32, i64, i128),
        e: (f32, f64, bool, String, Uuid),
        f: (char, usize, isize, NonZeroU8, NonZeroU16, NonZeroU32),
        g: (
            NonZeroU64,
            NonZeroU128,
            NonZeroUsize,
            NonZeroIsize,
            NonZeroI8,
            NonZeroI16,
            NonZeroI32,
        ),
        h: (NonZeroI64, NonZeroI128, [u8; 1], Option<u8>, (), u8, u8, u8),
        i: (bool, Vec<u8>, Vec<u32>, u8, u8, u8, u8, u8, u8),
        j: (u8, u8, u8, u8, u8, u8, u8, u8, u8, u8),
    }

    let example = TestTypes {
        a: (255,),
        b: (65535, 4294967295),
        c: (
            18446744073709551615,
            340282366920938463463374607431768211455,
            -128,
        ),
        d: (
            -32768,
            -2147483648,
            -9223372036854775808,
            -170141183460469231731687303715884105728,
        ),
        e: (4.1, 5.7, true, String::from("Test String"), Uuid::new_v4()),
        f: (
            '🦀',
            123456789,
            -123456789,
            NonZeroU8::new(1).unwrap(),
            NonZeroU16::new(2).unwrap(),
            NonZeroU32::new(3).unwrap(),
        ),
        g: (
            NonZeroU64::new(4).unwrap(),
            NonZeroU128::new(5).unwrap(),
            NonZeroUsize::new(6).unwrap(),
            NonZeroIsize::new(-7).unwrap(),
            NonZeroI8::new(-8).unwrap(),
            NonZeroI16::new(-9).unwrap(),
            NonZeroI32::new(-10).unwrap(),
        ),
        h: (
            NonZeroI64::new(-11).unwrap(),
            NonZeroI128::new(-12).unwrap(),
            [42u8; 1],
            Some(99),
            (),
            1,
            2,
            3,
        ),
        i: (false, vec![5u8], vec![14u32, 15u32], 4, 5, 6, 7, 8, 9),
        j: (10, 11, 12, 13, 14, 15, 16, 17, 18, 19),
    };

    let mut buf = [0u8; 1024];
    let serialized = serialize(&example, &mut buf);
    let deserialized = deserialize::<TestTypes>(serialized).unwrap();
    assert_eq!(deserialized, example);
}
