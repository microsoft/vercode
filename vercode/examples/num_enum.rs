// Copyright (c) Microsoft Corporation. All rights reserved.
//! Run with: cargo run -p vercode --example num_enum
//!
//! num_enum supplies the conversions; Vercode supplies integer encoding and
//! Default fallback. Use explicit, stable codes and a safe default. This is
//! not the same wire format as #[derive(Vercode)] on an enum, and fallback
//! discards the original unknown numeric value.

use num_enum::{IntoPrimitive, TryFromPrimitive};
use vercode::{Vercode, VercodeEnumU8, VercodeEnumU16, deserialize, serialize_to_vec};

#[derive(
    Debug, Copy, Clone, Default, PartialEq, IntoPrimitive, TryFromPrimitive, VercodeEnumU8,
)]
#[repr(u8)]
enum OldMode {
    Normal = 0,
    #[default]
    Unknown = 255,
}

#[derive(
    Debug, Copy, Clone, Default, PartialEq, IntoPrimitive, TryFromPrimitive, VercodeEnumU8,
)]
#[repr(u8)]
enum NewMode {
    Normal = 0,
    Turbo = 42,
    #[default]
    Unknown = 255,
}

#[derive(
    Debug, Copy, Clone, Default, PartialEq, IntoPrimitive, TryFromPrimitive, VercodeEnumU16,
)]
#[repr(u16)]
enum Status {
    Ready = 0x1234,
    #[default]
    Unknown = 0xffff,
}

#[derive(Debug, Vercode)]
struct OldConfig {
    mode: OldMode,
    status: Status,
    retries: u32,
}

#[derive(Vercode)]
struct NewConfig {
    mode: NewMode,
    status: Status,
    retries: u32,
}

fn main() -> Result<(), vercode::InvalidEncoding> {
    let bytes = serialize_to_vec(&NewConfig {
        mode: NewMode::Turbo,
        status: Status::Ready,
        retries: 42,
    });
    assert_eq!(bytes.len(), 4 + 1 + 2 + 4);

    let old = deserialize::<OldConfig>(&bytes)?;
    assert_eq!(old.mode, OldMode::Unknown);
    assert_eq!(old.status, Status::Ready);
    assert_eq!(old.retries, 42);
    println!("{old:?}");

    assert_eq!(serialize_to_vec(&Status::Ready), vec![0x34, 0x12]);
    assert_eq!(deserialize::<Status>(&[0x78, 0x56])?, Status::Unknown);
    assert_eq!(serialize_to_vec(&old.mode), vec![255]);
    Ok(())
}
