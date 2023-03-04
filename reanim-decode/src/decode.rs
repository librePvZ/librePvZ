/*
 * reanim-decode: decoder for PvZ reanim files.
 * Copyright (c) 2023  Ruifeng Xie
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

//! Utilities for binary decoding.

use std::fmt::Display;
use std::io::{IoSliceMut, Read, Seek, SeekFrom};
use std::marker::PhantomData;
use std::num::TryFromIntError;
use binrw::{BinRead, binread, BinResult, Endian};

/// Provides a trivial [`Seek`] interface for any [`Read`] streams.
///
/// In contrast to [`binrw::io::NoSeek`], this wrapper supports seeking forward in the stream by
/// reading into a temporary buffer. Backward seek returns an error at runtime.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TrivialSeek<R> {
    inner: R,
    position: u64,
}

impl<R> TrivialSeek<R> {
    /// Wraps a [`Read`] and provide the [`Seek`] interface.
    pub fn new(r: R) -> Self { TrivialSeek { inner: r, position: 0 } }
}

fn update_position(result: std::io::Result<usize>, position: &mut u64) -> std::io::Result<usize> {
    if let Ok(n) = result { *position += index::<_, u64>(n)?; }
    result
}

fn index<T: TryInto<U, Error=TryFromIntError> + Display + Copy, U>(n: T) -> std::io::Result<U> {
    use std::io::{Error, ErrorKind};
    n.try_into().map_err(|err| Error::new(
        ErrorKind::InvalidInput,
        format!("index ({n}) out of bound: {err}"),
    ))
}

impl<R: Read> Read for TrivialSeek<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        update_position(self.inner.read(buf), &mut self.position)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        update_position(self.inner.read_vectored(bufs), &mut self.position)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        update_position(self.inner.read_to_end(buf), &mut self.position)
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        update_position(self.inner.read_to_string(buf), &mut self.position)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        self.inner.read_exact(buf)?;
        self.position += index::<_, u64>(buf.len())?;
        Ok(())
    }
}

impl<R: Read> TrivialSeek<R> {
    fn seek_forward(&mut self, n: usize) -> std::io::Result<u64> {
        const SMALL_BUFFER_SIZE: usize = 64;
        let mut stack_buffer = [0_u8; SMALL_BUFFER_SIZE];
        let mut heap_buffer;
        let buffer = match stack_buffer.get_mut(..n) {
            Some(buffer) => buffer,
            None => {
                heap_buffer = vec![0_u8; n];
                &mut heap_buffer
            }
        };
        self.read_exact(buffer).map(|_| self.position)
    }
}

impl<R: Read> Seek for TrivialSeek<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        use std::io::{Error, ErrorKind};
        let error = |msg: &str| Err(Error::new(ErrorKind::Other, msg));
        match pos {
            SeekFrom::Start(n) if n == self.position => Ok(n),
            SeekFrom::Start(n) if n > self.position => self.seek_forward(index(n - self.position)?),
            SeekFrom::Current(0) => Ok(self.position),
            SeekFrom::Current(n) if n > 0 => self.seek_forward(index(n)?),
            SeekFrom::Start(_) | SeekFrom::Current(_) => error("TrivialSeek: cannot seek backward"),
            SeekFrom::End(_) => error("TrivialSeek: cannot seek from the end"),
        }
    }

    fn stream_position(&mut self) -> std::io::Result<u64> { Ok(self.position) }
}

/// Parse a [`Vec<T>`] using a distinct argument for each element.
#[derive(Debug, Clone)]
pub struct ArgVec<T, A> {
    /// Contents, length determined from the number of input arguments.
    pub contents: Vec<T>,
    _input_marker: PhantomData<fn(A)>,
}

impl<T, A> ArgVec<T, A> {
    /// [`Vec::into_boxed_slice`] for the inner [`Vec<T>`].
    pub fn into_boxed_slice(self) -> Box<[T]> { self.contents.into_boxed_slice() }
}

impl<T: BinRead, A> BinRead for ArgVec<T, A>
    where for<'a> A: Into<T::Args<'a>> {
    type Args<'a> = Vec<A>;
    fn read_options<R: Read + Seek>(reader: &mut R, endian: Endian, args: Self::Args<'_>) -> BinResult<Self> {
        args.into_iter()
            .map(|a| T::read_options(reader, endian, a.into()))
            .collect::<BinResult<_>>()
            .map(|contents| ArgVec { contents, _input_marker: PhantomData })
    }
}

/// String immediately following its length.
#[binread]
#[derive(Debug, Clone)]
pub struct LenString {
    // length in bytes of this string.
    #[br(temp)]
    length: u32,
    /// Content of this string.
    #[br(count = length, try_map = String::from_utf8)]
    pub content: String,
}

impl LenString {
    /// [`String::into_boxed_str`] for the inner [`String`].
    pub fn into_boxed_str(self) -> Box<str> { self.content.into_boxed_str() }
}

/// An optional `f32`. Missing value represented as `-10000.0`.
pub fn optional_f32(x: f32) -> Option<f32> { (x > -10000.0).then_some(x) }

/// An optional `String`. Missing value represented as an empty string.
pub fn optional_string(s: LenString) -> Option<String> {
    (!s.content.is_empty()).then_some(s.content)
}
