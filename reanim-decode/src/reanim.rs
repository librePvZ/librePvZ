/*
 * reanim-decode: decoder for PvZ reanim files.
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

//! Definition and decoding logic for `.reanim.compiled` files.

use std::io::BufRead;
use flate2::bufread::ZlibDecoder;
use serde::{Serialize, Deserialize};
#[cfg(feature = "packed")]
use libre_pvz_resources::sprite as packed;
use libre_pvz_resources::sprite::Element;
use crate::stream::{Decode, Stream, Result};

/// Animation in a `.reanim` file.
#[derive(Debug, Serialize, Deserialize)]
pub struct Animation {
    /// Frames per second, typically 12.
    pub fps: f32,
    /// All tracks in this animation.
    pub tracks: Box<[Track]>,
}

impl Animation {
    /// Decode a `.reanim` or `.reanim.compiled` file.
    /// Performs decompression before decoding if necessary.
    pub fn decompress_and_decode<R: Stream + BufRead + ?Sized>(s: &mut R) -> Result<Animation> {
        if let Ok([0xD4, 0xFE, 0xAD, 0xDE, ..]) = s.fill_buf() {
            s.consume(8);
            Animation::decode(&mut ZlibDecoder::new(s))
        } else {
            Animation::decode(s)
        }
    }
}

impl Decode for Animation {
    fn decode<S: Stream + ?Sized>(s: &mut S) -> Result<Animation> {
        log::debug!("decoding Animation (XML root node)");
        s.check_magic(0xB3_93_B4_C0)?;
        s.drop_padding("after-magic", 4)?;
        let track_count = s.read_data::<u32>()? as usize;
        let fps = s.read_data::<f32>()?;
        s.drop_padding("prop", 4)?;
        s.check_magic(0x0C)?;
        let mut tracks = Vec::with_capacity(track_count);
        let frame_counts = std::iter::repeat_with(|| {
            s.drop_padding("frame", 8)?;
            s.read_data::<u32>()
        }).take(track_count).collect::<Result<Vec<_>>>()?;
        for frame_count in frame_counts {
            tracks.push(Track::decode_with_frame_count(s, frame_count as usize)?);
        }
        Ok(Animation { fps, tracks: tracks.into_boxed_slice() })
    }
}

#[cfg(feature = "packed")]
impl From<Animation> for packed::Animation {
    fn from(anim: Animation) -> packed::Animation {
        packed::Animation {
            fps: anim.fps,
            tracks: anim.tracks.into_vec().into_iter().map(From::from).collect(),
        }
    }
}

/// A single track in an [`Animation`].
#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
    /// Name of this track for internal use.
    pub name: String,
    /// Frames, possibly grouped into several parts.
    pub frames: Box<[Frame]>,
}

impl Track {
    fn decode_with_frame_count<S: Stream + ?Sized>(s: &mut S, n: usize) -> Result<Self> {
        let name = s.read_string()?;
        log::debug!("decoding Track '{name}' of length {n} (XML tag <track>)");
        s.check_magic(0x2C)?;
        let transforms = s.read_n::<Transform>(n)?;
        let elements = s.read_n::<Elements>(n)?;
        let frames = transforms.into_iter()
            .zip(elements.into_iter())
            .map(|(transform, elements)| Frame { transform, elements })
            .collect();
        Ok(Track { name, frames })
    }
}

#[cfg(feature = "packed")]
impl From<Track> for packed::Track {
    fn from(track: Track) -> packed::Track {
        let mut frames = Vec::with_capacity(track.frames.len());
        let mut last_frame = [[0_f32; 3]; 2];
        for frame in track.frames.into_vec() {
            let mut packed = Vec::new();
            // transformations: matrix and/or alpha
            let Transform { x, y, kx, ky, sx, sy, f, a } = frame.transform;
            if [x, y, kx, ky, sx, sy].iter().any(Option::is_some) {
                let [[last_sx, last_kx, last_x], [last_ky, last_sy, last_y]] = last_frame;
                let x = x.unwrap_or(last_x);
                let y = y.unwrap_or(last_y);
                let sx = sx.unwrap_or(last_sx);
                let sy = sy.unwrap_or(last_sy);
                let kx = kx.unwrap_or(last_kx);
                let ky = ky.unwrap_or(last_ky);
                last_frame = [[sx, kx, x], [ky, sy, y]];
                packed.push(packed::Transform::Transform(last_frame));
            }
            if let Some(a) = a {
                packed.push(packed::Transform::Alpha(a));
            }
            if let Some(f) = f {
                log::warn!(target: "pack", "dropped node <f>{f}</f>");
            }
            // elements: text OR image
            let Elements { text, font, image } = frame.elements;
            let mut has_image = false;
            if let Some(image) = image {
                packed.push(packed::Transform::LoadElement(Element::Image { image }));
                has_image = true;
            }
            match (text, font) {
                (Some(text), Some(font)) => if has_image {
                    log::warn!(target: "pack", "dropped <text>{text}</text> in favour of <i>");
                } else {
                    packed.push(packed::Transform::LoadElement(Element::Text { text, font }));
                },
                (Some(text), None) => log::warn!(target: "pack", "dropped <text>{text}</text> without <font>"),
                (None, Some(font)) => log::warn!(target: "pack", "dropped <font>{font}</font> without <text>"),
                _ => {}
            }
            frames.push(packed::Frame(packed.into_boxed_slice()))
        }
        packed::Track { name: track.name, frames: frames.into_boxed_slice() }
    }
}

/// A transformation.
#[derive(Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Transform {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kx: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ky: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sx: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sy: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub f: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<f32>,
}

impl Decode for Transform {
    fn decode<S: Stream + ?Sized>(s: &mut S) -> Result<Transform> {
        log::debug!("decoding Transform (XML tag <t>)");
        let x = s.read_optional::<f32>()?;
        let y = s.read_optional::<f32>()?;
        let kx = s.read_optional::<f32>()?;
        let ky = s.read_optional::<f32>()?;
        let sx = s.read_optional::<f32>()?;
        let sy = s.read_optional::<f32>()?;
        let f = s.read_optional::<f32>()?;
        let a = s.read_optional::<f32>()?;
        s.drop_padding("transform", 12)?;
        Ok(Transform { x, y, kx, ky, sx, sy, f, a })
    }
}

/// An element in a [`Frame`].
#[derive(Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Elements {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Decode for Elements {
    fn decode<S: Stream + ?Sized>(s: &mut S) -> Result<Elements> {
        fn opt(s: String) -> Option<String> {
            if s.is_empty() { None } else { Some(s) }
        }
        let image_name = opt(s.read_string()?);
        let font_name = opt(s.read_string()?);
        let text = opt(s.read_string()?);
        Ok(Elements { image: image_name, font: font_name, text })
    }
}

/// A frame in a [`Track`], consist of (optional) image, text, and transformation.
#[derive(Debug, Serialize, Deserialize)]
pub struct Frame {
    /// Transformation: translation, skew, rotation, etc.
    #[serde(flatten)]
    pub transform: Transform,
    /// Elements: image and text (with font).
    #[serde(flatten)]
    pub elements: Elements,
}
