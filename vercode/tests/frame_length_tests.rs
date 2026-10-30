// Copyright (c) Microsoft Corporation. All rights reserved.
use vercode::{InvalidEncoding, VerCodable, Vercode, deserialize, serialize_to_vec};

#[derive(Debug, PartialEq, Vercode)]
struct Empty;

#[derive(Debug, PartialEq, Vercode)]
struct Named {
    value: u32,
}

#[derive(Debug, PartialEq, Vercode)]
struct Tuple(u32);

#[derive(Debug, PartialEq, Vercode)]
enum Event {
    Unit,
    Tuple(u32),
    Named { value: u32 },
}

fn check_frame<T: VerCodable + std::fmt::Debug + PartialEq>(value: T) {
    let bytes = serialize_to_vec(&value);
    for end in 0..bytes.len() {
        assert_eq!(deserialize::<T>(&bytes[..end]), Err(InvalidEncoding));
    }

    for length in (0..bytes.len() as u32).chain([bytes.len() as u32 + 1, u32::MAX]) {
        let mut invalid = bytes.clone();
        invalid[..4].copy_from_slice(&length.to_le_bytes());
        assert_eq!(
            deserialize::<T>(&invalid),
            Err(InvalidEncoding),
            "accepted frame length {length} for {}",
            std::any::type_name::<T>()
        );
    }

    let mut trailing = bytes.clone();
    trailing.extend_from_slice(&[99, 100]);
    assert_eq!(
        T::read_version(T::MAX_VERSION, &trailing),
        Ok((value, bytes.len()))
    );
}

#[test]
fn structs_enforce_frame_lengths() {
    check_frame(Empty);
    check_frame(Named { value: 42 });
    check_frame(Tuple(42));
}

#[test]
fn enums_enforce_frame_lengths() {
    check_frame(Event::Unit);
    check_frame(Event::Tuple(42));
    check_frame(Event::Named { value: 42 });
    assert_eq!(
        deserialize::<Event>(&[6, 0, 0, 0, 255, 255]),
        Err(InvalidEncoding)
    );
}

#[derive(Debug, PartialEq, Vercode)]
struct Old {
    value: u8,
}

#[derive(Debug, PartialEq, Vercode)]
struct New {
    value: u8,
    #[version(1)]
    extra: u32,
}

#[derive(Debug, PartialEq, Vercode)]
struct Envelope<T: VerCodable> {
    inner: T,
    tail: u32,
}

#[test]
fn structs_consume_unknown_fields_before_the_next_value() {
    let bytes = serialize_to_vec(&New {
        value: 7,
        extra: 42,
    });
    assert_eq!(
        Old::read_version(0, &bytes),
        Ok((Old { value: 7 }, bytes.len()))
    );
    assert_eq!(
        New::read_version(0, &bytes),
        Ok((New { value: 7, extra: 0 }, bytes.len()))
    );
    assert_eq!(
        deserialize::<New>(&serialize_to_vec(&Old { value: 7 })),
        Ok(New { value: 7, extra: 0 })
    );

    let envelope = Envelope {
        inner: New {
            value: 7,
            extra: 42,
        },
        tail: 99,
    };
    assert_eq!(
        deserialize::<Envelope<Old>>(&serialize_to_vec(&envelope)),
        Ok(Envelope {
            inner: Old { value: 7 },
            tail: 99
        })
    );

    let items = vec![
        New {
            value: 7,
            extra: 42,
        },
        New {
            value: 8,
            extra: 43,
        },
    ];
    assert_eq!(
        deserialize::<Vec<Old>>(&serialize_to_vec(&items)),
        Ok(vec![Old { value: 7 }, Old { value: 8 }])
    );
    assert_eq!(
        deserialize::<(Option<Old>, u32)>(&serialize_to_vec(&(
            Some(New {
                value: 7,
                extra: 42
            }),
            99u32
        ))),
        Ok((Some(Old { value: 7 }), 99))
    );
}

#[derive(Debug, PartialEq, Vercode)]
enum OldEvent {
    Unit,
    Tuple(u8),
    Named { value: u8 },
}

#[derive(Debug, PartialEq, Vercode)]
enum NewEvent {
    Unit,
    Tuple(u8, #[version(1)] u32),
    Named {
        value: u8,
        #[version(1)]
        extra: u32,
    },
}

#[test]
fn enums_consume_unknown_fields_before_the_next_value() {
    for (new, old, defaulted) in [
        (NewEvent::Unit, OldEvent::Unit, NewEvent::Unit),
        (
            NewEvent::Tuple(7, 42),
            OldEvent::Tuple(7),
            NewEvent::Tuple(7, 0),
        ),
        (
            NewEvent::Named {
                value: 7,
                extra: 42,
            },
            OldEvent::Named { value: 7 },
            NewEvent::Named { value: 7, extra: 0 },
        ),
    ] {
        let bytes = serialize_to_vec(&new);
        assert_eq!(
            NewEvent::read_version(0, &bytes),
            Ok((defaulted, bytes.len()))
        );
        assert_eq!(
            deserialize::<(OldEvent, u32)>(&serialize_to_vec(&(new, 99u32))),
            Ok((old, 99))
        );
    }
    assert_eq!(
        deserialize::<NewEvent>(&serialize_to_vec(&OldEvent::Tuple(7))),
        Ok(NewEvent::Tuple(7, 0))
    );
}

#[test]
fn empty_structs_and_unit_variants_skip_unknown_fields() {
    assert_eq!(
        Empty::read_version(0, &[6, 0, 0, 0, 42, 43, 99]),
        Ok((Empty, 6))
    );
    assert_eq!(
        Event::read_version(0, &[8, 0, 0, 0, 0, 0, 42, 43, 99]),
        Ok((Event::Unit, 8))
    );
}

#[test]
fn nested_frames_cannot_read_past_the_parent_frame() {
    let mut bytes = serialize_to_vec(&Envelope {
        inner: Old { value: 7 },
        tail: 99,
    });
    bytes[..4].copy_from_slice(&8u32.to_le_bytes());
    assert_eq!(deserialize::<Envelope<Old>>(&bytes), Err(InvalidEncoding));
}

#[test]
fn a_partial_version_batch_is_not_defaulted_or_read_from_the_next_frame() {
    #[derive(Debug, PartialEq, Vercode)]
    struct Batch {
        value: u8,
        #[version(1)]
        first: u8,
        #[version(1)]
        second: u32,
    }

    let mut bytes = serialize_to_vec(&Batch {
        value: 7,
        first: 8,
        second: 9,
    });
    bytes[..4].copy_from_slice(&6u32.to_le_bytes());
    assert_eq!(deserialize::<Batch>(&bytes), Err(InvalidEncoding));

    let mut bytes = serialize_to_vec(&NewEvent::Tuple(7, 42));
    bytes[..4].copy_from_slice(&8u32.to_le_bytes());
    assert_eq!(deserialize::<NewEvent>(&bytes), Err(InvalidEncoding));
}
