// Copyright (c) Microsoft Corporation. All rights reserved.
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::num::{
    NonZeroI64, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
};
use std::rc::Rc;
use vercode::{
    InvalidEncoding, VerCodable, Vercode, VercodeTransparent, serialize, serialize_to_vec,
    serialize_version, size, size_version,
};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct ReportedSize(usize);

impl VerCodable for ReportedSize {
    const MAX_VERSION: u32 = 0;

    fn write_version(&self, _: u32, _: &mut [u8]) -> Result<usize, InvalidEncoding> {
        Err(InvalidEncoding)
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

struct Counted {
    value: u8,
    size_calls: Rc<Cell<usize>>,
    write_calls: Rc<Cell<usize>>,
    fail_size: bool,
}

impl VerCodable for Counted {
    const MAX_VERSION: u32 = 0;

    fn write_version(&self, version: u32, buf: &mut [u8]) -> Result<usize, InvalidEncoding> {
        self.write_calls.set(self.write_calls.get() + 1);
        self.value.write_version(version, buf)
    }

    fn read_version(_: u32, _: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        Err(InvalidEncoding)
    }

    fn size_version(&self, _: u32) -> Result<usize, InvalidEncoding> {
        self.size_calls.set(self.size_calls.get() + 1);
        if self.fail_size {
            Err(InvalidEncoding)
        } else {
            Ok(1)
        }
    }
}

#[test]
fn backpatching_does_not_size_nested_values_during_buffer_serialization() {
    let size_calls = Rc::new(Cell::new(0));
    let write_calls = Rc::new(Cell::new(0));
    let leaves = (0u8..64)
        .map(|value| Counted {
            value,
            size_calls: Rc::clone(&size_calls),
            write_calls: Rc::clone(&write_calls),
            fail_size: false,
        })
        .collect::<Vec<_>>();
    let value = Named {
        value: Event::Tuple(Tuple(leaves)),
    };
    let mut expected = vec![82, 0, 0, 0, 78, 0, 0, 0, 1, 0, 72, 0, 0, 0, 64, 0, 0, 0];
    expected.extend(0u8..64);
    let mut buf = [0u8; 128];

    assert_eq!(serialize(&value, &mut buf).unwrap(), expected);
    assert_eq!(size_calls.get(), 0);
    assert_eq!(write_calls.get(), 64);

    write_calls.set(0);
    assert_eq!(serialize_version(&value, 0, &mut buf).unwrap(), expected);
    assert_eq!(size_calls.get(), 0);
    assert_eq!(write_calls.get(), 64);

    write_calls.set(0);
    assert_eq!(serialize_to_vec(&value).unwrap(), expected);
    assert_eq!(size_calls.get(), 64);
    assert_eq!(write_calls.get(), 64);
}

#[test]
fn backpatching_uses_write_errors_not_size_errors() {
    let size_calls = Rc::new(Cell::new(0));
    let write_calls = Rc::new(Cell::new(0));
    let value = Named {
        value: Counted {
            value: 42,
            size_calls: Rc::clone(&size_calls),
            write_calls: Rc::clone(&write_calls),
            fail_size: true,
        },
    };
    let mut buf = [0u8; 5];
    assert_eq!(serialize(&value, &mut buf).unwrap(), [5, 0, 0, 0, 42]);
    assert_eq!(size_calls.get(), 0);
    assert_eq!(write_calls.get(), 1);
    assert_eq!(serialize_to_vec(&value), Err(InvalidEncoding));
    assert_eq!(size_calls.get(), 1);
    assert_eq!(write_calls.get(), 1);
}

#[test]
fn backpatching_leaves_partial_output_on_error() {
    let mut buf = [0xa5; 6];
    assert_eq!(
        Tuple((0x11u8, 0x2233u16)).write_version(0, &mut buf),
        Err(InvalidEncoding)
    );
    assert_eq!(buf, [0xa5, 0xa5, 0xa5, 0xa5, 0x11, 0xa5]);

    let mut buf = [0xa5; 8];
    assert_eq!(
        Event::Tuple((0x11u8, 0x2233u16)).write_version(0, &mut buf),
        Err(InvalidEncoding)
    );
    assert_eq!(buf, [0xa5, 0xa5, 0xa5, 0xa5, 1, 0, 0x11, 0xa5]);
}

// Deliberately violate the writer contract to exercise accounting without a 4 GiB buffer.
struct ReportedWriteSize(usize);

impl VerCodable for ReportedWriteSize {
    const MAX_VERSION: u32 = 0;

    fn write_version(&self, _: u32, _: &mut [u8]) -> Result<usize, InvalidEncoding> {
        Ok(self.0)
    }

    fn read_version(_: u32, _: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        Err(InvalidEncoding)
    }

    fn size_version(&self, _: u32) -> Result<usize, InvalidEncoding> {
        Ok(0)
    }
}

#[test]
fn backpatching_rejects_written_frame_length_overflow() {
    let max = usize::try_from(u32::MAX).unwrap();
    for reported in [max - 3, max, usize::MAX] {
        assert_write_error(&Named {
            value: ReportedWriteSize(reported),
        });
        assert_write_error(&Tuple(ReportedWriteSize(reported)));
    }
    for reported in [max - 5, max, usize::MAX] {
        assert_write_error(&Event::Tuple(ReportedWriteSize(reported)));
        assert_write_error(&Event::Named {
            value: ReportedWriteSize(reported),
        });
    }
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

fn assert_buffer_bounds<T: VerCodable>(value: &T) {
    for version in [0, T::MAX_VERSION] {
        let size = size_version(value, version).unwrap();
        let mut exact = vec![0u8; size];
        let expected = serialize_version(value, version, &mut exact)
            .unwrap()
            .to_vec();
        let mut buf = vec![0xa5; size + 1];

        for length in 0..size {
            assert_eq!(
                value.write_version(version, &mut buf[..length]),
                Err(InvalidEncoding),
                "version {version}, buffer length {length}"
            );
            assert_eq!(
                serialize_version(value, version, &mut buf[..length]),
                Err(InvalidEncoding)
            );
            if version == T::MAX_VERSION {
                assert_eq!(serialize(value, &mut buf[..length]), Err(InvalidEncoding));
            }
        }

        assert_eq!(value.write_version(version, &mut buf[..size]), Ok(size));
        assert_eq!(&buf[..size], expected);
        assert_eq!(
            serialize_version(value, version, &mut buf).unwrap(),
            expected
        );
        assert_eq!(buf[size], 0xa5);
        if version == T::MAX_VERSION {
            assert_eq!(serialize(value, &mut buf).unwrap(), expected);
        }
    }
}

#[test]
fn primitive_and_option_writers_reject_short_buffers() {
    assert_buffer_bounds(&true);
    assert_buffer_bounds(&false);
    assert_buffer_bounds(&42u8);
    assert_buffer_bounds(&42u16);
    assert_buffer_bounds(&42u32);
    assert_buffer_bounds(&42u64);
    assert_buffer_bounds(&42u128);
    assert_buffer_bounds(&42usize);
    assert_buffer_bounds(&-42i8);
    assert_buffer_bounds(&-42i16);
    assert_buffer_bounds(&-42i32);
    assert_buffer_bounds(&-42i64);
    assert_buffer_bounds(&-42i128);
    assert_buffer_bounds(&-42isize);
    assert_buffer_bounds(&1.25f32);
    assert_buffer_bounds(&1.25f64);
    assert_buffer_bounds(&'a');
    assert_buffer_bounds(&uuid::Uuid::from_bytes([42; 16]));
    assert_buffer_bounds(&NonZeroU8::new(42).unwrap());
    assert_buffer_bounds(&NonZeroU64::new(42).unwrap());
    assert_buffer_bounds(&NonZeroI64::new(-42).unwrap());
    assert_buffer_bounds(&Some(true));
    assert_buffer_bounds(&Some(false));
    assert_buffer_bounds(&None::<bool>);
    assert_buffer_bounds(&Some(42u32));
    assert_buffer_bounds(&None::<u32>);
    assert_buffer_bounds(&Some(NonZeroU8::new(42).unwrap()));
    assert_buffer_bounds(&None::<NonZeroU8>);
    assert_buffer_bounds(&Some(NonZeroU16::new(42).unwrap()));
    assert_buffer_bounds(&None::<NonZeroU16>);
    assert_buffer_bounds(&Some(NonZeroU32::new(42).unwrap()));
    assert_buffer_bounds(&None::<NonZeroU32>);
    assert_buffer_bounds(&Some(NonZeroU64::new(42).unwrap()));
    assert_buffer_bounds(&None::<NonZeroU64>);
    assert_buffer_bounds(&Some(NonZeroU128::new(42).unwrap()));
    assert_buffer_bounds(&None::<NonZeroU128>);
    assert_buffer_bounds(&Some(NonZeroUsize::new(42).unwrap()));
    assert_buffer_bounds(&None::<NonZeroUsize>);
    assert_buffer_bounds(&Some(NonZeroI64::new(-42).unwrap()));
    assert_buffer_bounds(&None::<NonZeroI64>);
    assert_buffer_bounds(&Some(Some(42u32)));
    assert_buffer_bounds(&Some(()));
    assert_buffer_bounds(&());
}

#[test]
fn collection_and_socket_writers_reject_short_buffers() {
    assert_buffer_bounds(&String::new());
    assert_buffer_bounds(&"hello".to_string());
    assert_buffer_bounds(&vec![1u8, 2, 3]);
    assert_buffer_bounds(&vec![1u16, 2, 3]);
    assert_buffer_bounds(&vec![1u32, 2, 3]);
    assert_buffer_bounds(&vec![1u64, 2, 3]);
    assert_buffer_bounds(&vec![1u128, 2, 3]);
    assert_buffer_bounds(&vec![-1i8, 2, 3]);
    assert_buffer_bounds(&vec![-1i16, 2, 3]);
    assert_buffer_bounds(&vec![-1i32, 2, 3]);
    assert_buffer_bounds(&vec![-1i64, 2, 3]);
    assert_buffer_bounds(&vec![-1i128, 2, 3]);
    assert_buffer_bounds(&vec![(); 3]);
    assert_buffer_bounds(&Vec::<u32>::new());
    assert_buffer_bounds(&Vec::<bool>::new());
    assert_buffer_bounds(&vec![true, false]);
    assert_buffer_bounds(&vec!["hello".to_string(), "world".to_string()]);
    assert_buffer_bounds(&[1u32, 2]);
    assert_buffer_bounds(&[0u32; 0]);
    assert_buffer_bounds(&(1u8, 2u32, "hello".to_string()));
    assert_buffer_bounds(&HashMap::from([(1u8, 2u64), (3, 4)]));
    assert_buffer_bounds(&HashMap::<u8, u64>::new());
    assert_buffer_bounds(&BTreeMap::from([(1u8, 2u64), (3, 4)]));
    assert_buffer_bounds(&BTreeMap::<u8, u64>::new());
    assert_buffer_bounds(&HashSet::from([1u16, 2]));
    assert_buffer_bounds(&HashSet::<u16>::new());
    assert_buffer_bounds(&"127.0.0.1:1234".parse::<std::net::SocketAddr>().unwrap());
    assert_buffer_bounds(&std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
        std::net::Ipv6Addr::LOCALHOST,
        1234,
        5678,
        9012,
    )));
}

#[test]
fn derived_writers_check_buffer_space_for_each_version() {
    #[derive(Vercode)]
    struct Versioned {
        value: u8,
        #[version(1)]
        extra: String,
    }

    #[derive(Vercode)]
    enum VersionedEvent {
        Tuple(u8, #[version(1)] String),
    }

    assert_buffer_bounds(&Named { value: 42u32 });
    assert_buffer_bounds(&Tuple(42u32));
    assert_buffer_bounds(&Event::<u32>::Unit);
    assert_buffer_bounds(&Event::Tuple(42u32));
    assert_buffer_bounds(&Event::Named { value: 42u32 });
    assert_buffer_bounds(&Transparent(42u32));
    assert_buffer_bounds(&Some(Transparent(42u32)));
    assert_buffer_bounds(&Named {
        value: vec![Tuple(42u32), Tuple(43)],
    });
    assert_buffer_bounds(&Versioned {
        value: 42,
        extra: "hello".to_string(),
    });
    assert_buffer_bounds(&VersionedEvent::Tuple(42, "hello".to_string()));

    let mut buf = [0xa5; 4];
    assert_eq!(
        Named {
            value: ReportedSize(1)
        }
        .write_version(0, &mut buf),
        Err(InvalidEncoding)
    );
    assert_eq!(
        Tuple(ReportedSize(1)).write_version(0, &mut buf),
        Err(InvalidEncoding)
    );
    assert_eq!(
        Event::Tuple(ReportedSize(1)).write_version(0, &mut buf),
        Err(InvalidEncoding)
    );
    assert_eq!(
        Event::Named {
            value: ReportedSize(1)
        }
        .write_version(0, &mut buf),
        Err(InvalidEncoding)
    );
    assert_eq!(buf, [0xa5; 4]);
}

#[test]
fn derived_writers_ignore_inherent_size_version_methods() {
    #[derive(Vercode)]
    struct Versioned {
        value: u8,
        #[version(1)]
        extra: u16,
    }

    impl Versioned {
        fn size_version(&self, _: u32) -> Result<usize, InvalidEncoding> {
            Ok(0)
        }
    }

    #[derive(Vercode)]
    enum Event {
        Unit,
        Tuple(u8, #[version(1)] u16),
    }

    impl Event {
        fn size_version(&self, _: u32) -> Result<usize, InvalidEncoding> {
            Ok(4)
        }
    }

    let value = Versioned {
        value: 42,
        extra: 0x1234,
    };
    assert_eq!(value.size_version(0), Ok(0));
    assert_buffer_bounds(&value);
    assert_eq!(
        serialize_to_vec(&value).unwrap(),
        [7, 0, 0, 0, 42, 0x34, 0x12]
    );

    let event = Event::Tuple(42, 0x1234);
    assert_eq!(event.size_version(0), Ok(4));
    assert_buffer_bounds(&Event::Unit);
    assert_buffer_bounds(&event);
    assert_eq!(
        serialize_to_vec(&event).unwrap(),
        [9, 0, 0, 0, 1, 0, 42, 0x34, 0x12]
    );
}
