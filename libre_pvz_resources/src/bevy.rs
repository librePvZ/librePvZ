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
use optics::concrete::_Identity;
use once_cell::sync::OnceCell;
use libre_pvz_animation::player::AnimationPlayer;
use libre_pvz_animation::clip::{AnimationClip, EntityPath, TrackBuilder};
use libre_pvz_animation::curve::Segment;
use libre_pvz_animation::transform::{SpriteBundle2D, Transform2D, TransformBundle2D};
use crate::animation::{AnimDesc, Action, Element, Track, Frame, Meta};

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
    pub fn spawn_on(&self, meta: usize, commands: &mut Commands) -> (Entity, Vec<Entity>) {
        let clip = self.clip();
        let segment = Segment::from(&self.description.meta[meta]);
        let mut track_entities = Vec::new();
        let parent = commands.spawn_bundle(TransformBundle2D {
            local: Transform2D::from_translation(Vec2::new(-HALF_WIDTH, HALF_WIDTH)),
            ..TransformBundle2D::default()
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
        let player = AnimationPlayer::new(clip, segment, self.description.fps, true);
        commands.entity(parent).insert(player);
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
                LoadElement(Element::Image { image }) =>
                    builder.push_keyframe(_Image::default(), k, self.images[image].clone()),
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
            let anim = Animation { description: anim, images, clip: OnceCell::new() };
            load_context.set_default_asset(LoadedAsset::new(anim).with_dependencies(deps));
            Ok(())
        })
    }
    fn extensions(&self) -> &[&str] { &["anim"] }
}
