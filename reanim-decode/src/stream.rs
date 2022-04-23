/*
 * reanim-decode: decoder for PvZ reanim files.
 * Copyright (c) 2022  Ruifeng Xie
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

//! Binary streams for decoding `.reanim.compiled` files.

use std::fmt::{Display, Formatter};
use std::io::Read;
use std::string::FromUtf8Error;
use thiserror::Error;

/// Decoding errors.
#[derive(Debug, Error)]
pub enum DecodeError {
    /// Not enough bytes when decoding some `PlainData`.
    #[error("incomplete '{0}': {1}")]
    IncompleteData(&'static str, std::io::Error),
    /// Invalid byte sequence for some `PlainData`.
    #[error("invalid '{0}'")]
    InvalidData(&'static str),
    /// Incorrect magic number.
    #[error("incorrect magic: expecting '{expected_magic}', found '{real_bytes}'")]
    MagicMismatch {
        /// Real bytes in the binary file.
        real_bytes: Magic,
        /// Expected magic byte sequence.
        expected_magic: Magic,
    },
    /// Cannot decode UTF-8 strings.
    #[error("invalid UTF-8: found '{invalid_bytes:?}, after successfully decoding '{valid_prefix}''")]
    DecodeUtf8Error {
        /// The string is valid until this point.
        valid_prefix: String,
        /// The invalid bytes coming after the valid prefix.
        invalid_bytes: Box<[u8]>,
    },
    /// Superfluous bytes after decoding finished. EOF expected.
    #[error("input stream not exhausted, remaining bytes: {0:?}")]
    SuperfluousBytes(Box<[u8]>),
}

use DecodeError::*;

impl From<FromUtf8Error> for DecodeError {
    fn from(err: FromUtf8Error) -> Self {
        let utf8_error = err.utf8_error();
        let valid_up_to = utf8_error.valid_up_to();
        let invalid_to = valid_up_to + utf8_error.error_len().unwrap_or(0);
        let mut buffer = err.into_bytes();
        let invalid_bytes = buffer[valid_up_to..invalid_to].to_vec().into_boxed_slice();
        buffer.truncate(valid_up_to);
        let valid_prefix = String::from_utf8(buffer).unwrap();
        DecodeUtf8Error { valid_prefix, invalid_bytes }
    }
}

/// [Result](std::result::Result) type specialised for [`DecodeError`].
pub type Result<T> = std::result::Result<T, DecodeError>;

/// Plain old data, with a constant size known in advance.
pub trait PlainData: Sized {
    /// Size of this data in bytes.
    const SIZE_IN_BYTES: usize;
    /// Name of this type, used in diagnostics.
    const TYPE_NAME: &'static str;
    /// Decode from a byte sequence.
    ///
    /// # Note
    /// Length of the input slice is guaranteed to be `Self::SIZE_IN_BYTES`, but this information
    /// cannot be encoded in the type system (yet), due to limitations of `min_const_generics`.
    fn from_bytes(data: &[u8]) -> Option<Self>;
}

macro_rules! impl_plain_data {
    ($($type_name:ty),+) => {
        $(
            impl PlainData for $type_name {
                const SIZE_IN_BYTES: usize = std::mem::size_of::<$type_name>();
                const TYPE_NAME: &'static str = stringify!($type_name);
                fn from_bytes(data: &[u8]) -> Option<$type_name> {
                    let data: &[u8; Self::SIZE_IN_BYTES] = data.try_into().unwrap();
                    Some(<$type_name>::from_le_bytes(*data))
                }
            }
        )+
    }
}

impl_plain_data!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

impl PlainData for Option<f32> {
    const SIZE_IN_BYTES: usize = 4;
    const TYPE_NAME: &'static str = "optional f32";
    fn from_bytes(data: &[u8]) -> Option<Option<f32>> {
        let n = f32::from_bytes(data)?;
        Some(if n <= -10000.0 { None } else { Some(n) })
    }
}

/// 32bit magic sequence.
#[derive(Debug, Eq, PartialEq)]
pub struct Magic([u8; 4]);

impl From<u32> for Magic {
    fn from(n: u32) -> Self {
        Magic(n.to_le_bytes())
    }
}

impl Display for Magic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let [x, y, z, w] = self.0;
        write!(f, "{x:02X} {y:02X} {z:02X} {w:02X}")
    }
}

impl PlainData for Magic {
    const SIZE_IN_BYTES: usize = 4;
    const TYPE_NAME: &'static str = "Magic";
    fn from_bytes(data: &[u8]) -> Option<Self> {
        let data: &[u8; 4] = data.try_into().unwrap();
        Some(Magic(*data))
    }
}

/// Stream decoding API on top of [`Read`].
pub trait Stream: Read {
    /// Decode a [`PlainData`] at the start of this stream.
    fn read_data<T: PlainData>(&mut self) -> Result<T> {
        log::trace!("reading plain data '{}' ...", T::TYPE_NAME);
        // to work around current limitations around min_const_generics
        let mut buffer = vec![0_u8; T::SIZE_IN_BYTES];
        self.read_exact(&mut buffer).map_err(|err| IncompleteData(T::TYPE_NAME, err))?;
        T::from_bytes(&buffer).ok_or(InvalidData(T::TYPE_NAME))
    }

    /// Convenience function for `read::<Option<T>>`.
    fn read_optional<T>(&mut self) -> Result<Option<T>>
        where Option<T>: PlainData {
        self.read_data::<Option<T>>()
    }

    /// Decode a series of `N` [`Decode`] at the start of this stream.
    fn read_n<T: Decode>(&mut self, n: usize) -> Result<Vec<T>> {
        log::trace!("reading {n} consecutive elements ...");
        std::iter::repeat_with(|| T::decode(self)).take(n).collect()
    }

    /// Decode a length `n`, and an array of `n` [`Decode`] at the start of this stream.
    fn read_array<T: Decode>(&mut self) -> Result<Vec<T>> {
        let length = self.read_data::<u32>()?;
        self.read_n(length as usize)
    }

    /// Decode a length `n`, and then a string of length `n`.
    fn read_string(&mut self) -> Result<String> {
        let length = self.read_data::<u32>()?;
        log::trace!("reading string of length {length}");
        let mut buffer = vec![0_u8; length as usize];
        self.read_exact(&mut buffer).map_err(|err| IncompleteData("String", err))?;
        String::from_utf8(buffer).map_err(Into::into)
    }

    /// Decode and assert a 32bit magic.
    fn check_magic<M: Into<Magic>>(&mut self, magic: M) -> Result<()> {
        let magic = magic.into();
        log::trace!("checking magic {magic}");
        let val = self.read_data::<Magic>()?;
        if magic == val { Ok(()) } else {
            Err(MagicMismatch {
                real_bytes: val,
                expected_magic: magic,
            })
        }
    }

    /// Drop some information we possibly do not understand yet.
    fn drop_padding(&mut self, n: usize) -> Result<()> {
        let mut buffer = vec![0_u8; n];
        self.read_exact(&mut buffer).map_err(|err| IncompleteData("padding", err))?;
        if !buffer.iter().all(|x| *x == 0) {
            log::warn!("dropped {n} bytes of padding: {buffer:02X?}");
        } else {
            log::trace!("dropped {n} bytes of zero padding");
        }
        Ok(())
    }
}

impl<S: Read> Stream for S {}

/// Common entry for decoding binary data.
pub trait Decode: Sized {
    /// Decode complex data at current position in the [`Stream`].
    fn decode<S: Stream + ?Sized>(s: &mut S) -> Result<Self>;
}

impl<T: PlainData> Decode for T {
    fn decode<S: Stream + ?Sized>(s: &mut S) -> Result<Self> { s.read_data::<T>() }
}
