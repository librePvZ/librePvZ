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

use std::path::PathBuf;
use bevy::prelude::*;
use bevy::asset::Handle;
use bevy::text::Font;
use bincode::{Encode, Decode};
use serde::{Serialize, Deserialize};
use crate::cached::Cached;

/// Animations, originally in `.reanim` format.
#[derive(Debug, Encode, Decode)]
#[derive(Serialize, Deserialize)]
pub struct AnimDesc {
    /// Frames per second.
    pub fps: f32,
    /// Meta data for this animation.
    pub meta: Box<[Meta]>,
    /// Animation tracks.
    pub tracks: Box<[Track]>,
}

impl AnimDesc {
    /// Get an iterator of all the image file names in this animation.
    pub fn image_files(&self) -> impl Iterator<Item=&Cached<PathBuf, Handle<Image>>> {
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
pub type Vec2 = [f32; 2];

/// 2x2 matrices (column major).
pub type Mat2 = [Vec2; 2];

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
    Translation(Vec2),
    /// Change the scaling.
    Scale(Vec2),
    /// Change the rotation.
    Rotation(Vec2),
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
