// Copyright (c) Microsoft Corporation. All rights reserved.
use vercode::{Vercode, deserialize, deserialize_version, serialize_version};

#[test]
fn array_of_structs() {
    #[derive(Default, Vercode, Debug, PartialEq, Clone, Copy)]
    struct Point {
        x: u32,
        y: u32,
    }

    #[derive(Default, Vercode, Debug, PartialEq, Clone)]
    struct Shape {
        vertices: [Point; 3],
    }

    let triangle = Shape {
        vertices: [
            Point { x: 0, y: 0 },
            Point { x: 100, y: 0 },
            Point { x: 50, y: 100 },
        ],
    };

    let mut buf = [0u8; 128];
    let serialized = vercode::serialize(&triangle, &mut buf);

    // 4 (outer length prefix) +
    // 3 * (4 (Point length prefix) + 4 (x) + 4 (y)) = 4 + 3 * 12 = 40
    assert_eq!(serialized.len(), 40);

    let decoded = deserialize::<Shape>(serialized).unwrap();
    assert_eq!(decoded, triangle);
}

#[test]
fn array_of_structs_with_versions() {
    #[derive(Default, Vercode, Debug, PartialEq, Clone, Copy)]
    struct Item {
        id: u8,
        #[version(1)]
        label: u16,
    }

    #[derive(Default, Vercode, Debug, PartialEq, Clone)]
    struct Container {
        items: [Item; 2],
    }

    let container = Container {
        items: [Item { id: 1, label: 100 }, Item { id: 2, label: 200 }],
    };

    let mut buf0 = [0u8; 128];

    // Version 0: only id fields
    let serialized_v0 = serialize_version(&container, 0, &mut buf0);
    // 4 (outer length) + 2 * (4 (Item length) + 1 (id)) = 4 + 10 = 14
    assert_eq!(serialized_v0.len(), 14);

    let decoded_v0 = deserialize_version::<Container>(0, serialized_v0).unwrap();
    assert_eq!(decoded_v0.items[0].id, 1);
    assert_eq!(decoded_v0.items[0].label, 0); // default
    assert_eq!(decoded_v0.items[1].id, 2);
    assert_eq!(decoded_v0.items[1].label, 0); // default

    // Version 1: both id and label fields
    let mut buf1 = [0u8; 128];
    let serialized_v1 = serialize_version(&container, 1, &mut buf1);
    // 4 (outer length) + 2 * (4 (Item length) + 1 (id) + 2 (label)) = 4 + 14 = 18
    assert_eq!(serialized_v1.len(), 18);

    let decoded_v1 = deserialize_version::<Container>(1, serialized_v1).unwrap();
    assert_eq!(decoded_v1, container);
}
