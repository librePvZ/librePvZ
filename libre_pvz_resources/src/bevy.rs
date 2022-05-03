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
use bevy::prelude::*;
use bevy::asset::{AssetLoader, AssetPath, BoxedFuture, LoadContext, LoadedAsset};
use bevy::utils::HashMap;
use bevy::reflect::TypeUuid;
use bevy::sprite::Anchor;
use bincode::config::Configuration;
use bincode::decode_from_slice;
use crate::sprite::{AffineMatrix3d, AnimDesc, Action, Element, Meta, Track, Frame};

/// Animation and all its dependency images.
#[derive(Debug)]
#[derive(TypeUuid)]
#[uuid = "b3eaf6b5-4c37-47a5-b2b7-b03666d7939b"]
pub struct Animation {
    /// the animation description.
    pub description: AnimDesc,
    /// all the dependency images.
    pub images: HashMap<String, Handle<Image>>,
}

impl Animation {
    /// Spawn an animation.
    pub fn spawn_on(&self, meta: &Meta, commands: &mut Commands, anims: &mut ResMut<Assets<AnimationClip>>) -> Entity {
        let root = Name::new("root");
        let parent = commands.spawn_bundle(TransformBundle {
            local: bevy::prelude::Transform {
                scale: Vec3::new(3.0, 3.0, 3.0),
                translation: Vec3::new(-120.0, 120.0, 0.0),
                ..Default::default()
            },
            ..TransformBundle::default()
        }).insert(root.clone()).id();
        let mut anim = AnimationClip::default();
        for (z, track) in self.description.tracks.iter().enumerate() {
            let this = Name::new(track.name.to_string());
            let path = EntityPath { parts: vec![root.clone(), this.clone()] };
            anim.set_curves_for_path(path, self.curves_for(track, meta, z as f32));
            let mut bundle = SpriteBundle::default();
            bundle.sprite.anchor = Anchor::TopLeft;
            let this = commands.spawn_bundle(bundle).insert(this).id();
            commands.entity(parent).add_child(this);
        }
        let mut player = AnimationPlayer::default();
        player.play(anims.add(anim)).repeat();
        commands.entity(parent).insert(player);
        parent
    }

    /// Get key frames in this track.
    pub fn curves_for(&self, track: &Track, meta: &Meta, z_order: f32) -> Vec<VariableCurve> {
        let frame_len = 1.0 / self.description.fps;

        let mut transform_timestamps = Vec::new();
        let mut rotations = Vec::new();
        let mut translations = Vec::new();
        let mut scales = Vec::new();

        let mut texture_timestamps = Vec::new();
        let mut textures = Vec::new();

        let mut visibility_timestamps = Vec::new();
        let mut visibilities = Vec::new();

        let mut alpha_timestamps = Vec::new();
        let mut alphas = Vec::new();

        let frames_until_0 = || track
            .frames[..=meta.start_frame as usize].iter()
            .flat_map(|frame| frame.0.iter());
        let frame0 = Frame([
            frames_until_0().filter(|act| matches!(act, Action::LoadElement(_))).last(),
            frames_until_0().filter(|act| matches!(act, Action::Transform(_))).last(),
            frames_until_0().filter(|act| matches!(act, Action::Alpha(_))).last(),
            frames_until_0().filter(|act| matches!(act, Action::Show(_))).last(),
        ].into_iter().flatten().cloned().collect());
        for (k, frame) in std::iter::once(&frame0)
            .chain(track.frames[(
                Bound::Excluded(meta.start_frame as usize),
                Bound::Excluded(meta.end_frame as usize)
            )].iter())
            .chain(std::iter::once(&frame0))
            .enumerate() {
            let t = frame_len * k as f32;
            for trans in frame.0.iter() {
                match trans {
                    Action::LoadElement(Element::Text { .. }) => todo!(),
                    Action::LoadElement(Element::Image { image }) => {
                        texture_timestamps.push(t);
                        textures.push(self.images[image].clone());
                    }
                    Action::Alpha(alpha) => {
                        alpha_timestamps.push(t);
                        alphas.push(*alpha);
                    }
                    Action::Show(visible) => {
                        visibility_timestamps.push(t);
                        visibilities.push(*visible);
                    }
                    Action::Transform(mat) => {
                        transform_timestamps.push(t);
                        let transform = mat.as_transform_with_z(z_order);
                        rotations.push(transform.rotation);
                        translations.push(transform.translation);
                        scales.push(transform.scale);
                    }
                }
            }
        }

        let mut curves = Vec::new();
        if !transform_timestamps.is_empty() {
            curves.extend([
                VariableCurve {
                    keyframe_timestamps: transform_timestamps.clone(),
                    keyframes: Keyframes::Rotation(rotations),
                },
                VariableCurve {
                    keyframe_timestamps: transform_timestamps.clone(),
                    keyframes: Keyframes::Translation(translations),
                },
                VariableCurve {
                    keyframe_timestamps: transform_timestamps,
                    keyframes: Keyframes::Scale(scales),
                }
            ]);
        }
        if !texture_timestamps.is_empty() {
            curves.push(VariableCurve {
                keyframe_timestamps: texture_timestamps,
                keyframes: Keyframes::Texture(textures),
            });
        }
        if !visibility_timestamps.is_empty() {
            curves.push(VariableCurve {
                keyframe_timestamps: visibility_timestamps,
                keyframes: Keyframes::Visibility(visibilities),
            });
        }
        if !alpha_timestamps.is_empty() {
            curves.push(VariableCurve {
                keyframe_timestamps: alpha_timestamps,
                keyframes: Keyframes::Alpha(alphas),
            });
        }
        curves
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
            let anim = Animation { description: anim, images };
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
