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

//! Animation related support using Bevy and libre_pvz_animation.

use std::sync::Arc;
use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use bevy::prelude::*;
use bevy::asset::{AssetPath, LoadContext, LoadedAsset};
use bevy::utils::HashMap;
use bevy::reflect::TypeUuid;
use bevy::sprite::Anchor;
use bitvec::prelude::*;
use optics::concrete::_Identity;
use once_cell::sync::OnceCell;
use libre_pvz_animation::clip::{AnimationClip, EntityPath, TrackBuilder};
use libre_pvz_animation::curve::Segment;
use libre_pvz_animation::transform::{SpriteBundle2D, Transform2D, SpatialBundle2D};
use crate::animation::{AnimDesc, Action, Element, Track, Frame, Meta};
use super::loader::TwoStageAssetLoader;

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
}

/// Animation and all its dependency images.
#[derive(TypeUuid)]
#[uuid = "b3eaf6b5-4c37-47a5-b2b7-b03666d7939b"]
#[allow(missing_debug_implementations)]
pub struct Animation {
    /// the animation description.
    pub description: AnimDesc,
    /// all the dependency images.
    pub images: HashMap<PathBuf, Handle<Image>>,
    /// the [`AnimationClip`] generated from description.
    pub clip: OnceCell<Arc<AnimationClip>>,
}

impl From<&Meta> for Segment {
    fn from(meta: &Meta) -> Segment {
        Segment { start: meta.start_frame, end: meta.end_frame }
    }
}

const WIDTH: f32 = 80.0;
const HALF_WIDTH: f32 = WIDTH / 2.0;

impl Animation {
    /// Spawn an animation.
    pub fn spawn_on(&self, commands: &mut Commands) -> (Entity, Vec<Entity>) {
        let mut track_entities = Vec::new();
        let parent = commands.spawn_bundle(SpatialBundle2D {
            local: Transform2D::from_translation(Vec2::new(-HALF_WIDTH, HALF_WIDTH)),
            ..SpatialBundle2D::default()
        }).id();
        for (z, track) in self.description.tracks.iter().enumerate() {
            let this = Name::new(track.name.to_string());
            let mut bundle = SpriteBundle2D::default();
            bundle.sprite.anchor = Anchor::TopLeft;
            bundle.transform.z_order = z as f32 * 0.1;
            let this = commands.spawn_bundle(bundle).insert(this).id();
            commands.entity(parent).add_child(this);
            track_entities.push(this);
        }
        (parent, track_entities)
    }

    /// Accumulate the actions until some frame.
    pub fn accumulated_frame_to(&self, track: &Track, k: usize) -> Frame {
        let mut actions = HashMap::new();
        for action in track.frames[..=k].iter()
            .flat_map(|frame| frame.0.iter()) {
            actions.insert(std::mem::discriminant(action), action);
        }
        Frame(actions.into_iter().map(|(_, action)| action).cloned().collect())
    }

    fn push_frame<'a, I>(&self, builder: &mut TrackBuilder, k: usize, frame: I)
        where I: IntoIterator<Item=&'a Action> {
        for act in frame.into_iter() {
            type _Image = _Identity::<Handle<Image>>;
            use Action::*;
            match act {
                LoadElement(Element::Text { .. }) => todo!(),
                LoadElement(Element::Image { image }) => {
                    let image = image.cached.get().unwrap().clone();
                    builder.push_keyframe(_Image::default(), k, image)
                }
                &Alpha(alpha) => builder.push_keyframe(_Alpha, k, alpha),
                &Show(visible) => builder.push_keyframe(_IsVisible, k, visible),
                &Translation(t) => builder.push_keyframe(_Translation, k, Vec2::from(t)),
                &Scale(s) => builder.push_keyframe(_Scale, k, Vec2::from(s)),
                &Rotation(r) => builder.push_keyframe(_Rotation, k, Vec2::from(r)),
            }
        }
    }

    /// Animation clip for the [`Meta`](crate::animation::Meta) at some index.
    pub fn clip(&self) -> Arc<AnimationClip> {
        self.clip.get_or_init(|| {
            let mut clip_builder = AnimationClip::builder();
            for track in self.description.tracks.iter() {
                let path = EntityPath::from([Name::new(track.name.clone())]);
                let mut builder = TrackBuilder::default();
                builder.prepare_curve::<BitVec, _>(_IsVisible);
                for (k, frame) in track.frames.iter().enumerate() {
                    self.push_frame(&mut builder, k, frame.0.iter());
                }
                clip_builder.add_track(path, builder.finish());
            }

            Arc::new(clip_builder.build())
        }).clone()
    }
}

/// Asset loader for `.anim` files.
#[derive(Debug, Default)]
pub struct AnimationLoader;

impl TwoStageAssetLoader for AnimationLoader {
    type Repr = AnimDesc;
    fn extension(&self) -> &str { "anim" }
    fn post_process(&self, anim: AnimDesc, load_context: &mut LoadContext) -> anyhow::Result<()> {
        let dep_names = anim.image_files().collect::<Vec<_>>();
        let mut deps = Vec::with_capacity(dep_names.len());
        let mut images = HashMap::with_capacity(dep_names.len());
        for name in dep_names {
            name.init_handle(load_context);
            images.insert(name.raw_key.clone(), name.cached.get().unwrap().clone());
            deps.push(AssetPath::from(name.raw_key.as_path()).to_owned());
        }
        let anim = Animation { description: anim, images, clip: OnceCell::new() };
        load_context.set_default_asset(LoadedAsset::new(anim).with_dependencies(deps));
        Ok(())
    }
}
