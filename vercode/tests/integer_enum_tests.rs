// Copyright (c) Microsoft Corporation. All rights reserved.
use num_enum::{IntoPrimitive, TryFromPrimitive};
use vercode::{
    InvalidEncoding, VerCodable, Vercode, VercodeEnumU8, VercodeEnumU16, deserialize,
    serialize_to_vec, serialize_version, size,
};

macro_rules! integer_enum_tests {
    ($module:ident, $int:ident, $derive:ident, $marker:ident, $new_code:expr) => {
        mod $module {
            use super::*;

            #[derive(
                Debug, Clone, Copy, Default, PartialEq, IntoPrimitive, TryFromPrimitive, $derive,
            )]
            #[repr($int)]
            enum OldMode {
                Normal = 0,
                #[default]
                Unknown = $int::MAX,
            }

            #[derive(
                Debug, Clone, Copy, Default, PartialEq, IntoPrimitive, TryFromPrimitive, $derive,
            )]
            #[repr($int)]
            enum NewMode {
                Normal = 0,
                Turbo = $new_code,
                #[default]
                Unknown = $int::MAX,
            }

            #[derive(Debug, PartialEq, Vercode)]
            struct OldConfig {
                mode: OldMode,
                retries: u32,
            }

            #[derive(Debug, PartialEq, Vercode)]
            struct NewConfig {
                mode: NewMode,
                retries: u32,
            }

            #[test]
            fn primitive_encoding_and_marker() {
                fn assert_marker<T: vercode::$marker>() {}
                assert_marker::<OldMode>();
                assert_marker::<NewMode>();
                assert_eq!(OldMode::MAX_VERSION, <$int as VerCodable>::MAX_VERSION);
                assert_eq!(NewMode::MAX_VERSION, 0);

                for mode in [NewMode::Normal, NewMode::Turbo, NewMode::Unknown] {
                    let code: $int = mode.into();
                    let bytes = serialize_to_vec(&mode);
                    assert_eq!(bytes, code.to_le_bytes());
                    assert_eq!(size(&mode), size(&code));
                    assert_eq!(deserialize::<NewMode>(&bytes).unwrap(), mode);
                    // Versions do not change the integer encoding or read size.
                    for version in [0, 1, u32::MAX] {
                        let mut buf = [0u8; 8];
                        assert_eq!(serialize_version(&mode, version, &mut buf), bytes);
                        assert_eq!(
                            NewMode::read_version(version, &bytes).unwrap(),
                            (mode, bytes.len())
                        );
                    }
                }
            }

            #[test]
            fn new_variant_defaults_without_corrupting_parent() {
                for (mode, expected) in [
                    (NewMode::Normal, OldMode::Normal),
                    (NewMode::Turbo, OldMode::Unknown),
                    (NewMode::Unknown, OldMode::Unknown),
                ] {
                    let original = NewConfig { mode, retries: 42 };
                    let bytes = serialize_to_vec(&original);
                    assert_eq!(bytes.len(), 4 + size(&mode) + 4);
                    assert_eq!(size(&original), bytes.len());
                    assert_eq!(deserialize::<NewConfig>(&bytes).unwrap(), original);
                    let old = deserialize::<OldConfig>(&bytes).unwrap();
                    assert_eq!(
                        old,
                        OldConfig {
                            mode: expected,
                            retries: 42
                        }
                    );
                    let forwarded = deserialize::<NewConfig>(&serialize_to_vec(&old)).unwrap();
                    assert_eq!(forwarded.retries, 42);
                    // Unknown is a lossy fallback, not an open-enum payload.
                    assert_eq!(
                        forwarded.mode,
                        if mode == NewMode::Normal {
                            NewMode::Normal
                        } else {
                            NewMode::Unknown
                        }
                    );
                }
            }

            #[test]
            fn every_code_decodes_or_defaults() {
                for code in $int::MIN..=$int::MAX {
                    let bytes = serialize_to_vec(&code);
                    assert_eq!(
                        deserialize::<OldMode>(&bytes).unwrap(),
                        OldMode::try_from(code).unwrap_or_default()
                    );
                    assert_eq!(
                        deserialize::<NewMode>(&bytes).unwrap(),
                        NewMode::try_from(code).unwrap_or_default()
                    );
                }
            }

            #[test]
            fn truncated_input_is_an_error_not_a_default() {
                let bytes = serialize_to_vec(&NewMode::Turbo);
                for length in 0..bytes.len() {
                    assert_eq!(
                        deserialize::<OldMode>(&bytes[..length]),
                        Err(InvalidEncoding)
                    );
                    assert_eq!(
                        deserialize::<NewMode>(&bytes[..length]),
                        Err(InvalidEncoding)
                    );
                }
            }

            #[test]
            fn options_and_collections_use_integer_encoding() {
                let source = vec![None, Some(NewMode::Turbo), Some(NewMode::Normal)];
                let bytes = serialize_to_vec(&source);
                let decoded = deserialize::<Vec<Option<OldMode>>>(&bytes).unwrap();
                assert_eq!(
                    decoded,
                    vec![None, Some(OldMode::Unknown), Some(OldMode::Normal)]
                );
                assert_eq!(size(&source), bytes.len());
                // A present but truncated option must not turn into Some(default).
                assert_eq!(deserialize::<Option<OldMode>>(&[1]), Err(InvalidEncoding));
            }

            #[test]
            fn absent_versioned_field_uses_the_same_default() {
                #[derive(Debug, PartialEq, Vercode)]
                struct Config {
                    retries: u32,
                    #[version(1)]
                    mode: OldMode,
                }
                let config = Config {
                    retries: 42,
                    mode: OldMode::Normal,
                };
                let mut buf = [0u8; 32];
                let bytes = serialize_version(&config, 0, &mut buf);
                assert_eq!(
                    deserialize::<Config>(bytes).unwrap(),
                    Config {
                        retries: 42,
                        mode: OldMode::Unknown
                    }
                );
            }
        }
    };
}

integer_enum_tests!(u8_enums, u8, VercodeEnumU8, U8Enum, 42);
integer_enum_tests!(u16_enums, u16, VercodeEnumU16, U16Enum, 0x1234);

#[test]
fn fallback_uses_default_not_a_variant_name_or_zero() {
    #[derive(
        Debug, Clone, Copy, Default, PartialEq, IntoPrimitive, TryFromPrimitive, VercodeEnumU8,
    )]
    #[repr(u8)]
    enum Mode {
        Normal = 0,
        #[default]
        Disabled = 7,
        Unknown = 255,
    }

    assert_eq!(deserialize::<Mode>(&[42]).unwrap(), Mode::Disabled);
    assert_eq!(deserialize::<Mode>(&[255]).unwrap(), Mode::Unknown);
    assert_eq!(serialize_to_vec(&Mode::default()), vec![7]);
}
