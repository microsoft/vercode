// Copyright (c) Microsoft Corporation. All rights reserved.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::num::{
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
};
use vercode::{InvalidEncoding, VerCodable, Vercode, deserialize};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Oversize;

impl VerCodable for Oversize {
    const MAX_VERSION: u32 = 0;

    fn write_version(&self, _version: u32, _buf: &mut [u8]) -> usize {
        0
    }

    fn read_version(_version: u32, _buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        Ok((Oversize, usize::MAX))
    }

    fn size_version(&self, _version: u32) -> usize {
        0
    }
}

#[derive(Vercode)]
struct Wrap {
    value: Oversize,
}

#[derive(Vercode)]
struct One {
    value: u32,
}

#[derive(Vercode)]
enum Item {
    Num(u32),
}

#[test]
fn truncated_input_returns_invalid_encoding() {
    assert_eq!(deserialize::<bool>(&[]), Err(InvalidEncoding));
    assert_eq!(deserialize::<Option<bool>>(&[]), Err(InvalidEncoding));
    assert_eq!(deserialize::<u8>(&[]), Err(InvalidEncoding));
    assert_eq!(deserialize::<u16>(&[1]), Err(InvalidEncoding));
    assert_eq!(deserialize::<u32>(&[1, 2]), Err(InvalidEncoding));
    assert_eq!(deserialize::<u64>(&[1, 2, 3, 4]), Err(InvalidEncoding));
    assert_eq!(deserialize::<u128>(&[0; 15]), Err(InvalidEncoding));
    assert_eq!(deserialize::<i32>(&[]), Err(InvalidEncoding));
    assert_eq!(deserialize::<f64>(&[0; 4]), Err(InvalidEncoding));
    assert_eq!(deserialize::<char>(&[0, 0]), Err(InvalidEncoding));
    assert_eq!(deserialize::<usize>(&[]), Err(InvalidEncoding));
    assert_eq!(deserialize::<NonZeroU32>(&[1, 0]), Err(InvalidEncoding));
    assert_eq!(deserialize::<Option<u32>>(&[1]), Err(InvalidEncoding));
    assert_eq!(deserialize::<[u32; 2]>(&[1, 0, 0, 0]), Err(InvalidEncoding));
    assert_eq!(deserialize::<(u8, u32)>(&[1]), Err(InvalidEncoding));
    assert_eq!(
        deserialize::<Vec<bool>>(&2u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<String>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<HashMap<u8, u8>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<HashSet<u16>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<BTreeMap<u8, u8>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    // On 32-bit, 4 + u32::MAX overflows. On 64-bit this length only passes the buffer.
    assert_eq!(
        deserialize::<String>(&u32::MAX.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert!(matches!(
        deserialize::<One>(&[8, 0, 0, 0]),
        Err(InvalidEncoding)
    ));
    assert!(matches!(
        deserialize::<Item>(&[6, 0, 0, 0, 0, 0]),
        Err(InvalidEncoding)
    ));
}

#[test]
fn reported_length_past_the_buffer_is_an_error() {
    assert_eq!(
        deserialize::<Vec<Oversize>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(deserialize::<(Oversize,)>(&[]), Err(InvalidEncoding));
    assert_eq!(deserialize::<[Oversize; 1]>(&[]), Err(InvalidEncoding));
    assert_eq!(
        deserialize::<HashSet<Oversize>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<HashMap<Oversize, u8>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<BTreeMap<Oversize, u8>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    let mut keyed = [0u8; 5];
    keyed[..4].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(
        deserialize::<HashMap<u8, Oversize>>(&keyed),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<BTreeMap<u8, Oversize>>(&keyed),
        Err(InvalidEncoding)
    );
    assert!(matches!(
        deserialize::<Wrap>(&[4, 0, 0, 0]),
        Err(InvalidEncoding)
    ));
}

fn assert_one_byte_short<T>()
where
    T: VerCodable + std::fmt::Debug + PartialEq,
{
    let width = std::mem::size_of::<T>();
    let buf = vec![0u8; width.saturating_sub(1)];
    assert_eq!(
        deserialize::<T>(&buf),
        Err(InvalidEncoding),
        "{}",
        std::any::type_name::<T>()
    );
}

#[test]
fn remaining_fixed_width_readers_reject_short_input() {
    assert_one_byte_short::<i8>();
    assert_one_byte_short::<i16>();
    assert_one_byte_short::<i64>();
    assert_one_byte_short::<i128>();
    assert_one_byte_short::<isize>();
    assert_one_byte_short::<f32>();
    assert_one_byte_short::<NonZeroU8>();
    assert_one_byte_short::<NonZeroU16>();
    assert_one_byte_short::<NonZeroU64>();
    assert_one_byte_short::<NonZeroU128>();
    assert_one_byte_short::<NonZeroUsize>();
    assert_one_byte_short::<NonZeroI8>();
    assert_one_byte_short::<NonZeroI16>();
    assert_one_byte_short::<NonZeroI32>();
    assert_one_byte_short::<NonZeroI64>();
    assert_one_byte_short::<NonZeroI128>();
    assert_one_byte_short::<NonZeroIsize>();
}

#[test]
fn truncated_integer_vectors_return_invalid_encoding() {
    assert_eq!(
        deserialize::<Vec<u8>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<i8>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<u16>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<i16>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<u32>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<i32>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<u64>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<i64>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<u128>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<i128>>(&1u32.to_le_bytes()),
        Err(InvalidEncoding)
    );
    // On 32-bit, this length overflows len * size_of::<u16>().
    assert_eq!(
        deserialize::<Vec<u16>>(&u32::MAX.to_le_bytes()),
        Err(InvalidEncoding)
    );
    assert_eq!(
        deserialize::<Vec<u128>>(&u32::MAX.to_le_bytes()),
        Err(InvalidEncoding)
    );
}
