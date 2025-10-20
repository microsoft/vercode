// Copyright (c) Microsoft Corporation. All rights reserved.
use vercode::{VerCodable, Vercode, deserialize, serialize, serialize_version};

#[test]
fn size_test() {
    #[derive(Default, Vercode, Debug, PartialEq)]
    struct SizeTest {
        #[version(1)]
        a: u16,
        d: u8, // defaults to 0
    }

    let st = SizeTest { a: 0x1234, d: 0x56 };
    assert_eq!(st.size_version(0), 5); // only d (1 byte) + 4 byte length prefix
    assert_eq!(st.size_version(1), 7); // a (2 bytes) + d (1 byte) + 4 byte length prefix
    assert_eq!(vercode::size(&st), 7);
}

#[derive(Default, Vercode, Debug, PartialEq, Clone)]
struct ComplexVersioned {
    #[version(2)]
    a: u8,
    #[version(0)]
    b: u8,
    #[version(1)]
    c: u8,
    d: u8, // defaults to 0
}

#[test]
fn version_ordering() {
    let v = ComplexVersioned {
        a: 10,
        b: 20,
        c: 30,
        d: 40,
    };
    let mut buf = [0u8; 16];
    let serialized = serialize(&v, &mut buf);

    // 4 bytes of data + 4 bytes length prefix = 8
    assert_eq!(serialized.len(), 8);
    // Length prefix (u32 LE): 8
    assert_eq!(&serialized[0..4], &(8u32.to_le_bytes()));
    // Data: [20,40,30,10]
    assert_eq!(&serialized[4..8], &[20, 40, 30, 10]);
}

#[derive(Default, Vercode, Debug, PartialEq, Clone)]
struct Inner {
    #[version(1)]
    a: u8,
    b: u8,
}

#[derive(Default, Vercode, Debug, PartialEq, Clone)]
struct Outer {
    #[version(1)]
    x: Inner,
    y: Inner,
}

#[test]
fn nested_versions() {
    let o = Outer {
        x: Inner { a: 10, b: 20 },
        y: Inner { a: 30, b: 40 },
    };
    let mut buf = [0u8; 32];
    let serialized = serialize(&o, &mut buf);
    // Outer: 4 (length) + 4 (y length) + 2 (y data) + 4 (x length) + 2 (x data) = 16
    assert_eq!(serialized.len(), 16);
    assert_eq!(&serialized[0..4], &(16u32.to_le_bytes()));
    // y length prefix: 6 (4 + 2 bytes of data)
    assert_eq!(&serialized[4..8], &(6u32.to_le_bytes()));
    // y data: [40,30]
    assert_eq!(&serialized[8..10], &[40, 30]);
    // x length prefix: 6 (4 + 2 bytes of data)
    assert_eq!(&serialized[10..14], &(6u32.to_le_bytes()));
    // x data: [20,10]
    assert_eq!(&serialized[14..16], &[20, 10]);
}

#[test]
fn round_trip_nested() {
    let original = Outer {
        x: Inner { a: 10, b: 20 },
        y: Inner { a: 30, b: 40 },
    };
    let mut buf = [0u8; 32];
    let serialized = serialize(&original, &mut buf);
    let decoded = deserialize::<Outer>(serialized).expect("deserialize ok");
    assert_eq!(decoded, original);
}

#[test]
fn size_version_matches_write_version() {
    let cv = ComplexVersioned {
        a: 10,
        b: 20,
        c: 30,
        d: 40,
    };
    let mut buf = [0u8; 16];
    let s0 = serialize_version(&cv, 0, &mut buf);
    assert_eq!(s0.len(), 6);
    assert_eq!(cv.size_version(0), s0.len());
    let s1 = serialize_version(&cv, 1, &mut buf);
    assert_eq!(s1.len(), 7);
    assert_eq!(cv.size_version(1), s1.len());
    let s2 = serialize_version(&cv, 2, &mut buf);
    assert_eq!(s2.len(), 8);
    assert_eq!(cv.size_version(2), s2.len());

    let o = Outer {
        x: Inner { a: 10, b: 20 },
        y: Inner { a: 30, b: 40 },
    };
    let mut buf2 = [0u8; 32];
    let sv0 = serialize_version(&o, 0, &mut buf2);
    assert_eq!(sv0.len(), 9);
    assert_eq!(o.size_version(0), sv0.len());
    let sv1 = serialize_version(&o, 1, &mut buf2);
    assert_eq!(sv1.len(), 16);
    assert_eq!(o.size_version(1), sv1.len());
    let sv2 = serialize_version(&o, 2, &mut buf2);
    assert_eq!(sv2.len(), 16);
    assert_eq!(o.size_version(2), sv2.len());
}

#[test]
fn nested_max_version_test() {
    #[derive(Default, Vercode, Debug, PartialEq, Clone)]
    struct Inner {
        a: u8,
        #[version(2)]
        b: u8,
    }

    #[derive(Default, Vercode, Debug, PartialEq, Clone)]
    struct Outer {
        x: u32,
        #[version(1)]
        y: Inner,
    }

    assert_eq!(Inner::MAX_VERSION, 2);
    assert_eq!(Outer::MAX_VERSION, 2);
}

#[test]
fn nested_max_version_enum_test() {
    #[derive(Default, Vercode)]
    struct Inner {
        a: u8,
        #[version(2)]
        b: u8,
    }

    #[derive(Vercode)]
    enum Outer {
        X {
            x: u32,
        },
        Y {
            #[version(1)]
            y: Inner,
        },
    }

    assert_eq!(Inner::MAX_VERSION, 2);
    assert_eq!(Outer::MAX_VERSION, 2);
}
