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
//!
//! # Notes for Source Code
//! Fields marked with `#[br(temp)]` are removed from `struct` definition. We use normal comments
//! instead of doc comments for them. This way, if a `#[br(temp)]` is missing, we get a warning
//! from `rustdoc`.

use std::io::{BufRead, Seek};
use std::path::PathBuf;
use binrw::{binread, BinRead, BinResult};
use flate2::bufread::ZlibDecoder;
use serde::{Serialize, Deserialize};
use libre_pvz_resources::animation as packed;
use libre_pvz_resources::animation::Element;
use libre_pvz_resources::cached::{Cached, SortedSlice};
use packed::Action;
use crate::decode::{TrivialSeek, ArgVec, LenString, optional_f32, optional_string};

/// Animation in a `.reanim` file.
#[binread]
#[br(little)]
#[br(magic = 0xB3_93_B4_C0_u32)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Animation {
    // number of tracks in this animation.
    #[br(temp, pad_before = 4, pad_after = 4)]
    track_count: u32,
    /// Frames per second, typically 12.
    pub fps: f32,
    // number of frames in each track.
    #[br(temp, magic = 0x0C_u32, count = track_count)]
    frame_counts: Vec<FrameCount>,
    /// All tracks in this animation.
    #[br(args_raw = std::mem::take(& mut frame_counts), map = ArgVec::into_boxed_slice)]
    pub tracks: Box<[Track]>,
}

impl Animation {
    /// Decode a `.reanim` or `.reanim.compiled` file.
    /// Performs decompression before decoding if necessary.
    pub fn decompress_and_decode<R: BufRead + Seek>(s: &mut R) -> BinResult<Animation> {
        if let Ok([0xD4, 0xFE, 0xAD, 0xDE, ..]) = s.fill_buf() {
            s.consume(8);
            Animation::read(&mut TrivialSeek::new(ZlibDecoder::new(s)))
        } else {
            Animation::read(s)
        }
    }
}

macro_rules! narrow {
    ($n:expr, $on_err:expr, $or_else:expr) => {
        match $n.try_into() {
            Ok(n) => n,
            Err(_) => {
                $on_err($n);
                return Err($or_else)
            }
        }
    }
}

fn track_to_meta(track: packed::Track) -> Result<packed::Meta, packed::Track> {
    let mut ranges = Vec::new();
    let mut ignored_count = 0_usize;
    // visible by default from the start
    let mut current_visible = true;
    let mut last_key_frame = 0;
    for (k, frame) in track.frames
        .iter().enumerate()
        .filter(|(_, frame)| !frame.0.is_empty()) {
        for trans in frame.0.iter() {
            let visible = match trans {
                &Action::Show(visible) => visible,
                Action::LoadElement(_) => return Err(track),
                Action::Alpha(_)
                | Action::Translation(_)
                | Action::Scale(_)
                | Action::Rotation(_) => {
                    ignored_count += 1;
                    continue;
                }
            };
            if current_visible && !visible && last_key_frame != k {
                ranges.push((last_key_frame, k));
            }
            if current_visible != visible {
                last_key_frame = k;
                current_visible = visible;
            } else {
                tracing::warn!(target: "pack", "redundant 'show' in track '{}' frame {k}", track.name);
            }
        }
    }
    // still visible up until finished
    if current_visible {
        ranges.push((last_key_frame, track.frames.len()));
    }
    // only one range is allowed
    if let [(start_frame, end_frame)] = ranges[..] {
        let on_err = |n: usize| tracing::error!(target: "pack", "frame index ({n}) overflow in a meta track");
        let start_frame = narrow!(start_frame, on_err, track);
        let end_frame = narrow!(end_frame - 1, on_err, track);
        if ignored_count > 0 {
            tracing::warn!(target: "pack", "ignored {ignored_count} transform/alpha in meta track {}", track.name);
        }
        Ok(packed::Meta { name: track.name, start_frame, end_frame })
    } else {
        tracing::warn!(target: "pack", "discontinuous meta track {}: found ranges {ranges:?}", track.name);
        Err(track)
    }
}

impl From<Animation> for packed::AnimDesc {
    fn from(anim: Animation) -> packed::AnimDesc {
        let mut metas = Vec::new();
        let mut tracks = Vec::new();
        for track in anim.tracks.into_vec().into_iter().map(packed::Track::from) {
            match track_to_meta(track) {
                Ok(meta) => metas.push(meta),
                Err(track) => tracks.push(track),
            }
        }
        packed::AnimDesc {
            fps: anim.fps,
            meta: SortedSlice::from(metas),
            tracks: tracks.into_boxed_slice(),
        }
    }
}

#[derive(Copy, Clone, BinRead)]
struct FrameCount(#[br(pad_before = 8)] u32);

impl From<FrameCount> for u32 {
    fn from(n: FrameCount) -> u32 { n.0 }
}

/// A single track in an [`Animation`].
#[binread]
#[derive(Debug, Serialize, Deserialize)]
#[br(import_raw(frame_count: u32))]
pub struct Track {
    /// Name of this track for internal use.
    #[br(map = LenString::into_boxed_str)]
    pub name: Box<str>,
    #[br(magic = 0x2C_u32)]
    // Transforms in each frame.
    #[br(temp, count = frame_count, postprocess_now)]
    transforms: Vec<Transform>,
    // Elements in each frame.
    #[br(temp, count = frame_count, postprocess_now)]
    elements: Vec<Elements>,
    /// Frames, possibly grouped into several parts.
    #[br(calc = zip_frames(transforms, elements))]
    pub frames: Box<[Frame]>,
}

fn zip_frames(transforms: Vec<Transform>, elements: Vec<Elements>) -> Box<[Frame]> {
    transforms.into_iter()
        .zip(elements.into_iter())
        .map(|(transform, elements)| Frame { transform, elements })
        .collect()
}

impl From<Track> for packed::Track {
    fn from(track: Track) -> packed::Track {
        let mut frames = Vec::with_capacity(track.frames.len());
        #[derive(Copy, Clone)]
        struct RawTrans {
            x: f32,
            y: f32,
            kx: f32,
            ky: f32,
            sx: f32,
            sy: f32,
        }
        let mut last_frame = RawTrans { x: 0.0, y: 0.0, kx: 0.0, ky: 0.0, sx: 1.0, sy: 1.0 };
        for frame in track.frames.into_vec() {
            let mut packed = Vec::new();
            // transformations: matrix and/or alpha
            let Transform { x, y, kx, ky, sx, sy, f, a } = frame.transform;
            if x.is_some() || y.is_some() {
                last_frame.x = x.unwrap_or(last_frame.x);
                last_frame.y = y.unwrap_or(last_frame.y);
                // our y-axis is in the opposite direction
                packed.push(Action::Translation([last_frame.x, -last_frame.y]));
            }
            if sx.is_some() || sy.is_some() {
                last_frame.sx = sx.unwrap_or(last_frame.sx);
                last_frame.sy = sy.unwrap_or(last_frame.sy);
                packed.push(Action::Scale([last_frame.sx, last_frame.sy]));
            }
            if kx.is_some() || ky.is_some() {
                last_frame.kx = kx.map(f32::to_radians).unwrap_or(last_frame.kx);
                last_frame.ky = ky.map(f32::to_radians).unwrap_or(last_frame.ky);
                // it is 'ky' that was negated in 'FlashReanimExport.jsfl'
                // but our y-axis is in the opposite direction
                // so anti-diagonal elements should be negated
                // the overall effect is to negate 'kx' instead of 'ky'
                packed.push(Action::Rotation([-last_frame.kx, last_frame.ky]));
            }
            if let Some(a) = a {
                packed.push(Action::Alpha(a));
            }
            if let Some(f) = f {
                packed.push(Action::Show(f >= 0.0));
                if ![0.0, -1.0].contains(&f) {
                    tracing::warn!(target: "pack", "non-standard <f> node with value {f}");
                }
            }
            // elements: text OR image
            let Elements { text, font, image } = frame.elements;
            let mut has_image = false;
            if let Some(mut image) = image {
                let mut image_name_valid = false;
                if let Some(s) = image.strip_prefix("IMAGE_REANIM_") {
                    image = s.to_string();
                    if let Some(tail) = image.get_mut(1..) {
                        tail.make_ascii_lowercase();
                        image.push_str(".png");
                        image_name_valid = true;
                    }
                }
                if !image_name_valid {
                    tracing::error!(target: "pack", "exotic file name: {image}");
                }
                let image = Cached::from(PathBuf::from(image));
                packed.push(Action::LoadElement(Element::Image { image }));
                has_image = true;
            }
            match (text, font) {
                (Some(text), Some(font)) => if has_image {
                    tracing::warn!(target: "pack", "dropped <text>{text}</text> in favour of <i>");
                } else {
                    let text = text.into_boxed_str();
                    let font = Cached::from(PathBuf::from(font));
                    packed.push(Action::LoadElement(Element::Text { text, font }));
                },
                (Some(text), None) => tracing::warn!(target: "pack", "dropped <text>{text}</text> without <font>"),
                (None, Some(font)) => tracing::warn!(target: "pack", "dropped <font>{font}</font> without <text>"),
                _ => {}
            }
            frames.push(packed::Frame(packed.into_boxed_slice()))
        }
        packed::Track { name: track.name, frames: frames.into_boxed_slice() }
    }
}

/// A transformation.
#[derive(Debug, BinRead, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Transform {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_f32)]
    pub x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_f32)]
    pub y: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_f32)]
    pub kx: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_f32)]
    pub ky: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_f32)]
    pub sx: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_f32)]
    pub sy: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_f32)]
    pub f: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_f32)]
    #[br(pad_after = 12)]
    pub a: Option<f32>,
}

/// An element in a [`Frame`].
#[derive(Debug, BinRead, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Elements {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_string)]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_string)]
    pub font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[br(map = optional_string)]
    pub text: Option<String>,
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
