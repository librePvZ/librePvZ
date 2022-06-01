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
use libre_pvz_animation::key_frame::{ConstCurve, CurveBuilder};
use libre_pvz_animation::transform::{SpriteBundle2D, Transform2D, TransformBundle2D};
use crate::sprite::{AnimDesc, Action, Element, Track, Frame};

/// Resources plugin.
#[derive(Default, Debug, Copy, Clone)]
pub struct ResourcesPlugin;

impl Plugin for ResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Animation>()
            .init_asset_loader::<AnimationLoader>();
    }
}

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

optics::declare_lens_from_field! {
    _Translation for translation as Transform2D => Vec2;
    _Scale for scale as Transform2D => Vec2;
    _Rotation for rotation as Transform2D => Vec2;
    _ZOrder for z_order as Transform2D => f32;
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
    /// all the [`AnimationClip`]s generated from the [`Meta`](crate::sprite::Meta)s.
    pub clips: Box<[OnceCell<Arc<AnimationClip>>]>,
}

const WIDTH: f32 = 80.0;
const HALF_WIDTH: f32 = WIDTH / 2.0;

impl Animation {
    /// Spawn an animation.
    pub fn spawn_on(&self, meta: usize, commands: &mut Commands) -> Entity {
        let clip = self.clip_for(meta);
        let parent = commands.spawn_bundle(TransformBundle2D {
            local: Transform2D::from_translation(Vec2::new(-HALF_WIDTH, HALF_WIDTH)),
            ..TransformBundle2D::default()
        }).id();
        for track in self.description.tracks.iter() {
            let this = Name::new(track.name.to_string());
            let mut bundle = SpriteBundle2D::default();
            bundle.sprite.anchor = Anchor::TopLeft;
            let this = commands.spawn_bundle(bundle).insert(this).id();
            commands.entity(parent).add_child(this);
        }
        let player = AnimationPlayer::new(clip, 1.0, true);
        commands.entity(parent).insert(player);
        parent
    }

    /// Accumulate the actions until some frame.
    pub fn accumulated_frame_to(&self, track: &Track, k: usize) -> Frame {
        let frames_until_k = || track
            .frames[..=k].iter()
            .flat_map(|frame| frame.0.iter());
        Frame([
            frames_until_k().filter(|act| matches!(act, Action::LoadElement(_))).last(),
            frames_until_k().filter(|act| matches!(act, Action::Translation(_))).last(),
            frames_until_k().filter(|act| matches!(act, Action::Scale(_))).last(),
            frames_until_k().filter(|act| matches!(act, Action::Rotation(_))).last(),
            frames_until_k().filter(|act| matches!(act, Action::Alpha(_))).last(),
            frames_until_k().filter(|act| matches!(act, Action::Show(_))).last(),
        ].into_iter().flatten().cloned().collect())
    }

    /// Animation clip for the [`Meta`](crate::sprite::Meta) at some index.
    pub fn clip_for(&self, k: usize) -> Arc<AnimationClip> {
        self.clips[k].get_or_init(|| {
            let frame_len = 1.0 / self.description.fps;

            let mut clip_builder = AnimationClip::builder();
            let meta = &self.description.meta[k];
            for (z_order, track) in self.description.tracks.iter().enumerate() {
                let path = EntityPath::from([Name::new(track.name.clone())]);

                let z_order = z_order as f32 * 0.1;

                let n = (meta.end_frame - meta.start_frame + 1) as usize;
                let mut translations = CurveBuilder::<Vec<Vec2>>::with_capacity(n);
                let mut scales = CurveBuilder::<Vec<Vec2>>::with_capacity(n);
                let mut rotations = CurveBuilder::<Vec<Vec2>>::with_capacity(n);
                let mut textures = CurveBuilder::<Vec<Handle<Image>>>::with_capacity(n);
                let mut visibilities = CurveBuilder::<BitVec>::with_capacity(n);
                let mut alphas = CurveBuilder::<Vec<f32>>::with_capacity(n);

                let frame0 = self.accumulated_frame_to(track, meta.start_frame as usize);
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
                            &Action::Alpha(alpha) => alphas.push_keyframe(k, alpha),
                            &Action::Show(visible) => visibilities.push_keyframe(k, visible),
                            &Action::Translation(t) => translations.push_keyframe(k, Vec2::from(t)),
                            &Action::Scale(s) => scales.push_keyframe(k, Vec2::from(s)),
                            &Action::Rotation(r) => rotations.push_keyframe(k, Vec2::from(r)),
                        }
                    }
                }

                let z_order_curve = ConstCurve::new_boxed(z_order, _ZOrder);
                for curve in std::iter::once(z_order_curve).chain([
                    translations.finish(frame_len, _Translation),
                    scales.finish(frame_len, _Scale),
                    rotations.finish(frame_len, _Rotation),
                    textures.finish(frame_len, _Identity::<Handle<Image>>::default()),
                    visibilities.finish(frame_len, _IsVisible),
                    alphas.finish(frame_len, optics!(_Color._Alpha)),
                ].into_iter().flatten()) {
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
