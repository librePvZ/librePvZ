/*
 * librePvZ-animation: animation playing for librePvZ.
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

//! Concrete curves.

use std::borrow::Borrow;
use std::fmt::Debug;
use std::marker::PhantomData;
use bitvec::prelude::*;
use derivative::Derivative;
use optics::traits::{AffineFoldRef, AffineFoldMut};
use crate::curve::AnyComponent;
use super::{Curve, TypedCurve, Segment};
use super::animatable::Animatable;

/// The lifetime-irrelevant part of the [`CurveContent`] interface.
pub trait CurveContentStatic: Send + Sync + 'static {
    /// Keyframe content type.
    type Keyframe: 'static;
    /// Track length.
    fn curve_content_len(&self) -> usize;
}

/// Provided borrowed access to the keyframe contents.
pub trait CurveContentBorrow<'a>: CurveContentStatic {
    /// Borrow of keyframe contents.
    type KeyframeRef: Borrow<Self::Keyframe>;
    /// Get keyframe at specific index.
    fn curve_content_get(&'a self, k: usize) -> Self::KeyframeRef;
}

/// Provides linear random access to keyframe contents in a [`Curve`](super::Curve).
/// See also [`CurveContentStatic`] and [`CurveContentBorrow`].
pub trait CurveContent: for<'a> CurveContentBorrow<'a> {}

impl<T: for<'a> CurveContentBorrow<'a>> CurveContent for T {}

impl<T: Send + Sync + 'static> CurveContentStatic for Box<[T]> {
    type Keyframe = T;
    fn curve_content_len(&self) -> usize { self.as_ref().len() }
}

impl<'a, T: Send + Sync + 'static> CurveContentBorrow<'a> for Box<[T]> {
    type KeyframeRef = &'a T;
    fn curve_content_get(&'a self, k: usize) -> &'a T { &self[k] }
}

impl<T, O> CurveContentStatic for BitBox<T, O>
    where T: BitStore + Send + Sync + 'static,
          O: BitOrder + Send + Sync + 'static {
    type Keyframe = bool;
    fn curve_content_len(&self) -> usize { self.len() }
}

impl<'a, T, O> CurveContentBorrow<'a> for BitBox<T, O>
    where T: BitStore + Send + Sync + 'static,
          O: BitOrder + Send + Sync + 'static {
    type KeyframeRef = bool;
    fn curve_content_get(&'a self, k: usize) -> bool { self[k] }
}

/// Keyframe animation curve.
#[derive(Derivative)]
#[derivative(Debug(bound = "F: Debug"))]
pub struct KeyframeCurve<S, F, C> {
    /// Target component type.
    #[derivative(Debug = "ignore")]
    _component_type: PhantomData<fn() -> S>,
    /// Field accessor from `S`.
    field_accessor: F,
    /// Keyframe indices: the actual frame indices, including non-keyframes.
    keyframe_indices: Box<[u16]>,
    /// Keyframe contents.
    #[derivative(Debug = "ignore")]
    keyframes: C,
}

impl<S, F, C: CurveContent> KeyframeCurve<S, F, C> {
    /// Create a keyframe curve.
    pub fn new(field: F, indices: Box<[u16]>, keyframes: C) -> Self {
        assert!(!indices.is_empty(), "never create empty curves");
        assert_eq!(keyframes.curve_content_len(), indices.len(), "unaligned curve");
        KeyframeCurve {
            _component_type: PhantomData,
            field_accessor: field,
            keyframe_indices: indices,
            keyframes,
        }
    }

    fn next_keyframe(&self, frame: u16) -> usize {
        self.keyframe_indices.partition_point(|&ix| ix <= frame)
    }

    fn last_keyframe(&self, frame: u16) -> Option<usize> {
        let next = self.next_keyframe(frame);
        next.checked_sub(1)
    }
}

impl<S, F, C> Curve for KeyframeCurve<S, F, C>
    where S: 'static, C: CurveContent<Keyframe=F::View>,
          F::View: PartialEq + Animatable + Sized + Send + Sync + 'static,
          F: Send + Sync + 'static
          + for<'a> AffineFoldRef<'a, S, Error=String>
          + for<'a> AffineFoldMut<'a, S, Error=String> {
    type Component = S;
    fn frame_count(&self) -> usize { *self.keyframe_indices.last().unwrap() as usize }
    fn apply_sampled(&self, segment: Segment, frame: f32, output: impl AnyComponent<S>) -> Result<(), String> {
        if let Some(val) = self.sample(segment, frame) {
            self.update_field(output, val)?;
        }
        Ok(())
    }
}

impl<S, F, C> TypedCurve for KeyframeCurve<S, F, C>
    where S: 'static, C: CurveContent<Keyframe=F::View>,
          F::View: PartialEq + Animatable + Sized + Send + Sync + 'static,
          F: Send + Sync + 'static
          + for<'a> AffineFoldRef<'a, S, Error=String>
          + for<'a> AffineFoldMut<'a, S, Error=String> {
    type Value = F::View;
    type FieldAccessor = F;
    fn sample(&self, segment: Segment, frame: f32) -> Option<F::View> {
        let frame = frame + segment.start as f32;
        let (this, next, ratio) = if frame >= segment.end as f32 { // wrap back (looping)
            // l = the first keyframe
            let l = self.last_keyframe(segment.start)?;
            // r = the last keyframe
            // now l..=r range contains all frames in the required segment
            let r = self.last_keyframe(segment.end)?;
            // absent keyframe means "the same as the last one"
            // so interpolation starts from segment.end
            // over the one-frame interval from segment.end to segment.start
            (r, l, frame - segment.end as f32)
        } else { // normal in-range interpolation
            let n = self.keyframe_indices.len();
            // k = last keyframe before 'frame'
            let k = self.last_keyframe(frame as u16)?;
            if k + 1 >= n {
                (k, k, 0.0)
            } else {
                let elapsed = frame - self.keyframe_indices[k] as f32;
                let delta = self.keyframe_indices[k + 1] - self.keyframe_indices[k];
                (k, k + 1, elapsed / delta as f32)
            }
        };

        Some(C::Keyframe::interpolate(
            self.keyframes.curve_content_get(this).borrow(),
            self.keyframes.curve_content_get(next).borrow(),
            ratio,
        ))
    }
    fn field_accessor(&self) -> &Self::FieldAccessor {
        &self.field_accessor
    }
}
