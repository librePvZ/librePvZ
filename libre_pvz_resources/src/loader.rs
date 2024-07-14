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

//! Loaders for `bincode`, JSON, and YAML files. These files can decode into different data
//! structures, and therefore require customisation over bevy's [`AssetLoader`]. We do so by
//! requiring an additional "secondary extension" in asset file names. For example, a file named
//! "`Peashooter.anim.bin`" is treated as encoded in `bincode`, and has a resource type "`anim`".

use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use anyhow::{Error, Result};
use bevy::prelude::*;
use bevy::asset::{Asset, AssetLoader, AssetPath, AsyncReadExt, LoadContext};
use bevy::asset::io::Reader;
use bevy::log::warn;
use bevy::utils::ConditionalSendFuture;
use bincode::Decode;
use derivative::Derivative;
use serde::de::DeserializeOwned;

/// List of `str`, with static lifetime all the way down.
pub type StrList = &'static [&'static str];

/// File extensions for two-stage assets. See also [`asset_ext`](crate::asset_ext).
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AssetExtensions {
    /// File extensions for YAML file storage, e.g., `["anim.yaml", "anim.yml"]`.
    pub yaml: StrList,
    /// File extensions for JSON file storage, e.g., `["anim.json"]`.
    pub json: StrList,
    /// File extensions for JSON file storage, e.g., `["anim.bin"]`.
    pub bin: StrList,
}

/// Generate proper values for [`AssetExtensions`]. Always prefer this macro instead of manually
/// constructing [`AssetExtensions`] values so that the program behaviour is consistent.
///
/// ```
/// # use libre_pvz_resources::asset_ext;
/// use libre_pvz_resources::loader::AssetExtensions;
/// assert_eq!(asset_ext!("anim"), AssetExtensions {
///     yaml: &["anim.yaml", "anim.yml"],
///     json: &["anim.json"],
///     bin: &["anim.bin"],
/// });
/// ```
#[macro_export]
macro_rules! asset_ext {
    ($ext:literal) => {
        $crate::loader::AssetExtensions {
            yaml: &[::std::concat!($ext, ".yaml"), ::std::concat!($ext, ".yml")],
            json: &[::std::concat!($ext, ".json")],
            bin: &[::std::concat!($ext, ".bin")],
        }
    }
}

/// A new trait for two-stage asset loading, in place of [`AssetLoader`]:
/// - Decode from `bincode`/JSON/YAML etc. to get structural data
/// - Post-processing the structural data, transforming & adding dependencies
/// This is meant to support loading the same data structures stored in different serialised forms,
/// and share their post-processing logic.
pub trait TwoStageAsset: Asset + Sized {
    /// The decoded representation for this asset.
    type Repr: Decode + DeserializeOwned;
    /// The file extensions this asset is associated to.
    const EXTENSIONS: AssetExtensions;
    /// The post-processing logic: transform the `Repr` to a more compact in-memory form, require
    /// loading the dependencies and store their handles in the appropriate locations, and submit
    /// the resulting asset to the asset loader.
    fn post_process(repr: Self::Repr, load_context: &mut LoadContext) -> Result<(Self, Vec<AssetPath<'static>>)>;
}

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

/// Frontend asset format.
pub trait AssetFormat: Copy + Send + Sync + 'static {
    /// Get the extensions list.
    fn get_extension(self, extensions: AssetExtensions) -> StrList;
    /// Load from raw bytes to intermediate representation.
    fn load_raw<T: Decode + DeserializeOwned>(self, src: &[u8]) -> Result<T>;
}

/// [JSON](serde_json) format.
#[derive(Default, Debug, Copy, Clone)]
pub struct Json;

impl AssetFormat for Json {
    fn get_extension(self, extensions: AssetExtensions) -> StrList { extensions.json }
    fn load_raw<T: Decode + DeserializeOwned>(self, src: &[u8]) -> Result<T> {
        serde_json::from_slice(src).map_err(Error::from)
    }
}

/// [YAML](serde_yaml) format.
#[derive(Default, Debug, Copy, Clone)]
pub struct Yaml;

impl AssetFormat for Yaml {
    fn get_extension(self, extensions: AssetExtensions) -> StrList { extensions.yaml }
    fn load_raw<T: Decode + DeserializeOwned>(self, src: &[u8]) -> Result<T> {
        serde_yaml::from_slice(src).map_err(Error::from)
    }
}

/// [`bincode`] format.
#[derive(Default, Debug, Copy, Clone)]
pub struct Bincode;

impl AssetFormat for Bincode {
    fn get_extension(self, extensions: AssetExtensions) -> StrList { extensions.bin }
    fn load_raw<T: Decode + DeserializeOwned>(self, src: &[u8]) -> Result<T> {
        let (content, n) = bincode::decode_from_slice(src, BINCODE_CONFIG)?;
        if n < src.len() {
            let k = src.len() - n;
            warn!("{k} trailing bytes ignored when loading {}", std::any::type_name::<T>())
        }
        Ok(content)
    }
}

/// Asset loader for [`TwoStageAsset`]s.
#[derive(Copy, Clone)]
#[derive(Derivative)]
#[derivative(Default(bound = "Fmt: Default"))]
pub struct TwoStageAssetLoader<T, Fmt>(Fmt, PhantomData<fn() -> T>);

impl<T, Fmt: Debug> Debug for TwoStageAssetLoader<T, Fmt> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TwoStageAssetLoader({:?}, {})", self.0, std::any::type_name::<T>())
    }
}

impl<T: TwoStageAsset, Fmt: AssetFormat> AssetLoader for TwoStageAssetLoader<T, Fmt> {
    type Asset = T;
    type Settings = ();
    // TODO: consider using `thiserror` to define a proper error type.
    type Error = Error;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset>> {
        async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            // TODO: redesign `AssetFormat::load_raw` to use async?
            let raw = self.0.load_raw::<T::Repr>(&bytes)?;
            // TODO: check how dependencies are managed, redesign `TwoStageAsset::post_process`
            let (res, _) = T::post_process(raw, load_context)?;
            Ok(res)
        }
    }
    fn extensions(&self) -> &[&str] { self.0.get_extension(T::EXTENSIONS) }
}

/// Extension to the [`App`] API for adding two-stage asset loaders.
pub trait AddTwoStageAsset {
    /// Register a two-stage asset to the app.
    fn add_two_stage_asset<T: TwoStageAsset>(&mut self) -> &mut Self;
}

impl AddTwoStageAsset for App {
    fn add_two_stage_asset<T: TwoStageAsset>(&mut self) -> &mut App {
        // TODO: make type registry local to the loader
        // let registry = self.world.resource_mut::<AppTypeRegistry>();
        self.init_asset::<T>()
            .register_asset_loader(TwoStageAssetLoader::<T, Json>::default())
            .register_asset_loader(TwoStageAssetLoader::<T, Yaml>::default())
            .register_asset_loader(TwoStageAssetLoader::<T, Bincode>::default())
    }
}
