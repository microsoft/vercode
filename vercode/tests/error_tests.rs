// Copyright (c) Microsoft Corporation. All rights reserved.
use std::error::Error;
use vercode::InvalidEncoding;

#[test]
fn invalid_encoding_error_traits() {
    let err = InvalidEncoding;

    // Test Display trait
    let display_str = format!("{err}");
    assert_eq!(display_str, "invalid encoding");

    // Test Debug trait
    let debug_str = format!("{err:?}");
    assert_eq!(debug_str, "InvalidEncoding");

    // Test that it can be used as an Error trait object
    let _: &dyn Error = &err;

    // Test Error trait methods
    assert!(err.source().is_none());
}

#[test]
fn invalid_encoding_error_in_result() {
    fn returns_error() -> Result<(), Box<dyn Error>> {
        Err(Box::new(InvalidEncoding))
    }

    let result = returns_error();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.to_string(), "invalid encoding");
}
