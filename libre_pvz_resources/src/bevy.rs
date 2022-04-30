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

//! Bevy related utilities.

use bevy::prelude::*;
use bevy::asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use bincode::config::Configuration;
use bincode::decode_from_slice;
use crate::sprite::{AffineMatrix3d, Animation};

/// Asset loader for `.anim` files.
#[derive(Debug, Default)]
pub struct AnimationLoader;

impl AssetLoader for AnimationLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext)
                -> BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            const CONFIG: Configuration = bincode::config::standard();
            let (anim, n) = decode_from_slice::<Animation, _>(bytes, CONFIG)?;
            if n < bytes.len() {
                let k = bytes.len() - n;
                warn!("{k} trailing bytes ignored when loading an 'Animation' asset")
            }
            load_context.set_default_asset(LoadedAsset::new(anim));
            Ok(())
        })
    }
    fn extensions(&self) -> &[&str] { &["anim"] }
}

impl AffineMatrix3d {
    /// Convert to [`Mat4`] with a custom `z` order.
    pub fn as_mat4_with_z(&self, z: f32) -> Mat4 {
        let [[sx, kx, x], [ky, sy, y]] = self.0;
        // column major, see this as transposed
        Mat4::from_cols_array_2d(&[
            [sx, ky, 0., 0.],
            [kx, sy, 0., 0.],
            [0., 0., 1., 0.],
            [x, y, z, 1.],
        ])
    }

    /// Convert to [`Transform`] with a custom `z` order.
    pub fn as_transform_with_z(&self, z: f32) -> Transform {
        Transform::from_matrix(self.as_mat4_with_z(z))
    }
}

impl From<&'_ AffineMatrix3d> for Mat4 {
    fn from(m: &AffineMatrix3d) -> Mat4 { m.as_mat4_with_z(0.0) }
}

impl From<&'_ AffineMatrix3d> for Transform {
    fn from(m: &AffineMatrix3d) -> Transform { m.as_transform_with_z(0.0) }
}
