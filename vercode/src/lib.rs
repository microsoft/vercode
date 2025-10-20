// Copyright (c) Microsoft Corporation. All rights reserved.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::hash::{BuildHasher, Hash};
use std::num::{
    NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize, NonZeroU8,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize,
};

use uuid::Uuid;

// Re-export the Vercode attribute macros
pub use vercode_macros::{Vercode, VercodeTransparent};

/// Serialize a value into the provided buffer, returning a slice of the buffer
/// that contains the serialized data.
///
/// This is the primary serialization function that should be used in production code.
/// It serializes all fields up to the type's maximum version.
///
/// # Examples
///
/// ```
/// use vercode::{Vercode, serialize};
///
/// #[derive(Vercode, Default, Debug, PartialEq)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// let user = User {
///     id: 42,
///     name: "Alice".to_string(),
/// };
///
/// // Create a buffer to serialize into
/// let mut buf = vec![0u8; 1024];
///
/// // Serialize the user
/// let serialized = serialize(&user, &mut buf);
///
/// // The returned slice contains only the bytes that were written
/// assert!(serialized.len() > 0);
/// ```
pub fn serialize<'a, T: VerCodable>(value: &T, buf: &'a mut [u8]) -> &'a [u8] {
    let amount = value.write_version(T::MAX_VERSION, buf);
    &buf[..amount]
}

/// Serialize a value to a newly allocated `Vec<u8>`.
///
/// This is a convenience function that allocates a buffer of the correct size
/// and serializes the value into it using the maximum version.
///
/// # Example
///
/// ```
/// use vercode::{Vercode, serialize_to_vec, deserialize};
///
/// #[derive(Vercode, Debug, PartialEq)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let person = Person {
///     name: "Alice".to_string(),
///     age: 30,
/// };
///
/// // Serialize to a Vec<u8>
/// let bytes = serialize_to_vec(&person);
///
/// // Deserialize back
/// let decoded = deserialize::<Person>(&bytes).unwrap();
/// assert_eq!(person, decoded);
/// ```
pub fn serialize_to_vec<T: VerCodable>(value: &T) -> Vec<u8> {
    let size = value.size_version(T::MAX_VERSION);
    let mut buf = vec![0u8; size];
    let serialized = serialize(value, &mut buf);
    debug_assert_eq!(serialized.len(), size);
    buf
}

/// Deserialize a value from the provided buffer.
///
/// This is the primary deserialization function that should be used in production code.
/// It deserializes all fields up to the type's maximum version.
///
/// # Examples
///
/// ```
/// use vercode::{Vercode, serialize, deserialize};
///
/// #[derive(Vercode, Default, Debug, PartialEq, Clone)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// let user = User {
///     id: 42,
///     name: "Alice".to_string(),
/// };
///
/// // Serialize
/// let mut buf = vec![0u8; 1024];
/// let serialized = serialize(&user, &mut buf);
///
/// // Deserialize
/// let deserialized: User = deserialize(serialized).expect("deserialize failed");
///
/// assert_eq!(deserialized, user);
/// ```
pub fn deserialize<T: VerCodable>(buf: &[u8]) -> Result<T, InvalidEncoding> {
    let (value, _size) = T::read_version(T::MAX_VERSION, buf)?;
    Ok(value)
}

/// Return the size of the buffer that would be returned by `serialize`.
///
/// This function calculates the exact number of bytes that will be written
/// by [`serialize`] for the given value. It's useful for pre-allocating buffers
/// of the correct size.
///
/// # Examples
///
/// ```
/// use vercode::{Vercode, serialize, size};
///
/// #[derive(Vercode, Default, Debug, PartialEq)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// let user = User {
///     id: 42,
///     name: "Alice".to_string(),
/// };
///
/// // Get the size needed
/// let needed_size = size(&user);
///
/// // Allocate exactly the right amount
/// let mut buf = vec![0u8; needed_size];
/// let serialized = serialize(&user, &mut buf);
///
/// // The serialized data fills the entire buffer
/// assert_eq!(serialized.len(), needed_size);
/// ```
pub fn size<T: VerCodable>(value: &T) -> usize {
    value.size_version(T::MAX_VERSION)
}

/// Serialize a value up to a specific version.  Only fields with version less than or equal to
/// the specified version will be serialized.
///
/// **⚠️ For Testing Only**: This function is primarily intended for testing purposes.
/// In production code, use [`serialize`] instead. The serialized data always includes all
/// fields up to MAX_VERSION when communicating with older software versions, so this function
/// is not helpful for production use.
///
/// However, it can be useful for testing version compatibility since it lets you
/// create an old data format without having to duplicate struct definitions.
///
/// # Examples
///
/// ```
/// use vercode::{Vercode, serialize_version, deserialize};
///
/// #[derive(Vercode, Default, Debug, PartialEq, Clone)]
/// struct User {
///     id: u32,
///     #[version(1)]
///     name: String,
/// }
///
/// let user = User {
///     id: 42,
///     name: "Alice".to_string(),
/// };
///
/// let mut buf = vec![0u8; 1024];
///
/// // Serialize only version 0 fields (testing old format)
/// let v0_data = serialize_version(&user, 0, &mut buf);
///
/// // When deserialized, newer fields get their default values
/// let deserialized: User = deserialize(v0_data).expect("deserialize failed");
/// assert_eq!(deserialized.id, 42);
/// assert_eq!(deserialized.name, ""); // Default for String
/// ```
pub fn serialize_version<'a, T: VerCodable>(
    value: &T,
    version: u32,
    buf: &'a mut [u8],
) -> &'a [u8] {
    let amount = value.write_version(version, buf);
    &buf[..amount]
}

/// Deserialize a value from a specific version.  Only fields with version less than or equal to
/// the specified version will be deserialized; higher-version fields will be initialized with
/// default values.
///
/// **⚠️ For Testing Only**: This function is almost never useful except for testing purposes.
/// In production code, use [`deserialize`] instead.
///
/// This function can be useful in tests to simulate an older version of code reading
/// newer serialized data, or to verify that default values are correctly applied to
/// fields that weren't present in older versions.
///
/// # Examples
///
/// ```
/// use vercode::{Vercode, serialize, deserialize_version};
///
/// #[derive(Vercode, Default, Debug, PartialEq, Clone)]
/// struct Config {
///     host: String,
///     #[version(1)]
///     port: u16,
///     #[version(2)]
///     timeout_ms: u32,
/// }
///
/// let config = Config {
///     host: "localhost".to_string(),
///     port: 8080,
///     timeout_ms: 5000,
/// };
///
/// // Serialize with all fields
/// let mut buf = vec![0u8; 1024];
/// let serialized = serialize(&config, &mut buf);
///
/// // Simulate old code (version 1) reading new data
/// let old_config: Config = deserialize_version(1, serialized)
///     .expect("deserialize failed");
///
/// // Version 1 can see host and port, but timeout_ms gets default value
/// assert_eq!(old_config.host, "localhost");
/// assert_eq!(old_config.port, 8080);
/// assert_eq!(old_config.timeout_ms, 0); // Default for u32
/// ```
pub fn deserialize_version<T: VerCodable>(version: u32, buf: &[u8]) -> Result<T, InvalidEncoding> {
    let (value, _size) = T::read_version(version, buf)?;
    Ok(value)
}

/// Return the size of the buffer that would be returned by [`serialize_version`]
/// for the given version.
///
/// **⚠️ For Testing Only**: This function is primarily intended for testing purposes.
/// In production code, use [`size`] instead.
///
/// This function calculates the exact number of bytes that will be written
/// by [`serialize_version`] for a specific version. It's useful in tests to verify
/// that size calculations match actual serialized data sizes.
///
/// # Examples
///
/// ```
/// use vercode::{Vercode, serialize_version, size_version};
///
/// #[derive(Vercode, Default, Debug, Clone)]
/// struct Data {
///     id: u8,
///     #[version(1)]
///     name: String,
///     #[version(2)]
///     timestamp: u64,
/// }
///
/// let data = Data {
///     id: 1,
///     name: "test".to_string(),
///     timestamp: 1234567890,
/// };
///
/// let mut buf = vec![0u8; 1024];
///
/// // Check size for version 0 (only id field)
/// let size_v0 = size_version(&data, 0);
/// let serialized_v0 = serialize_version(&data, 0, &mut buf);
/// assert_eq!(serialized_v0.len(), size_v0);
///
/// // Check size for version 1 (id and name fields)
/// let size_v1 = size_version(&data, 1);
/// let serialized_v1 = serialize_version(&data, 1, &mut buf);
/// assert_eq!(serialized_v1.len(), size_v1);
/// ```
pub fn size_version<T: VerCodable>(value: &T, version: u32) -> usize {
    value.size_version(version)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidEncoding;

impl fmt::Display for InvalidEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid encoding")
    }
}

impl Error for InvalidEncoding {}

/// Unified versioned serialization/deserialization trait.
pub trait VerCodable: Sized {
    const MAX_VERSION: u32;

    /// Writes fields at the specified version (does not write fields
    /// with a greater version)
    fn write_version(&self, version: u32, buf: &mut [u8]) -> usize;

    /// Reads fields up to a specified version (does not read fields
    /// with a greater version). Returns the value and the number of bytes read.
    fn read_version(version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding>;

    #[inline(always)]
    fn write_option(this: Option<&Self>, version: u32, buf: &mut [u8]) -> usize {
        match this {
            Some(inner) => {
                buf[0] = 1;
                1 + inner.write_version(version, &mut buf[1..])
            }
            None => {
                buf[0] = 0;
                1
            }
        }
    }

    #[inline(always)]
    fn read_option(version: u32, buf: &[u8]) -> Result<(Option<Self>, usize), InvalidEncoding> {
        if !buf.is_empty() {
            match buf[0] {
                0 => Ok((None, 1)),
                1 => {
                    let (value, size) = Self::read_version(version, &buf[1..])?;
                    Ok((Some(value), 1 + size))
                }
                _ => Err(InvalidEncoding),
            }
        } else {
            Err(InvalidEncoding)
        }
    }

    #[inline(always)]
    fn size_option_version(this: &Option<Self>, version: u32) -> usize {
        match this {
            Some(inner) => 1 + inner.size_version(version),
            None => 1,
        }
    }

    /// Return the number of bytes that would be written by write_version for this version.
    fn size_version(&self, version: u32) -> usize;
}

impl VerCodable for bool {
    const MAX_VERSION: u32 = 0;

    #[inline(always)]
    fn write_version(&self, _version: u32, buf: &mut [u8]) -> usize {
        buf[0] = if *self { 1 } else { 0 };
        1
    }

    #[inline(always)]
    fn read_version(_version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        match buf[0] {
            0 => Ok((false, 1)),
            1 => Ok((true, 1)),
            _ => Err(InvalidEncoding),
        }
    }

    #[inline(always)]
    fn size_version(&self, _version: u32) -> usize {
        1
    }

    #[inline(always)]
    fn write_option(this: Option<&Self>, _version: u32, buf: &mut [u8]) -> usize {
        match this {
            Some(false) => buf[0] = 0,
            Some(true) => buf[0] = 1,
            None => buf[0] = 2,
        }
        1
    }

    #[inline(always)]
    fn read_option(_version: u32, buf: &[u8]) -> Result<(Option<Self>, usize), InvalidEncoding> {
        Ok((
            match buf[0] {
                0 => Some(false),
                1 => Some(true),
                2 => None,
                _ => return Err(InvalidEncoding),
            },
            1,
        ))
    }

    #[inline(always)]
    fn size_option_version(_this: &Option<Self>, _version: u32) -> usize {
        1
    }
}

macro_rules! impl_vercodable_bytes_try_as {
    ($t:ty, $encoded:ty) => {
        impl VerCodable for $t {
            const MAX_VERSION: u32 = 0;

            #[inline(always)]
            fn write_version(&self, _version: u32, buf: &mut [u8]) -> usize {
                let value: $encoded = (*self).into();
                let bytes = value.to_le_bytes();
                let len = std::mem::size_of::<$encoded>();
                buf[..len].copy_from_slice(&bytes);
                len
            }

            #[inline(always)]
            fn read_version(_version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
                let len = std::mem::size_of::<$encoded>();
                let value = <$encoded>::from_le_bytes(buf[..len].try_into().unwrap());
                let value: $t = value.try_into().map_err(|_| InvalidEncoding)?;
                Ok((value, len))
            }

            #[inline(always)]
            fn size_version(&self, _version: u32) -> usize {
                std::mem::size_of::<$t>()
            }
        }
    };
}

macro_rules! impl_vercodable_bytes_nonzero_try_as {
    ($t:ty, $encoded:ty) => {
        impl VerCodable for $t {
            const MAX_VERSION: u32 = 0;

            #[inline(always)]
            fn write_version(&self, _version: u32, buf: &mut [u8]) -> usize {
                let value: $encoded = (*self).into();
                let bytes = value.to_le_bytes();
                let len = std::mem::size_of::<$encoded>();
                buf[..len].copy_from_slice(&bytes);
                len
            }

            #[inline(always)]
            fn read_version(_version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
                let len = std::mem::size_of::<$encoded>();
                let value = <$encoded>::from_le_bytes(buf[..len].try_into().unwrap());
                let value: $t = value.try_into().map_err(|_| InvalidEncoding)?;
                Ok((value, len))
            }

            #[inline(always)]
            fn size_version(&self, _version: u32) -> usize {
                std::mem::size_of::<$t>()
            }

            #[inline(always)]
            fn write_option(this: Option<&Self>, _version: u32, buf: &mut [u8]) -> usize {
                let value: $encoded = this.map(|v| (*v).into()).unwrap_or(0);
                let bytes = value.to_le_bytes();
                let len = std::mem::size_of::<$encoded>();
                buf[..len].copy_from_slice(&bytes);
                len
            }

            #[inline(always)]
            fn read_option(
                _version: u32,
                buf: &[u8],
            ) -> Result<(Option<Self>, usize), InvalidEncoding> {
                let len = std::mem::size_of::<$encoded>();
                if len <= buf.len() {
                    let len = std::mem::size_of::<$encoded>();
                    let value = <$encoded>::from_le_bytes(
                        buf[..len].try_into().map_err(|_| InvalidEncoding)?,
                    );
                    let value = <$t>::new(value);
                    Ok((value, len))
                } else {
                    Err(InvalidEncoding)
                }
            }

            #[inline(always)]
            fn size_option_version(_this: &Option<Self>, _version: u32) -> usize {
                std::mem::size_of::<$t>()
            }
        }
    };
}

impl_vercodable_bytes_try_as! {char, u32}
impl_vercodable_bytes_try_as! {u8, u8}
impl_vercodable_bytes_try_as! {u16, u16}
impl_vercodable_bytes_try_as! {u32, u32}
impl_vercodable_bytes_try_as! {u64, u64}
impl_vercodable_bytes_try_as! {u128, u128}
impl_vercodable_bytes_nonzero_try_as! {NonZeroU8, u8}
impl_vercodable_bytes_nonzero_try_as! {NonZeroU16, u16}
impl_vercodable_bytes_nonzero_try_as! {NonZeroU32, u32}
impl_vercodable_bytes_nonzero_try_as! {NonZeroU64, u64}
impl_vercodable_bytes_nonzero_try_as! {NonZeroU128, u128}
impl_vercodable_bytes_nonzero_try_as! {NonZeroUsize, usize}
impl_vercodable_bytes_nonzero_try_as! {NonZeroI8, i8}
impl_vercodable_bytes_nonzero_try_as! {NonZeroI16, i16}
impl_vercodable_bytes_nonzero_try_as! {NonZeroI32, i32}
impl_vercodable_bytes_nonzero_try_as! {NonZeroI64, i64}
impl_vercodable_bytes_nonzero_try_as! {NonZeroI128, i128}
impl_vercodable_bytes_nonzero_try_as! {NonZeroIsize, isize}
impl_vercodable_bytes_try_as! {i8, i8}
impl_vercodable_bytes_try_as! {i16, i16}
impl_vercodable_bytes_try_as! {i32, i32}
impl_vercodable_bytes_try_as! {i64, i64}
impl_vercodable_bytes_try_as! {i128, i128}
impl_vercodable_bytes_try_as! {f32, f32}
impl_vercodable_bytes_try_as! {f64, f64}
impl_vercodable_bytes_try_as! {usize, usize}
impl_vercodable_bytes_try_as! {isize, isize}

impl VerCodable for Uuid {
    const MAX_VERSION: u32 = 0;

    #[inline(always)]
    fn write_version(&self, _version: u32, buf: &mut [u8]) -> usize {
        let bytes = self.as_bytes();
        let len = bytes.len();
        buf[..len].copy_from_slice(bytes);
        len
    }

    #[inline(always)]
    fn read_version(_version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        if buf.len() < 16 {
            return Err(InvalidEncoding);
        }
        let value = Uuid::from_slice(&buf[..16]).map_err(|_| InvalidEncoding)?;
        Ok((value, 16))
    }

    #[inline(always)]
    fn size_version(&self, _version: u32) -> usize {
        16
    }
}

impl<const N: usize, T: VerCodable + Default> VerCodable for [T; N] {
    const MAX_VERSION: u32 = T::MAX_VERSION;

    #[inline(always)]
    fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
        let mut offset = 0;
        for item in self.iter() {
            offset += item.write_version(version, &mut buf[offset..]);
        }
        offset
    }

    #[inline(always)]
    fn read_version(version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        let mut offset = 0;
        let mut error = false;
        let result: [T; N] =
            std::array::from_fn(|_| match T::read_version(version, &buf[offset..]) {
                Ok((item, read_bytes)) => {
                    offset += read_bytes;
                    item
                }
                Err(_) => {
                    error = true;
                    Default::default()
                }
            });

        if !error {
            Ok((result, offset))
        } else {
            Err(InvalidEncoding)
        }
    }

    #[inline(always)]
    fn size_version(&self, version: u32) -> usize {
        let mut total = 0;
        for item in self.iter() {
            total += item.size_version(version);
        }
        total
    }
}

impl<T: VerCodable> VerCodable for Option<T> {
    const MAX_VERSION: u32 = T::MAX_VERSION;

    #[inline(always)]
    fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
        T::write_option(self.as_ref(), version, buf)
    }

    #[inline(always)]
    fn read_version(version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        T::read_option(version, buf)
    }

    #[inline(always)]
    fn size_version(&self, version: u32) -> usize {
        T::size_option_version(self, version)
    }
}

impl VerCodable for String {
    const MAX_VERSION: u32 = 0;

    #[inline(always)]
    fn write_version(&self, _version: u32, buf: &mut [u8]) -> usize {
        let bytes = self.as_bytes();
        let len = bytes.len() as u32;
        buf[0..4].copy_from_slice(&len.to_le_bytes());
        buf[4..4 + bytes.len()].copy_from_slice(bytes);
        4 + bytes.len()
    }

    #[inline(always)]
    fn read_version(_version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        if buf.len() < 4 {
            return Err(InvalidEncoding);
        }
        let len = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
        if buf.len() < 4 + len {
            return Err(InvalidEncoding);
        }
        let string = String::from_utf8(buf[4..4 + len].to_vec()).map_err(|_| InvalidEncoding)?;
        Ok((string, 4 + len))
    }

    #[inline(always)]
    fn size_version(&self, _version: u32) -> usize {
        4 + self.len()
    }
}

impl<T: VerCodable + 'static> VerCodable for Vec<T> {
    const MAX_VERSION: u32 = T::MAX_VERSION;

    fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
        let mut offset = 0;

        // Write length as u32
        let len = self.len() as u32;
        buf[offset..offset + 4].copy_from_slice(&len.to_le_bytes());
        offset += 4;

        let special_case = std::any::TypeId::of::<T>() == std::any::TypeId::of::<u8>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u16>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u32>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u64>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u128>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i8>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i16>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i32>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i64>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i128>();

        if special_case {
            let value = self.as_slice();
            let byte_len = std::mem::size_of_val(value);
            // SAFETY: these types have same representation as their raw memory layout
            let bytes =
                unsafe { std::slice::from_raw_parts(value.as_ptr() as *const u8, byte_len) };
            buf[offset..offset + byte_len].copy_from_slice(bytes);
            offset += byte_len;
        } else {
            // Write each element
            for item in self.iter() {
                offset += item.write_version(version, &mut buf[offset..]);
            }
        }

        offset
    }

    fn read_version(version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        if buf.len() < 4 {
            return Err(InvalidEncoding);
        }

        let len = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
        let mut offset = 4;

        let mut result: Vec<T> = Vec::with_capacity(len);
        let special_case = std::any::TypeId::of::<T>() == std::any::TypeId::of::<u8>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u16>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u32>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u64>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<u128>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i8>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i16>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i32>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i64>()
            || std::any::TypeId::of::<T>() == std::any::TypeId::of::<i128>();
        if special_case {
            let byte_len = len * std::mem::size_of::<T>();
            assert!(buf.len() >= offset + byte_len);
            // SAFETY: these types have same representation as their raw memory layout
            unsafe {
                std::ptr::copy_nonoverlapping(
                    buf[offset..].as_ptr(),
                    result.as_mut_ptr() as *mut u8,
                    byte_len,
                );
                result.set_len(len);
            }
            Ok((result, offset + byte_len))
        } else {
            let mut result = Vec::with_capacity(len);
            for _ in 0..len {
                let (item, read_bytes) = T::read_version(version, &buf[offset..])?;
                result.push(item);
                offset += read_bytes;
            }
            Ok((result, offset))
        }
    }

    fn size_version(&self, version: u32) -> usize {
        let mut total = 4; // length prefix
        for item in self.iter() {
            total += item.size_version(version);
        }
        total
    }
}

impl<K, V, S> VerCodable for HashMap<K, V, S>
where
    K: VerCodable + Eq + Hash + 'static,
    V: VerCodable + 'static,
    S: BuildHasher + Default,
{
    const MAX_VERSION: u32 = {
        let mut max = K::MAX_VERSION;
        if V::MAX_VERSION > max {
            max = V::MAX_VERSION;
        }
        max
    };

    fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
        let mut offset = 0;

        // Write length as u32
        let len = self.len() as u32;
        buf[offset..offset + 4].copy_from_slice(&len.to_le_bytes());
        offset += 4;

        // Write each key-value pair
        for (key, value) in self.iter() {
            offset += key.write_version(version, &mut buf[offset..]);
            offset += value.write_version(version, &mut buf[offset..]);
        }

        offset
    }

    fn read_version(version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        if buf.len() < 4 {
            return Err(InvalidEncoding);
        }

        let len = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
        let mut offset = 4;

        let mut result = HashMap::with_capacity_and_hasher(len, S::default());
        for _ in 0..len {
            let (key, key_bytes) = K::read_version(version, &buf[offset..])?;
            offset += key_bytes;
            let (value, value_bytes) = V::read_version(version, &buf[offset..])?;
            offset += value_bytes;
            result.insert(key, value);
        }

        Ok((result, offset))
    }

    fn size_version(&self, version: u32) -> usize {
        let mut total = 4; // length prefix
        for (key, value) in self.iter() {
            total += key.size_version(version);
            total += value.size_version(version);
        }
        total
    }
}

impl<T, S> VerCodable for HashSet<T, S>
where
    T: VerCodable + Eq + Hash + 'static,
    S: BuildHasher + Default,
{
    const MAX_VERSION: u32 = T::MAX_VERSION;

    fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
        let mut offset = 0;

        // Write length as u32
        let len = self.len() as u32;
        buf[offset..offset + 4].copy_from_slice(&len.to_le_bytes());
        offset += 4;

        // Write each element
        for item in self.iter() {
            offset += item.write_version(version, &mut buf[offset..]);
        }

        offset
    }

    fn read_version(version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        if buf.len() < 4 {
            return Err(InvalidEncoding);
        }

        let len = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
        let mut offset = 4;

        let mut result = HashSet::with_capacity_and_hasher(len, S::default());
        for _ in 0..len {
            let (item, read_bytes) = T::read_version(version, &buf[offset..])?;
            result.insert(item);
            offset += read_bytes;
        }

        Ok((result, offset))
    }

    fn size_version(&self, version: u32) -> usize {
        let mut total = 4; // length prefix
        for item in self.iter() {
            total += item.size_version(version);
        }
        total
    }
}

impl<K, V> VerCodable for BTreeMap<K, V>
where
    K: VerCodable + Ord + 'static,
    V: VerCodable + 'static,
{
    const MAX_VERSION: u32 = {
        let mut max = K::MAX_VERSION;
        if V::MAX_VERSION > max {
            max = V::MAX_VERSION;
        }
        max
    };

    fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
        let mut offset = 0;

        // Write length as u32
        let len = self.len() as u32;
        buf[offset..offset + 4].copy_from_slice(&len.to_le_bytes());
        offset += 4;

        // Write each key-value pair
        for (key, value) in self.iter() {
            offset += key.write_version(version, &mut buf[offset..]);
            offset += value.write_version(version, &mut buf[offset..]);
        }

        offset
    }

    fn read_version(version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        if buf.len() < 4 {
            return Err(InvalidEncoding);
        }

        let len = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
        let mut offset = 4;

        let mut result = BTreeMap::new();
        for _ in 0..len {
            let (key, key_bytes) = K::read_version(version, &buf[offset..])?;
            offset += key_bytes;
            let (value, value_bytes) = V::read_version(version, &buf[offset..])?;
            offset += value_bytes;
            result.insert(key, value);
        }

        Ok((result, offset))
    }

    fn size_version(&self, version: u32) -> usize {
        let mut total = 4; // length prefix
        for (key, value) in self.iter() {
            total += key.size_version(version);
            total += value.size_version(version);
        }
        total
    }
}

// Unit type implementation
impl VerCodable for () {
    const MAX_VERSION: u32 = 0;

    #[inline(always)]
    fn write_version(&self, _version: u32, _buf: &mut [u8]) -> usize {
        0
    }

    #[inline(always)]
    fn read_version(_version: u32, _buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        Ok(((), 0))
    }

    #[inline(always)]
    fn size_version(&self, _version: u32) -> usize {
        0
    }
}

// Macro to implement VerCodable for tuples
macro_rules! impl_vercodable_tuple {
    ($($T:ident $idx:tt $var:ident),+) => {
        impl<$($T: VerCodable),+> VerCodable for ($($T,)+) {
            const MAX_VERSION: u32 = {
                let mut max = 0;
                $(
                    if $T::MAX_VERSION > max {
                        max = $T::MAX_VERSION;
                    }
                )+
                max
            };

            #[inline(always)]
            fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
                let mut offset = 0;
                $(
                    offset += self.$idx.write_version(version, &mut buf[offset..]);
                )+
                offset
            }

            #[inline(always)]
            fn read_version(version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
                let mut offset = 0;
                $(
                    let ($var, size) = $T::read_version(version, &buf[offset..])?;
                    offset += size;
                )+
                Ok((($($var,)+), offset))
            }

            #[inline(always)]
            fn size_version(&self, version: u32) -> usize {
                let mut total = 0;
                $(
                    total += self.$idx.size_version(version);
                )+
                total
            }
        }
    };
}

// Implement for tuples of sizes 1 through 10
impl_vercodable_tuple!(T0 0 v0);
impl_vercodable_tuple!(T0 0 v0, T1 1 v1);
impl_vercodable_tuple!(T0 0 v0, T1 1 v1, T2 2 v2);
impl_vercodable_tuple!(T0 0 v0, T1 1 v1, T2 2 v2, T3 3 v3);
impl_vercodable_tuple!(T0 0 v0, T1 1 v1, T2 2 v2, T3 3 v3, T4 4 v4);
impl_vercodable_tuple!(T0 0 v0, T1 1 v1, T2 2 v2, T3 3 v3, T4 4 v4, T5 5 v5);
impl_vercodable_tuple!(T0 0 v0, T1 1 v1, T2 2 v2, T3 3 v3, T4 4 v4, T5 5 v5, T6 6 v6);
impl_vercodable_tuple!(T0 0 v0, T1 1 v1, T2 2 v2, T3 3 v3, T4 4 v4, T5 5 v5, T6 6 v6, T7 7 v7);
impl_vercodable_tuple!(T0 0 v0, T1 1 v1, T2 2 v2, T3 3 v3, T4 4 v4, T5 5 v5, T6 6 v6, T7 7 v7, T8 8 v8);
impl_vercodable_tuple!(T0 0 v0, T1 1 v1, T2 2 v2, T3 3 v3, T4 4 v4, T5 5 v5, T6 6 v6, T7 7 v7, T8 8 v8, T9 9 v9);

impl VerCodable for std::net::SocketAddr {
    const MAX_VERSION: u32 = 0;
    fn write_version(&self, _version: u32, buf: &mut [u8]) -> usize {
        match self {
            std::net::SocketAddr::V4(socket_addr_v4) => {
                buf[0] = 4;
                let ip = socket_addr_v4.ip().to_bits();
                buf[1..5].copy_from_slice(&ip.to_le_bytes());
                buf[5..7].copy_from_slice(&socket_addr_v4.port().to_le_bytes());
                7
            }
            std::net::SocketAddr::V6(socket_addr_v6) => {
                buf[0] = 6;
                let ip = socket_addr_v6.ip().to_bits();
                buf[1..17].copy_from_slice(&ip.to_le_bytes());
                buf[17..19].copy_from_slice(&socket_addr_v6.port().to_le_bytes());
                buf[19..23].copy_from_slice(&socket_addr_v6.flowinfo().to_le_bytes());
                buf[23..27].copy_from_slice(&socket_addr_v6.scope_id().to_le_bytes());
                27
            }
        }
    }
    fn read_version(_version: u32, buf: &[u8]) -> Result<(Self, usize), InvalidEncoding> {
        if buf.len() < 7 {
            return Err(InvalidEncoding);
        }
        match buf[0] {
            4 => {
                let ip = u32::from_le_bytes(buf[1..5].try_into().unwrap());
                let ip = std::net::Ipv4Addr::from_bits(ip);
                let port = u16::from_le_bytes([buf[5], buf[6]]);
                let socket_addr = std::net::SocketAddr::V4(std::net::SocketAddrV4::new(ip, port));
                Ok((socket_addr, 7))
            }
            6 => {
                if buf.len() < 27 {
                    return Err(InvalidEncoding);
                }
                let ip: u128 = u128::from_le_bytes(buf[1..17].try_into().unwrap());
                let ip = std::net::Ipv6Addr::from_bits(ip);
                let port = u16::from_le_bytes(buf[17..19].try_into().unwrap());
                let flowinfo = u32::from_le_bytes(buf[19..23].try_into().unwrap());
                let scope_id = u32::from_le_bytes(buf[23..27].try_into().unwrap());
                let socket_addr = std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
                    ip, port, flowinfo, scope_id,
                ));
                Ok((socket_addr, 27))
            }
            _ => Err(InvalidEncoding),
        }
    }
    fn size_version(&self, _version: u32) -> usize {
        match self {
            std::net::SocketAddr::V4(_) => 7,
            std::net::SocketAddr::V6(_) => 27,
        }
    }
}
