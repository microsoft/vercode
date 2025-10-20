// Copyright (c) Microsoft Corporation. All rights reserved.
use vercode::{Vercode, deserialize};

#[test]
fn serialize_to_vec_test() {
    #[derive(Vercode, Debug, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    let point = Point { x: 42, y: 100 };

    // Use serialize_to_vec
    let bytes = vercode::serialize_to_vec(&point);

    // Verify we can deserialize it back
    let decoded: Point = deserialize(&bytes).unwrap();
    assert_eq!(decoded, point);

    // Verify the size is correct
    assert_eq!(bytes.len(), vercode::size(&point));
}
