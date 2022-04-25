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

/// Animations, originally in `.reanim` format.
#[derive(Debug, Encode, Decode)]
pub struct Animation {
    /// Frames per second.
    pub fps: f32,
    /// Animation tracks.
    pub tracks: Box<[Track]>,
}

/// A series of frames to play consecutively.
#[derive(Debug, Encode, Decode)]
pub struct Track {
    /// Track name for internal recognition.
    pub name: String,
    /// Frame list, grouped into segments internally.
    pub frames: Box<[Frame]>,
}

/// Key frame: show and transform elements.
/// Transformations are applied sequentially in one frame.
#[derive(Debug, Encode, Decode)]
pub struct Frame(pub Box<[Transform]>);

/// Key frame action.
#[derive(Debug, Encode, Decode)]
pub enum Transform {
    /// Load an element to replace the current one on the stage.
    LoadElement(Element),
    /// Change alpha (transparency).
    Alpha(f32),
    /// Change transformation matrix in `[[sx, kx, tx], [ky, sy, ty]]` format.
    Transform([[f32; 3]; 2]),
}

/// Element on the stage. Only one element is allowed on a single frame.
#[derive(Debug, Encode, Decode)]
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
