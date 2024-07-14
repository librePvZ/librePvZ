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

//! Sprite and animation API.

use std::sync::Arc;
use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use bevy::prelude::*;
use bevy::asset::{Handle, AssetPath, LoadContext};
use bevy::text::Font;
use bevy::utils::HashMap;
use bevy::sprite::Anchor;
use bincode::{Encode, Decode};
use serde::{Serialize, Deserialize};
use optics::concrete::_Identity;
use once_cell::sync::OnceCell;
use libre_pvz_animation::clip::{AnimationClip, EntityPath, TrackBuilder};
use libre_pvz_animation::curve::Segment;
use libre_pvz_animation::transform::{SpriteBundle2D, Transform2D, SpatialBundle2D};
use crate::asset_ext;
use crate::cached::{Cached, EntryWithKey, SortedSlice};
use crate::loader::{AddTwoStageAsset, AssetExtensions, TwoStageAsset};

/// Resources plugin.
#[derive(Default, Debug, Copy, Clone)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_two_stage_asset::<Animation>();
    }
}

/// Animations, originally in `.reanim` format.
#[derive(Debug, Encode, Decode)]
#[derive(Serialize, Deserialize)]
pub struct AnimDesc {
    /// Frames per second.
    pub fps: f32,
    /// Meta data for this animation.
    pub meta: SortedSlice<Meta>,
    /// Animation tracks.
    pub tracks: Box<[Track]>,
}

impl AnimDesc {
    /// Get an iterator of all the image file names in this animation.
    pub fn image_files(&self) -> impl Iterator<Item = &Cached<PathBuf, Handle<Image>>> {
        self.tracks.iter()
            .flat_map(|track| track.frames.iter())
            .flat_map(|frame| frame.0.iter())
            .filter_map(|trans| match trans {
                Action::LoadElement(Element::Image { image }) => Some(image),
                _ => None,
            })
    }

    /// Get a meta track by name.
    pub fn get_meta(&self, name: &str) -> Option<(usize, &Meta)> {
        let k = self.meta.binary_search_by_key(&name, |meta| meta.name.as_str()).ok()?;
        Some((k, &self.meta[k]))
    }
}

/// Meta data for animations.
#[derive(Debug, Encode, Decode)]
#[derive(Serialize, Deserialize)]
pub struct Meta {
    /// Name of this meta data.
    pub name: String,
    /// (inclusive) Start of the frame range this meta data covers.
    pub start_frame: u16,
    /// (inclusive) End of the frame range this meta data covers.
    pub end_frame: u16,
}

impl EntryWithKey for Meta {
    type Key = str;
    fn key(&self) -> &str { &self.name }
}

/// A series of frames to play consecutively.
#[derive(Debug, Encode, Decode)]
#[derive(Serialize, Deserialize)]
pub struct Track {
    /// Track name for internal recognition.
    pub name: String,
    /// Frame list, grouped into segments internally.
    pub frames: Box<[Frame]>,
}

/// Key frame: show and transform elements.
/// Transformations are applied sequentially in one frame.
#[derive(Debug, Encode, Decode)]
#[derive(Serialize, Deserialize)]
pub struct Frame(pub Box<[Action]>);

/// 2D vectors.
pub type RawVec2 = [f32; 2];

/// Key frame action.
#[derive(Debug, Clone, Encode, Decode)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Load an element to replace the current one on the stage.
    LoadElement(Element),
    /// Change alpha (transparency).
    Alpha(f32),
    /// Show or hide the element.
    Show(bool),
    /// Change the translation.
    Translation(RawVec2),
    /// Change the scaling.
    Scale(RawVec2),
    /// Change the rotation.
    Rotation(RawVec2),
}

/// Optics for [`Action`].
pub mod action {
    use super::{Action, Element, RawVec2};
    optics::declare_prism_from_variant! {
        /// Prism for [`Action::LoadElement`].
        pub _LoadElement for LoadElement as Action => Element;
        /// Prism for [`Action::Alpha`].
        pub _Alpha for Alpha as Action => f32;
        /// Prism for [`Action::Show`].
        pub _Show for Show as Action => bool;
        /// Prism for [`Action::Translation`].
        pub _Translation for Translation as Action => RawVec2;
        /// Prism for [`Action::Scale`].
        pub _Scale for Scale as Action => RawVec2;
        /// Prism for [`Action::Rotation`].
        pub _Rotation for Rotation as Action => RawVec2;
    }
}

/// Element on the stage. Only one element is allowed on a single frame.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Encode, Decode)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Element {
    /// Text element.
    Text {
        /// Text content to display. Characters not in the font are simply ignored.
        text: String,
        /// Font name.
        font: Cached<PathBuf, Handle<Font>>,
    },
    /// Image element.
    Image {
        /// Image name.
        image: Cached<PathBuf, Handle<Image>>,
    },
}

optics::declare_lens_from_field! {
    _Color for color as Sprite => Color;
}

optics::declare_lens! {
    _Alpha as Color => f32,
    (color) => match color {
        Color::Srgba(Srgba { alpha, .. }) |
        Color::LinearRgba(LinearRgba { alpha, .. }) |
        Color::Hsla(Hsla { alpha, .. }) |
        Color::Hsva(Hsva { alpha, .. }) |
        Color::Hwba(Hwba { alpha, .. }) |
        Color::Laba(Laba { alpha, .. }) |
        Color::Lcha(Lcha { alpha, .. }) |
        Color::Oklaba(Oklaba { alpha, .. }) |
        Color::Oklcha(Oklcha { alpha, .. }) |
        Color::Xyza(Xyza { alpha, .. }) => alpha,
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
    _Translation for translation as Transform2D => Vec2;
    _Scale for scale as Transform2D => Vec2;
    _Rotation for rotation as Transform2D => Vec2;
}

/// Animation and all its dependency images.
#[derive(Asset, TypePath)]
#[allow(missing_debug_implementations)]
pub struct Animation {
    /// the animation description.
    pub description: AnimDesc,
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
    pub fn spawn_on(&self, commands: &mut Commands, translation: Vec2,
                    mut call_back: impl FnMut(usize, &str, Entity)) -> Entity {
        let parent = commands.spawn(SpatialBundle2D {
            local: Transform2D::from_translation(translation),
            ..SpatialBundle2D::default()
        }).id();
        for (z, track) in self.description.tracks.iter().enumerate() {
            let this = Name::new(track.name.to_string());
            let mut bundle = SpriteBundle2D::default();
            bundle.sprite.anchor = Anchor::TopLeft;
            bundle.transform.z_order = z as f32 * 0.1;
            let this = commands.spawn((bundle, this)).id();
            commands.entity(parent).add_child(this);
            call_back(z, &track.name, this);
        }
        parent
    }

    /// Spawn an animation, ignore internal entities.
    pub fn spawn_on_(&self, commands: &mut Commands) -> Entity {
        self.spawn_on(commands, Vec2::new(-HALF_WIDTH, HALF_WIDTH), |_, _, _| {})
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
    where
        I: IntoIterator<Item = &'a Action>,
    {
        for act in frame.into_iter() {
            type _Image = _Identity<Handle<Image>>;
            type _IsVisible = _Identity<Visibility>;
            let vis = |vis| if vis { Visibility::Inherited } else { Visibility::Hidden };
            use Action::*;
            match act {
                LoadElement(Element::Text { .. }) => todo!(),
                LoadElement(Element::Image { image }) => {
                    let image = image.cached.get().unwrap().clone();
                    builder.push_keyframe(_Image::default(), k, image)
                }
                &Alpha(alpha) => builder.push_keyframe(_Alpha, k, alpha),
                &Show(visible) => builder.push_keyframe(_IsVisible::default(), k, vis(visible)),
                &Translation(t) => builder.push_keyframe(_Translation, k, Vec2::from(t)),
                &Scale(s) => builder.push_keyframe(_Scale, k, Vec2::from(s)),
                &Rotation(r) => builder.push_keyframe(_Rotation, k, Vec2::from(r)),
            }
        }
    }

    /// Animation clip for the [`Meta`] at some index.
    pub fn clip(&self) -> Arc<AnimationClip> {
        self.clip.get_or_init(|| {
            let mut clip_builder = AnimationClip::builder();
            for track in self.description.tracks.iter() {
                let path = EntityPath::from([Name::new(track.name.clone())]);
                let mut builder = TrackBuilder::default();
                for (k, frame) in track.frames.iter().enumerate() {
                    self.push_frame(&mut builder, k, frame.0.iter());
                }
                clip_builder.add_track(path, builder.finish());
            }

            Arc::new(clip_builder.build())
        }).clone()
    }
}

impl TwoStageAsset for Animation {
    type Repr = AnimDesc;
    const EXTENSIONS: AssetExtensions = asset_ext!("anim");
    fn post_process(anim: AnimDesc, load_context: &mut LoadContext) -> anyhow::Result<(Animation, Vec<AssetPath<'static>>)> {
        let deps = anim.image_files().collect::<Vec<_>>();
        let mut dep_paths = Vec::with_capacity(deps.len());
        for name in deps {
            name.init_handle(load_context);
            dep_paths.push(name.asset_path().into_owned());
        }
        let anim = Animation { description: anim, clip: OnceCell::new() };
        Ok((anim, dep_paths))
    }
}
