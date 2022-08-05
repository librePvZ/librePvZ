/*
 * librePvZ-resources: resource loading for librePvZ.
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

//! Serialize and deserialize as some kind of raw form (a [`String`] or a [`PathBuf`] usually),
//! used as a key indexing into some collection (a boxed slice, or the asset collection). Keep also
//! a cached handle to speed up subsequent access (an integer index, or a [`Handle`]).

use std::borrow::Borrow;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;
use std::path::PathBuf;
use bevy::asset::{Asset, AssetPath, LoadContext};
use bevy::prelude::*;
use bincode::{Encode, Decode};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use derivative::Derivative;
use once_cell::sync::OnceCell;
use serde::{Serialize, Deserialize, Deserializer};

/// Raw key storage with cached handle.
#[derive(Clone)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
#[derive(Derivative)]
#[derivative(Default(bound = "K: Default"))]
#[derivative(Hash(bound = "K: Hash"))]
#[derivative(Debug(bound = "K: Debug"), Debug = "transparent")]
#[derivative(Eq(bound = "K: Eq"), PartialEq(bound = "K: PartialEq"))]
pub struct Cached<K, I> {
    /// Raw key to be serialized to the storage.
    pub raw_key: K,
    /// Cached shortcut version for this key.
    #[derivative(Debug = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    #[serde(skip)]
    pub cached: OnceCell<I>,
}

impl<K, I> Cached<K, I> {
    /// Get from a container, and cache the handle for shortcut.
    /// If the result is a [`Some`], the handle is properly cached.
    pub fn get_or_init<'a, C>(&self, container: &'a C) -> Option<&'a C::Value>
        where C: ContainerWithKey<Handle=I>, K: Borrow<C::Key>, I: Clone {
        let handle = self.cached.get_or_try_init(|| {
            container.get_by_key(self.raw_key.borrow()).ok_or(())
        }).ok()?.clone();
        Some(container.get_by_handle(handle))
    }
}

impl<T: Asset> Cached<PathBuf, Handle<T>> {
    /// Initialise and cache the handle. Panics if called more than once.
    pub fn init_handle(&self, load_context: &mut LoadContext) {
        let asset_path = AssetPath::from(self.raw_key.as_path());
        let handle = load_context.get_handle(asset_path.get_id());
        self.cached.set(handle).unwrap();
    }

    /// Get the asset managed by Bevy.
    pub fn get<'a>(&self, assets: &'a Assets<T>) -> Option<&'a T> {
        assets.get(self.cached.get()?)
    }

    /// Get a mutable borrow of the asset managed by Bevy.
    pub fn get_mut<'a>(&self, assets: &'a mut Assets<T>) -> Option<&'a mut T> {
        assets.get_mut(self.cached.get()?)
    }
}

impl<K, I> From<K> for Cached<K, I> {
    fn from(raw_key: K) -> Self { Cached { raw_key, cached: OnceCell::new() } }
}

impl<K: Encode, I> Encode for Cached<K, I> {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.raw_key.encode(encoder)
    }
}

impl<K: Decode, I> Decode for Cached<K, I> {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Cached::from(K::decode(decoder)?))
    }
}

/// Entries in linear homogeneous collections.
pub trait EntryWithKey {
    /// Key type of this entry.
    type Key: ?Sized;
    /// Get the key in this entry.
    fn key(&self) -> &Self::Key;
}

/// Container of key-value pairs, equipped with a handle type to speed up lookup.
pub trait ContainerWithKey {
    /// Key for fast lookup: indices into arrays, [`Handle`]s in Bevy, etc.
    /// Should be cheap to clone.
    type Handle;
    /// Key type, the actual key for entries in the container.
    type Key: ?Sized;
    /// Value type, usually the entries in the container.
    type Value: ?Sized;
    /// Access the values in the container by handle. Should be a cheap operation.
    fn get_by_handle(&self, handle: Self::Handle) -> &Self::Value;
    /// Get the handle for the specific key, for access and for caching. Potentially expensive, so
    /// cache the handle somewhere to avoid calling this method repeatedly (i.e., use [`Cached`]).
    fn get_by_key(&self, key: &Self::Key) -> Option<Self::Handle>;
}

/// Raw key storage with cached handle.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[derive(Serialize, Encode)]
#[serde(transparent)]
pub struct SortedSlice<T>(Box<[T]>);

impl<T> Deref for SortedSlice<T> {
    type Target = [T];
    fn deref(&self) -> &[T] { self.0.as_ref() }
}

impl<T> AsRef<[T]> for SortedSlice<T> {
    fn as_ref(&self) -> &[T] { self.deref() }
}

impl<T> Borrow<[T]> for SortedSlice<T> {
    fn borrow(&self) -> &[T] { self.deref() }
}

impl<T: EntryWithKey> From<Box<[T]>> for SortedSlice<T>
    where T::Key: Ord {
    fn from(mut xs: Box<[T]>) -> Self {
        xs.sort_unstable_by(|x, y| Ord::cmp(x.key(), y.key()));
        SortedSlice(xs)
    }
}

impl<E: EntryWithKey> ContainerWithKey for SortedSlice<E>
    where E::Key: Ord {
    type Handle = usize;
    type Key = E::Key;
    type Value = E;
    fn get_by_handle(&self, handle: usize) -> &E { &self[handle] }
    fn get_by_key(&self, key: &E::Key) -> Option<usize> {
        self.binary_search_by(|x| x.key().cmp(key)).ok()
    }
}

impl<'de, T> Deserialize<'de> for SortedSlice<T>
    where T: EntryWithKey + Deserialize<'de>, T::Key: Ord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Box::<[T]>::deserialize(deserializer).map(SortedSlice::from)
    }
}

impl<T> Decode for SortedSlice<T>
    where T: EntryWithKey + Decode, T::Key: Ord {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        Box::<[T]>::decode(decoder).map(SortedSlice::from)
    }
}
