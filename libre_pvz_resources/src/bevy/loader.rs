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

use std::fmt::{Display, Formatter, Write};
use std::path::Path;
use std::sync::Arc;
use anyhow::{Context, Result};
use parking_lot::RwLock;
use bevy::asset::{Asset, AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use bevy::prelude::*;
use bevy::log::warn;
use bevy::utils::HashMap;
use bincode::Decode;
use serde::de::DeserializeOwned;

/// A new trait for two-stage asset loading, in place of [`AssetLoader`]:
/// - Decode from `bincode`/JSON/YAML etc. to get structural data
/// - Post-processing the structural data, transforming & adding dependencies
/// This is meant to support loading the same data structures stored in different serialised forms,
/// and share their post-processing logic.
pub trait TwoStageAssetLoader: Send + Sync + 'static {
    /// The decoded representation for this asset.
    type Repr: Decode + DeserializeOwned;
    /// The "secondary file extension" this asset is associated to.
    fn extension(&self) -> &str;
    /// The post-processing logic: transform the `Repr` to a more compact in-memory form, require
    /// loading the dependencies and store their handles in the appropriate locations, and submit
    /// the resulting asset to the asset loader.
    fn post_process(&self, repr: Self::Repr, load_context: &mut LoadContext) -> Result<()>;
}

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Format { Bin, Json, Yaml }

impl Display for Format {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Format::Bin => "binary",
            Format::Json => "JSON",
            Format::Yaml => "YAML",
        })
    }
}

trait DynLoader: Send + Sync + 'static {
    fn extension(&self) -> &str;
    fn load(&self, ext: Format, src: &[u8], load_context: &mut LoadContext) -> Result<()>;
}

impl<L: TwoStageAssetLoader> DynLoader for L {
    fn extension(&self) -> &str { TwoStageAssetLoader::extension(self) }
    fn load(&self, ext: Format, src: &[u8], load_context: &mut LoadContext) -> Result<()> {
        let repr = match ext {
            Format::Bin => {
                let (content, n) = bincode::decode_from_slice(src, BINCODE_CONFIG)?;
                if n < src.len() {
                    let k = src.len() - n;
                    warn!("{k} trailing bytes ignored when loading a {ext} asset")
                }
                content
            }
            Format::Json => serde_json::from_slice(src)?,
            Format::Yaml => serde_yaml::from_slice(src)?,
        };
        self.post_process(repr, load_context)
    }
}

/// Default post processing: no processing at all, submit the decoded data as loaded asset.
pub fn default_post_process<R: Asset>(repr: R, load_context: &mut LoadContext) -> Result<()> {
    load_context.set_default_asset(LoadedAsset::new(repr));
    Ok(())
}

/// Registry for asset loaders.
#[allow(missing_debug_implementations)]
pub struct LoaderRegistry {
    /// The shared file extension for this loader registry, e.g., "yaml", "json", "bin", etc.
    extensions: &'static [&'static str],
    /// List of loaders in this registry, indexed by asset type name, e.g., "anim", "model", etc.
    loaders: RwLock<HashMap<Box<str>, Arc<dyn DynLoader>>>,
}

impl LoaderRegistry {
    /// Create a new loader registry with the given file extension.
    pub fn new(extensions: &'static [&'static str]) -> LoaderRegistry {
        assert!(!extensions.is_empty());
        LoaderRegistry { extensions, loaders: RwLock::new(HashMap::new()) }
    }

    /// Add a loader for the specific type into the registry. If the type is already registered, a
    /// warning is emitted, and the loader is overwritten.
    pub fn add_loader<T: TwoStageAssetLoader>(&self, loader: T) {
        let loader = Arc::new(loader);
        let ext = loader.extension();
        let mut loaders = self.loaders.write();
        let old = loaders.insert(ext.into(), loader as _);
        if let Some(old) = old {
            warn!("overwriting asset loader for '*.{}.{}'",
                old.extension(), ManyDisplay(self.extensions));
        }
    }
}

impl AssetLoader for LoaderRegistry {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let path = load_context.path();
            // this method gets called, there must be an extension.
            let ext = path.extension().and_then(|ext| ext.to_str()).unwrap();
            let minor_ext = path.file_stem()
                .and_then(|stem| Path::extension(stem.as_ref()))
                .and_then(|ext| ext.to_str())
                .with_context(|| format!("cannot determine file type from path: {}", path.display()))?;
            let loader = self.loaders.read().get(minor_ext).cloned()
                .with_context(|| format!("no loader exists for '*.{minor_ext}.{ext}' files"))?;
            let ext = match ext {
                "bin" => Format::Bin,
                "json" => Format::Json,
                "yaml" | "yml" => Format::Yaml,
                _ => unreachable!(),
            };
            loader.load(ext, bytes, load_context)
        })
    }
    fn extensions(&self) -> &[&str] { self.extensions }
}

struct ManyDisplay<'a, T>(&'a [T]);

impl<'a, T: Display> Display for ManyDisplay<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            [] => unreachable!(),
            [x] => x.fmt(f),
            [x, xs @ ..] => {
                f.write_char('{')?;
                x.fmt(f)?;
                for x in xs { write!(f, ",{}", x)?; }
                f.write_char('}')
            }
        }
    }
}

/// Plugin for two-stage asset loading.
#[derive(Default, Debug, Copy, Clone)]
pub struct TwoStageAssetPlugin;

impl Plugin for TwoStageAssetPlugin {
    fn build(&self, app: &mut App) {
        const EXTENSIONS: &[&str] = &["bin", "json", "yaml", "yml"];
        app.insert_resource(LoaderRegistry::new(EXTENSIONS));
    }
}

/// Extension to the [`App`] API for adding two-stage asset loaders.
pub trait AddTwoStageAsset {
    /// Add a two-stage asset loader to the app.
    fn add_two_stage_asset_loader<L>(&mut self, loader: L) -> &mut Self
        where L: TwoStageAssetLoader;
    /// Initialise a two-stage asset loader from the current world status.
    fn init_two_stage_asset_loader<L>(&mut self) -> &mut Self
        where L: TwoStageAssetLoader + FromWorld;
    /// Freeze the loader registry and submit to asset server.
    fn freeze_two_stage_asset_loaders(&mut self) -> &mut Self;
}

impl AddTwoStageAsset for App {
    fn add_two_stage_asset_loader<L>(&mut self, loader: L) -> &mut Self
        where L: TwoStageAssetLoader {
        self.world.get_resource_mut::<LoaderRegistry>()
            .expect("add loaders before calling 'freeze'")
            .add_loader(loader);
        self
    }

    fn init_two_stage_asset_loader<L>(&mut self) -> &mut Self
        where L: TwoStageAssetLoader + FromWorld {
        let loader = L::from_world(&mut self.world);
        self.add_two_stage_asset_loader(loader)
    }

    fn freeze_two_stage_asset_loaders(&mut self) -> &mut Self {
        let reg = self.world.remove_resource::<LoaderRegistry>()
            .expect("add TwoStageAssetPlugin to use relevant APIs");
        self.add_asset_loader(reg)
    }
}
