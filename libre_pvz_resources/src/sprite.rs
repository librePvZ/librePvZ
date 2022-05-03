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

use bincode::{Encode, Decode};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

/// Animations, originally in `.reanim` format.
#[derive(Debug, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    pub fn image_files(&self) -> impl Iterator<Item=&str> {
        self.tracks.iter()
            .flat_map(|track| track.frames.iter())
            .flat_map(|frame| frame.0.iter())
            .filter_map(|trans| match trans {
                Action::LoadElement(Element::Image { image }) => Some(image.as_str()),
                _ => None,
            })
    }

    /// Get a meta track by name.
    pub fn get_meta(&self, name: &str) -> Option<&Meta> {
        let k = self.meta.binary_search_by_key(&name, |meta| meta.name.as_str()).ok()?;
        Some(&self.meta[k])
    }
}

/// Meta data for animations.
#[derive(Debug, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Meta {
    /// Name of this meta data.
    pub name: String,
    /// (inclusive) Start of the frame range this meta data covers.
    pub start_frame: u16,
    /// (exclusive) End of the frame range this meta data covers.
    pub end_frame: u16,
}

/// A series of frames to play consecutively.
#[derive(Debug, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Track {
    /// Track name for internal recognition.
    pub name: String,
    /// Frame list, grouped into segments internally.
    pub frames: Box<[Frame]>,
}

/// Key frame: show and transform elements.
/// Transformations are applied sequentially in one frame.
#[derive(Debug, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Frame(pub Box<[Action]>);

/// Affine transformation in 3D.
#[derive(Debug, Copy, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AffineMatrix3d(pub [[f32; 3]; 2]);

/// Key frame action.
#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Action {
    /// Load an element to replace the current one on the stage.
    LoadElement(Element),
    /// Change alpha (transparency).
    Alpha(f32),
    /// Show or hide the element.
    Show(bool),
    /// Change transformation matrix in `[[sx, kx, tx], [ky, sy, ty]]` format.
    Transform(AffineMatrix3d),
}

/// Element on the stage. Only one element is allowed on a single frame.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Element {
    /// Text element.
    Text {
        /// Text content to display. Characters not in the font are simply ignored.
        text: String,
        /// Font name.
        font: String,
    },
    /// Image element.
    Image {
        /// Image name.
        image: String,
    },
}
