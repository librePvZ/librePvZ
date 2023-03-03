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

//! Dynamic resource support. For simplicity, we use shared global state for registry.

use std::any::TypeId;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use bevy::prelude::*;
use bevy::reflect::erased_serde::{
    Serialize as ErasedSerialize,
    Deserializer as ErasedDeserializer,
    serialize as erased_serde_serialize,
    Error,
};
use bevy::reflect::{GetTypeRegistration, TypeRegistry};
use bevy::utils::HashMap;
use bincode::{Decode, Encode};
use bincode::config::Configuration;
use bincode::de::{Decoder, DecoderImpl};
use bincode::de::read::Reader;
use bincode::enc::{Encoder, EncoderImpl};
use bincode::enc::write::Writer;
use bincode::error::{DecodeError, EncodeError};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{DeserializeOwned, DeserializeSeed, MapAccess, Visitor, Error as _};
use serde::ser::{SerializeMap, Error as _};

/// Type registry for dynamic content (de)serialization.
#[allow(missing_debug_implementations)]
pub struct DynamicRegistry {
    readable_name_to_id: RwLock<HashMap<Box<str>, TypeId>>,
    readable_name_from_id: RwLock<BTreeMap<TypeId, Box<str>>>,
    type_registry: TypeRegistry,
}

static GLOBAL_REGISTRY: OnceCell<DynamicRegistry> = OnceCell::new();

impl DynamicRegistry {
    /// Initialize the global dynamic registry. This saves the [`TypeRegistry`] provided by Bevy in
    /// the global [`DynamicRegistry`]. Panics if the global registry is already initialized.
    pub fn initialize(type_registry: TypeRegistry) {
        GLOBAL_REGISTRY.set(DynamicRegistry {
            readable_name_to_id: RwLock::new(HashMap::new()),
            readable_name_from_id: RwLock::new(BTreeMap::new()),
            type_registry,
        }).ok().expect("DynamicRegistry must not be initialized more than once")
    }

    /// Initialize the global dynamic registry, without a Bevy app.
    pub fn initialize_without_bevy() {
        DynamicRegistry::initialize(TypeRegistry::default())
    }

    /// Get the global dynamic registry. Panics if called before initialization of the registry.
    pub fn global() -> &'static DynamicRegistry {
        GLOBAL_REGISTRY.get().expect("DynamicRegistry: must initialize before use")
    }

    /// Get a shared reference to Bevy's type registry (already wrapped in `Arc`).
    pub fn get_bevy_type_registry(&self) -> &TypeRegistry { &self.type_registry }

    /// Get the [`ReflectAnyResource`] for the type registered as the given name.
    pub fn resource_by_name(&self, name: &str) -> Option<ReflectAnyResource> {
        let id = self.readable_name_to_id.read().get(name).copied()?;
        self.type_registry.read().get_type_data::<ReflectAnyResource>(id).copied()
    }

    /// Register a type for dynamic (de)serialization.
    pub fn register_dynamic<T: AnyResource + GetTypeRegistration>(&self, name: &str) {
        self.type_registry.write().register::<T>();
        self.readable_name_from_id.write().insert(TypeId::of::<T>(), name.into());
        let old = self.readable_name_to_id.write().insert(name.into(), TypeId::of::<T>());
        assert!(
            old.is_none(),
            "DynamicResource: name '{name}' is already taken by {}, cannot overwrite it with {}",
            self.type_registry.read().get_type_info(old.unwrap()).unwrap().type_name(),
            std::any::type_name::<T>(),
        );
    }
}

/// Resource types with dynamic (de)serialization. Serialization through [`bincode`] is fixed to
/// using the [`standard`](bincode::config::standard) configuration, and cannot be changed at
/// runtime, due to the current design of [`bincode`] API.
pub trait AnyResource: Reflect + ErasedSerialize + Send + Sync + 'static {
    /// Convert to a [`Reflect`] trait object.
    fn as_reflect(&self) -> &dyn Reflect;
    /// Convert to a mutable [`Reflect`] trait object.
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect;
    /// Deserialize from an [`erased`](bevy::reflect::erased_serde) deserializer.
    fn erased_deserialize(src: &mut dyn ErasedDeserializer) -> Result<Box<dyn AnyResource>, Error> where Self: Sized;
    /// Encode as [`bincode`] to a given [`Writer`] using standard configuration.
    fn erased_encode(&self, writer: &mut dyn Writer) -> Result<(), EncodeError>;
    /// Decode as [`bincode`] from a given [`Reader`] using standard configuration.
    fn erased_decode(reader: &mut dyn Reader) -> Result<Box<dyn AnyResource>, DecodeError> where Self: Sized;
}

/// [`TypeData`](bevy::reflect::TypeData) providing support for [`AnyResource`] trait.
#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
#[allow(clippy::type_complexity)]
pub struct ReflectAnyResource {
    get: fn(&dyn Reflect) -> &dyn AnyResource,
    get_mut: fn(&mut dyn Reflect) -> &mut dyn AnyResource,
    get_boxed: fn(Box<dyn Reflect>) -> Box<dyn AnyResource>,
    erased_deserialize: fn(&mut dyn ErasedDeserializer) -> Result<Box<dyn AnyResource>, Error>,
    erased_decode: fn(&mut dyn Reader) -> Result<Box<dyn AnyResource>, DecodeError>,
}

impl ReflectAnyResource {
    /// Try to downcast a `&dyn Reflect` type to `&dyn AnyResource`.
    pub fn get<'a>(&self, val: &'a dyn Reflect) -> &'a dyn AnyResource { (self.get)(val) }
    /// Try to downcast a `&mut dyn Reflect` type to `&mut dyn AnyResource`.
    pub fn get_mut<'a>(&self, val: &'a mut dyn Reflect) -> &'a mut dyn AnyResource { (self.get_mut)(val) }
    /// Try to downcast a `Box<dyn Reflect>` type to `Box<dyn AnyResource>`.
    pub fn get_boxed(&self, val: Box<dyn Reflect>) -> Box<dyn AnyResource> { (self.get_boxed)(val) }
    /// Deserialize using [`serde`] into a trait object.
    pub fn erased_deserialize(&self, src: &mut dyn ErasedDeserializer) -> Result<Box<dyn AnyResource>, Error> {
        (self.erased_deserialize)(src)
    }
    /// Deserialize using [`bincode`] into a trait object.
    pub fn erased_decode(&self, reader: &mut dyn Reader) -> Result<Box<dyn AnyResource>, DecodeError> {
        (self.erased_decode)(reader)
    }
}

const BINCODE_CONFIG: Configuration = bincode::config::standard();

struct Proxy<'a, T: ?Sized>(&'a mut T);

impl<'a> Writer for Proxy<'a, dyn Writer + 'a> {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> { self.0.write(bytes) }
}

impl<'a> Reader for Proxy<'a, dyn Reader + 'a> {
    #[inline(always)]
    fn read(&mut self, bytes: &mut [u8]) -> Result<(), DecodeError> { self.0.read(bytes) }
    #[inline(always)]
    fn peek_read(&mut self, n: usize) -> Option<&[u8]> { self.0.peek_read(n) }
    #[inline(always)]
    fn consume(&mut self, n: usize) { self.0.consume(n) }
}

impl<T> AnyResource for T
    where T: Reflect + Serialize + DeserializeOwned + Encode + Decode + Send + Sync + 'static {
    fn as_reflect(&self) -> &dyn Reflect { self }
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect { self }
    fn erased_deserialize(src: &mut dyn ErasedDeserializer) -> Result<Box<dyn AnyResource>, Error> where Self: Sized {
        T::deserialize(src).map(|x| Box::new(x) as _)
    }
    fn erased_encode(&self, writer: &mut dyn Writer) -> Result<(), EncodeError> {
        // hopefully this gets inlined, and the double reference is optimised away
        T::encode(self, &mut EncoderImpl::new(Proxy(writer), BINCODE_CONFIG))
    }
    fn erased_decode(reader: &mut dyn Reader) -> Result<Box<dyn AnyResource>, DecodeError> where Self: Sized {
        // hopefully this gets inlined, and the double reference is optimised away
        T::decode(&mut DecoderImpl::new(Proxy(reader), BINCODE_CONFIG)).map(|x| Box::new(x) as _)
    }
}

impl Debug for dyn AnyResource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { self.debug(f) }
}

struct Wrapper<'a, T: ?Sized>(&'a T);

impl<'a, T: AnyResource + ?Sized> Serialize for Wrapper<'a, T> {
    #[inline(always)]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        erased_serde_serialize(self, serializer)
    }
}

fn serialize_any_resource<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where T: AnyResource + ?Sized, S: Serializer {
    let g = DynamicRegistry::global().readable_name_from_id.read();
    let name = g.get(&value.type_id()).map(Box::as_ref)
        .ok_or_else(|| S::Error::custom(format_args!(
            "type '{}' does not support dynamic serialization", value.type_name())))?;
    let mut map = serializer.serialize_map(Some(2))?;
    map.serialize_entry(name, &Wrapper(value))?;
    map.end()
}

impl Serialize for dyn AnyResource {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_any_resource(self, serializer)
    }
}

impl<'de> DeserializeSeed<'de> for ReflectAnyResource {
    type Value = Box<dyn AnyResource>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        self.erased_deserialize(&mut <dyn ErasedDeserializer>::erase(deserializer))
            .map_err(D::Error::custom)
    }
}

impl<'de> Deserialize<'de> for Box<dyn AnyResource> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DynResVisitor;
        impl<'de> Visitor<'de> for DynResVisitor {
            type Value = Box<dyn AnyResource>;
            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a dynamic resource")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let name = map.next_key::<String>()?.ok_or_else(||
                    A::Error::custom("type tag for DynamicResource required"))?;
                let reg = DynamicRegistry::global();
                let reflect = reg.resource_by_name(&name).ok_or_else(|| A::Error::custom(
                    format_args!("type {} not registered for dynamic deserialization", name)))?;
                let result = map.next_value_seed(reflect)?;
                if map.next_key::<String>()?.is_none() { Ok(result) } else {
                    Err(A::Error::custom(format_args!("too many entries for DynamicResource '{}'", name)))
                }
            }
        }
        deserializer.deserialize_map(DynResVisitor)
    }
}

fn encode_any_resource<T, E>(value: &T, encoder: &mut E) -> Result<(), EncodeError>
    where T: AnyResource + ?Sized, E: Encoder {
    let g = DynamicRegistry::global().readable_name_from_id.read();
    let name = g.get(&value.type_id()).map(Box::as_ref)
        .ok_or_else(|| EncodeError::OtherString(format!(
            "type '{}' does not support dynamic serialization", value.type_name())))?;
    name.encode(encoder)?;
    value.erased_encode(encoder.writer())
}

impl Encode for dyn AnyResource {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encode_any_resource(self, encoder)
    }
}

impl Decode for Box<dyn AnyResource> {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let name = String::decode(decoder)?;
        let reg = DynamicRegistry::global();
        let reflect = reg.resource_by_name(&name)
            .ok_or_else(|| DecodeError::OtherString(
                format!("type {} not registered for dynamic deserialization", name)
            ))?;
        reflect.erased_decode(decoder.reader())
    }
}

/// Resource, but without a statically-known type.
#[repr(transparent)]
pub struct DynamicResource<T: ?Sized>(Box<T>);

impl<T: ?Sized> From<Box<T>> for DynamicResource<T> {
    fn from(value: Box<T>) -> Self { DynamicResource(value) }
}

impl<T: ErasedResource + ?Sized> Debug for DynamicResource<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

/// Allow dynamic (de)serialization for trait objects.
pub trait ErasedResource: AnyResource {
    /// Readable name for this type of trait objects as dynamic resource (for diagnostics).
    const RESOURCE_TYPE: &'static str;
    /// Try convert from a fully-erased dynamic resource to this trait object.
    fn try_from_erased(erased: Box<dyn AnyResource>) -> Result<DynamicResource<Self>, String>;
}

/// Mark a trait available for dynamic (de)serialization.
/// ```
/// # use bevy::reflect::reflect_trait;
/// # use libre_pvz_resources::dynamic::AnyResource;
/// # use libre_pvz_resources::mark_trait_as_dynamic_resource;
/// #[reflect_trait]
/// pub trait DoThing: AnyResource {
///     fn do_thing(&self) -> String;
/// }
/// mark_trait_as_dynamic_resource!(DoThing, ReflectDoThing);
/// ```
#[macro_export]
macro_rules! mark_trait_as_dynamic_resource {
    ($trait_name:ident, $reflect_trait:ty $(,)?) => {
        $crate::mark_trait_as_dynamic_resource! {
            $trait_name, $reflect_trait, stringify!($trait_name),
        }
    };
    ($trait_name:ident, $reflect_trait:ty, $readable_name:expr $(,)?) => {
        impl $crate::dynamic::ErasedResource for dyn $trait_name {
            const RESOURCE_TYPE: &'static str = $readable_name;
            fn try_from_erased(erased: Box<dyn $crate::dynamic::AnyResource>)
                    -> Result<$crate::dynamic::DynamicResource<Self>, String> {
                let reg = $crate::dynamic::DynamicRegistry::global().get_bevy_type_registry().read();
                let report_error = |real_type: &str| {
                    let expected = Self::RESOURCE_TYPE;
                    format!("{real_type} is not an instance of {expected}")
                };
                let reflect = reg.get_type_data::<$reflect_trait>(erased.type_id())
                    .ok_or_else(|| report_error(erased.type_name()))?;
                reflect.get_boxed(erased.into_reflect())
                    .map($crate::dynamic::DynamicResource::from)
                    .map_err(|e| report_error(e.type_name()))
            }
        }
    }
}

impl<T: ?Sized> Deref for DynamicResource<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { self.0.as_ref() }
}

impl<T: ErasedResource + ?Sized> Serialize for DynamicResource<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_any_resource(self.0.as_ref(), serializer)
    }
}

impl<'de, T: ErasedResource + ?Sized> Deserialize<'de> for DynamicResource<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let erased = Box::<dyn AnyResource>::deserialize(deserializer)?;
        T::try_from_erased(erased).map_err(D::Error::custom)
    }
}

impl<T: ErasedResource + ?Sized> Encode for DynamicResource<T> {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encode_any_resource(self.0.as_ref(), encoder)
    }
}

impl<T: ErasedResource + ?Sized> Decode for DynamicResource<T> {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let erased = Box::<dyn AnyResource>::decode(decoder)?;
        T::try_from_erased(erased).map_err(DecodeError::OtherString)
    }
}
