// Copyright (c) Microsoft Corporation. All rights reserved.
use std::collections::{BTreeMap, HashMap, HashSet};
use vercode::{
    InvalidEncoding, VerCodable, Vercode, VercodeTransparent, serialize, serialize_to_vec,
    serialize_version, size, size_version,
};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ReportedSize(usize);

impl VerCodable for ReportedSize {
    const MAX_VERSION: u32 = 0;

    fn write_version(&self, _: u32, _: &mut [u8]) -> Result<usize, InvalidEncoding> {
        panic!("a rejected size must not reach the field writer");
    }

    fn read_version(_: u32, _: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        Err(InvalidEncoding)
    }

    fn size_version(&self, _: u32) -> Result<usize, InvalidEncoding> {
        Ok(self.0)
    }
}

#[derive(Vercode)]
struct Named<T: VerCodable> {
    value: T,
}

#[derive(Vercode)]
struct Tuple<T: VerCodable>(T);

#[derive(Vercode)]
enum Event<T: VerCodable> {
    Unit,
    Tuple(T),
    Named { value: T },
}

#[derive(VercodeTransparent)]
struct Transparent<T: VerCodable + Copy>(T);

fn assert_write_error<T: VerCodable>(value: &T) {
    let mut buf = [0u8; 64];
    assert_eq!(
        value.write_version(T::MAX_VERSION, &mut buf),
        Err(InvalidEncoding)
    );
    assert_eq!(serialize(value, &mut buf), Err(InvalidEncoding));
    assert_eq!(
        serialize_version(value, T::MAX_VERSION, &mut buf),
        Err(InvalidEncoding)
    );
    assert_eq!(serialize_to_vec(value), Err(InvalidEncoding));
}

#[test]
fn derived_frame_sizes_include_the_header_and_reject_overflow() {
    let max = usize::try_from(u32::MAX).unwrap();
    assert_eq!(
        size(&Named {
            value: ReportedSize(max - 4)
        }),
        Ok(max)
    );
    assert_eq!(size(&Tuple(ReportedSize(max - 4))), Ok(max));
    assert_eq!(size(&Event::Tuple(ReportedSize(max - 6))), Ok(max));
    assert_eq!(
        size(&Event::Named {
            value: ReportedSize(max - 6)
        }),
        Ok(max)
    );
    assert_eq!(size(&Event::<ReportedSize>::Unit), Ok(6));

    for reported in [max - 3, max, usize::MAX] {
        let named = Named {
            value: ReportedSize(reported),
        };
        let tuple = Tuple(ReportedSize(reported));
        assert_eq!(size(&named), Err(InvalidEncoding));
        assert_eq!(size(&tuple), Err(InvalidEncoding));
        assert_write_error(&named);
        assert_write_error(&tuple);
    }
    for reported in [max - 5, max, usize::MAX] {
        let tuple = Event::Tuple(ReportedSize(reported));
        let named = Event::Named {
            value: ReportedSize(reported),
        };
        assert_eq!(size(&tuple), Err(InvalidEncoding));
        assert_eq!(size(&named), Err(InvalidEncoding));
        assert_write_error(&tuple);
        assert_write_error(&named);
    }
}

#[test]
fn excluded_versions_do_not_reject_oversized_fields() {
    #[derive(Vercode)]
    struct Versioned {
        value: u8,
        #[version(1)]
        extra: ReportedSize,
    }
    #[derive(Vercode)]
    enum VersionedEvent {
        Tuple(u8, #[version(1)] ReportedSize),
    }

    let value = Versioned {
        value: 7,
        extra: ReportedSize(usize::MAX),
    };
    let mut buf = [0u8; 16];
    assert_eq!(size_version(&value, 0), Ok(5));
    assert_eq!(
        serialize_version(&value, 0, &mut buf).unwrap(),
        &[5, 0, 0, 0, 7]
    );
    assert_eq!(size(&value), Err(InvalidEncoding));
    assert_write_error(&value);

    let event = VersionedEvent::Tuple(7, ReportedSize(usize::MAX));
    assert_eq!(size_version(&event, 0), Ok(7));
    assert_eq!(
        serialize_version(&event, 0, &mut buf).unwrap(),
        &[7, 0, 0, 0, 0, 0, 7]
    );
    assert_eq!(size(&event), Err(InvalidEncoding));
    assert_write_error(&event);
}

#[test]
fn size_calculation_rejects_usize_overflow() {
    let huge = ReportedSize(usize::MAX);
    assert_eq!(size(&(1u8, huge)), Err(InvalidEncoding));
    assert_eq!(size(&[huge; 2]), Err(InvalidEncoding));
    assert_eq!(size(&Some(huge)), Err(InvalidEncoding));
    assert_eq!(size(&vec![huge]), Err(InvalidEncoding));
    assert_eq!(size(&HashMap::from([(0u8, huge)])), Err(InvalidEncoding));
    assert_eq!(size(&HashMap::from([(huge, 0u8)])), Err(InvalidEncoding));
    assert_eq!(size(&BTreeMap::from([(0u8, huge)])), Err(InvalidEncoding));
    assert_eq!(size(&BTreeMap::from([(huge, 0u8)])), Err(InvalidEncoding));
    assert_eq!(size(&HashSet::from([huge])), Err(InvalidEncoding));
}

#[cfg(target_pointer_width = "64")]
#[test]
fn oversized_vector_counts_fail_without_an_allocation() {
    let too_large = usize::try_from(u32::MAX).unwrap() + 1;
    let value = vec![(); too_large];
    assert_eq!(size(&value), Err(InvalidEncoding));
    assert_write_error(&value);
}

#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Fails<const DURING_SIZE: bool>;

impl<const DURING_SIZE: bool> VerCodable for Fails<DURING_SIZE> {
    const MAX_VERSION: u32 = 0;

    fn write_version(&self, _: u32, _: &mut [u8]) -> Result<usize, InvalidEncoding> {
        Err(InvalidEncoding)
    }

    fn read_version(_: u32, _: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        Err(InvalidEncoding)
    }

    fn size_version(&self, _: u32) -> Result<usize, InvalidEncoding> {
        if DURING_SIZE {
            Err(InvalidEncoding)
        } else {
            Ok(0)
        }
    }
}

fn check_error_propagation<const DURING_SIZE: bool>() {
    let value = Fails::<DURING_SIZE>;
    assert_write_error(&value);
    assert_write_error(&Named { value });
    assert_write_error(&Tuple(value));
    assert_write_error(&Event::Tuple(value));
    assert_write_error(&Event::Named { value });
    assert_write_error(&Transparent(value));
    assert_write_error(&Some(Transparent(value)));
    assert_write_error(&Some(value));
    assert_write_error(&(value,));
    assert_write_error(&[value]);
    assert_write_error(&vec![value]);
    assert_write_error(&HashMap::from([(0u8, value)]));
    assert_write_error(&HashMap::from([(value, 0u8)]));
    assert_write_error(&BTreeMap::from([(0u8, value)]));
    assert_write_error(&BTreeMap::from([(value, 0u8)]));
    assert_write_error(&HashSet::from([value]));
}

#[test]
fn nested_size_errors_propagate() {
    check_error_propagation::<true>();
}

#[test]
fn nested_write_errors_propagate() {
    check_error_propagation::<false>();
}
