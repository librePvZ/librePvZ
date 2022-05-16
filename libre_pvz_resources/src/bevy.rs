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

use std::ops::Bound;
use std::sync::Arc;
use std::fmt::{Debug, Display, Formatter};
use bevy::prelude::*;
use bevy::asset::{AssetLoader, AssetPath, BoxedFuture, LoadContext, LoadedAsset};
use bevy::utils::HashMap;
use bevy::reflect::TypeUuid;
use bevy::sprite::Anchor;
use bitvec::prelude::*;
use bincode::config::Configuration;
use bincode::decode_from_slice;
use optics::{optics, concrete::_Identity};
use once_cell::sync::OnceCell;
use libre_pvz_animation::clip::{AnimationClip, AnimationPlayer, EntityPath};
use libre_pvz_animation::key_frame::CurveBuilder;
use crate::sprite::{AffineMatrix3d, AnimDesc, Action, Element, Track, Frame};

optics::declare_lens_from_field! {
    _Color for color as Sprite => Color;
}

optics::declare_lens! {
    _Alpha as Color => f32,
    (color) => match color {
        Color::Rgba { alpha, .. } |
        Color::RgbaLinear { alpha, .. } |
        Color::Hsla { alpha, .. } => alpha,
    }
}

impl Debug for _Alpha {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Color::alpha")
    }
}

impl Display for _Alpha {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("alpha")
    }
}

optics::declare_lens_from_field! {
    _IsVisible for is_visible as Visibility => bool;
}

/// Animation and all its dependency images.
#[derive(TypeUuid)]
#[uuid = "b3eaf6b5-4c37-47a5-b2b7-b03666d7939b"]
#[allow(missing_debug_implementations)]
pub struct Animation {
    /// the animation description.
    pub description: AnimDesc,
    /// all the dependency images.
    pub images: HashMap<String, Handle<Image>>,
    /// all the [`AnimationClip`]s generated from the [`Meta`]s.
    pub clips: Box<[OnceCell<Arc<AnimationClip>>]>,
}

impl Animation {
    /// Spawn an animation.
    pub fn spawn_on(&self, meta: &str, commands: &mut Commands) -> Option<Entity> {
        let k = self.description.meta.binary_search_by_key(&meta, |m| m.name.as_str()).ok()?;
        let clip = self.clip_for(k);
        let parent = commands.spawn_bundle(TransformBundle {
            local: bevy::prelude::Transform {
                scale: Vec3::new(3.0, 3.0, 3.0),
                translation: Vec3::new(-120.0, 120.0, 0.0),
                ..Default::default()
            },
            ..TransformBundle::default()
        }).id();
        for track in self.description.tracks.iter() {
            let this = Name::new(track.name.to_string());
            let mut bundle = SpriteBundle::default();
            bundle.sprite.anchor = Anchor::TopLeft;
            let this = commands.spawn_bundle(bundle).insert(this).id();
            commands.entity(parent).add_child(this);
        }
        let player = AnimationPlayer::new(clip, 1.0, true);
        commands.entity(parent).insert(player);
        Some(parent)
    }

    /// Accumulate the actions until some frame.
    pub fn accumulated_frame_to(&self, track: &Track, k: usize) -> Frame {
        let frames_until_k = || track
            .frames[..=k].iter()
            .flat_map(|frame| frame.0.iter());
        Frame([
            frames_until_k().filter(|act| matches!(act, Action::LoadElement(_))).last(),
            frames_until_k().filter(|act| matches!(act, Action::Transform(_))).last(),
            frames_until_k().filter(|act| matches!(act, Action::Alpha(_))).last(),
            frames_until_k().filter(|act| matches!(act, Action::Show(_))).last(),
        ].into_iter().flatten().cloned().collect())
    }

    /// Animation clip for the [`Meta`] at some index.
    pub fn clip_for(&self, k: usize) -> Arc<AnimationClip> {
        self.clips[k].get_or_init(|| {
            let frame_len = 1.0 / self.description.fps;

            let mut clip_builder = AnimationClip::builder();
            let meta = &self.description.meta[k];
            for (z_order, track) in self.description.tracks.iter().enumerate() {
                let path = EntityPath::from([Name::new(track.name.clone())]);

                let z_order = z_order as f32 * 0.1;

                let n = (meta.end_frame - meta.start_frame + 1) as usize;
                let mut transforms = CurveBuilder::<Vec<Transform>>::with_capacity(n);
                let mut textures = CurveBuilder::<Vec<Handle<Image>>>::with_capacity(n);
                let mut visibilities = CurveBuilder::<BitVec>::with_capacity(n);
                let mut alphas = CurveBuilder::<Vec<f32>>::with_capacity(n);

                let frame0 = self.accumulated_frame_to(track, meta.end_frame as usize);
                for (k, frame) in std::iter::once(&frame0)
                    .chain(track.frames[(
                        Bound::Excluded(meta.start_frame as usize),
                        Bound::Excluded(meta.end_frame as usize)
                    )].iter())
                    .chain(std::iter::once(&frame0))
                    .enumerate() {
                    for act in frame.0.iter() {
                        match act {
                            Action::LoadElement(Element::Text { .. }) => todo!(),
                            Action::LoadElement(Element::Image { image }) =>
                                textures.push_keyframe(k, self.images[image].clone()),
                            Action::Alpha(alpha) => alphas.push_keyframe(k, *alpha),
                            Action::Show(visible) => visibilities.push_keyframe(k, *visible),
                            Action::Transform(mat) =>
                                transforms.push_keyframe(k, mat.as_transform_with_z(z_order)),
                        }
                    }
                }

                for curve in [
                    transforms.finish(frame_len, _Identity::<Transform>::default()),
                    textures.finish(frame_len, _Identity::<Handle<Image>>::default()),
                    visibilities.finish(frame_len, _IsVisible),
                    alphas.finish(frame_len, optics!(_Color._Alpha)),
                ].into_iter().flatten() {
                    clip_builder.add_dyn_curve(path.clone(), curve);
                }
            }

            Arc::new(clip_builder.build())
        }).clone()
    }
}

/// Asset loader for `.anim` files.
#[derive(Debug, Default)]
pub struct AnimationLoader;

impl AssetLoader for AnimationLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext)
                -> BoxedFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            const CONFIG: Configuration = bincode::config::standard();
            let (anim, n) = decode_from_slice::<AnimDesc, _>(bytes, CONFIG)?;
            if n < bytes.len() {
                let k = bytes.len() - n;
                warn!("{k} trailing bytes ignored when loading an 'Animation' asset")
            }
            let dep_names = anim.image_files().map(str::to_string).collect::<Vec<_>>();
            let mut deps = Vec::with_capacity(dep_names.len());
            let mut images = HashMap::with_capacity(dep_names.len());
            for name in dep_names {
                let asset_path = AssetPath::from(&name).to_owned();
                images.insert(name, load_context.get_handle(asset_path.get_id()));
                deps.push(asset_path);
            }
            let clips = std::iter::repeat_with(OnceCell::new).take(anim.meta.len()).collect();
            let anim = Animation { description: anim, images, clips };
            load_context.set_default_asset(LoadedAsset::new(anim).with_dependencies(deps));
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
