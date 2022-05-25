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

//! Basic constructs for keyframe animations.

use std::any::TypeId;
use std::borrow::Borrow;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use bitvec::prelude::*;
use derivative::Derivative;
use bevy::prelude::*;
use bevy::asset::{Asset, HandleId};
use optics::concrete::Compose;
use optics::traits::{AffineFoldMut, OpticsFallible};
use crate::reflect::_Reflect;

/// Animatable types can be interpolated with `f32`s.
pub trait Animatable {
    /// Typically `a * (1 - time) + b * time`.
    fn interpolate(a: &Self, b: &Self, time: f32) -> Self;
}

impl Animatable for bool {
    fn interpolate(a: &bool, _b: &bool, _time: f32) -> bool { *a }
}

impl Animatable for f32 {
    fn interpolate(a: &f32, b: &f32, time: f32) -> f32 {
        a * (1_f32 - time) + b * time
    }
}

impl Animatable for Vec2 {
    fn interpolate(a: &Vec2, b: &Vec2, time: f32) -> Vec2 {
        Vec2::lerp(*a, *b, time)
    }
}

impl Animatable for Vec3 {
    fn interpolate(a: &Vec3, b: &Vec3, time: f32) -> Vec3 {
        Vec3::lerp(*a, *b, time)
    }
}

impl Animatable for Quat {
    fn interpolate(a: &Quat, b: &Quat, time: f32) -> Quat {
        Quat::slerp(*a, *b, time)
    }
}

impl Animatable for Transform {
    fn interpolate(a: &Transform, b: &Transform, time: f32) -> Transform {
        Transform {
            translation: Vec3::interpolate(&a.translation, &b.translation, time),
            rotation: Quat::interpolate(&a.rotation, &b.rotation, time),
            scale: Vec3::interpolate(&a.scale, &b.scale, time),
        }
    }
}

impl<T: Asset> Animatable for Handle<T> {
    fn interpolate(a: &Handle<T>, _b: &Handle<T>, _time: f32) -> Handle<T> { a.clone() }
}

impl Animatable for HandleId {
    fn interpolate(a: &HandleId, _b: &HandleId, _time: f32) -> HandleId { *a }
}

/// Animation curve.
pub trait Curve: Send + Sync + 'static {
    /// Get the type ID for the target component.
    fn component_type(&self) -> TypeId;
    /// Get the type name for the target component (used only for debug purpose).
    fn component_type_name(&self) -> &'static str;
    /// Duration of this curve, in seconds.
    fn duration(&self) -> f32;
    /// Apply the [`TypedCurve::sample`]d value to some [`Reflect`] as the result.
    fn apply_sampled(&self, time: f32, output: &mut dyn Reflect) -> Result<(), Box<str>>;
}

/// Typed curves.
pub trait TypedCurve {
    /// Value type for this curve.
    type Value: 'static;
    /// Sample the curve at specific time.
    ///
    /// Since multiple [`Curve`]s can coexist in a single [`AnimationClip`], and all of them need
    /// not have the same duration, we require `sample` be well-defined for any positive time.
    /// This typically will just return the value that would be sampled at time `duration`, but it
    /// doesn't have to be.
    ///
    /// [`AnimationClip`]: crate::clip::AnimationClip
    fn sample(&self, time: f32) -> Self::Value;
    /// Get a field accessor for the targeted field.
    fn access_field<'a>(&self, data: &'a mut dyn Reflect) -> Result<&'a mut Self::Value, Box<str>>;
}

/// Constant curve.
#[derive(Derivative)]
#[derivative(Debug(bound = "T: Debug, F: Debug"))]
pub struct ConstCurve<S, T, F> {
    value: T,
    field_accessor: F,
    #[derivative(Debug = "ignore")]
    _target_type: PhantomData<fn() -> S>,
}

impl<S, T, F> ConstCurve<S, T, F> {
    /// Create a constant curve.
    pub fn new(value: T, field_accessor: F) -> Self {
        ConstCurve {
            value,
            field_accessor,
            _target_type: PhantomData,
        }
    }

    /// Create a constant curve.
    pub fn new_boxed(value: T, field_accessor: F) -> Box<dyn Curve>
        where S: Reflect, T: Send + Sync + Clone + 'static,
              F: Send + Sync + 'static + for<'a> AffineFoldMut<'a, S, View=T, ViewLifeBound=T>,
              F::Error: Display {
        Box::new(Self::new(value, field_accessor))
    }
}

impl<S, T, F> Curve for ConstCurve<S, T, F>
    where S: Reflect, T: Send + Sync + Clone + 'static,
          F: Send + Sync + 'static + for<'a> AffineFoldMut<'a, S, View=T, ViewLifeBound=T>,
          F::Error: Display {
    fn component_type(&self) -> TypeId { TypeId::of::<S>() }
    fn component_type_name(&self) -> &'static str { std::any::type_name::<S>() }
    fn duration(&self) -> f32 { 0.0 }
    fn apply_sampled(&self, _time: f32, output: &mut dyn Reflect) -> Result<(), Box<str>> {
        let output = self.access_field(output)?;
        output.clone_from(&self.value);
        Ok(())
    }
}

impl<S, T, F> TypedCurve for ConstCurve<S, T, F>
    where S: Reflect, T: Send + Sync + Clone + 'static,
          F: Send + Sync + 'static + for<'a> AffineFoldMut<'a, S, View=T, ViewLifeBound=T>,
          F::Error: Display {
    type Value = T;
    fn sample(&self, _time: f32) -> T { self.value.clone() }
    fn access_field<'a>(&self, data: &'a mut dyn Reflect) -> Result<&'a mut Self::Value, Box<str>> {
        Compose(_Reflect::<S>::default(), &self.field_accessor).to_str_err().preview_mut(data)
    }
}

/// Provides linear random access to keyframe contents in a [`Track`].
#[allow(clippy::len_without_is_empty)] // never empty
pub trait TrackContent<'a>: Send + Sync + 'static {
    /// Keyframe content type.
    type Keyframe: ?Sized + 'static;
    /// Borrow of keyframe contents.
    type KeyframeRef: Borrow<Self::Keyframe>;
    /// Track length.
    fn track_len(&self) -> usize;
    /// Get keyframe at specific index.
    fn track_get(&'a self, k: usize) -> Self::KeyframeRef;
}

impl<'a, T: Send + Sync + 'static> TrackContent<'a> for Box<[T]> {
    type Keyframe = T;
    type KeyframeRef = &'a T;
    fn track_len(&self) -> usize { self.deref().len() }
    fn track_get(&'a self, k: usize) -> &'a T { &self[k] }
}

impl<'a, T, O> TrackContent<'a> for BitBox<T, O>
    where T: BitStore + Send + Sync + 'static,
          O: BitOrder + Send + Sync + 'static {
    type Keyframe = bool;
    type KeyframeRef = bool;
    fn track_len(&self) -> usize { self.len() }
    fn track_get(&'a self, k: usize) -> bool { self[k] }
}

/// Provides management on frame indices in a [`Track`].
pub trait FrameIndex: Send + Sync + 'static {
    /// Duration of this track, in seconds.
    fn duration(&self) -> f32;
    /// Number of indices.
    fn count(&self) -> usize;
    /// Frame index in the keyframe list from a timestamp.
    /// Return value interpreted the same way as [`slice::binary_search`].
    fn index_from_time(&self, time: f32) -> Result<usize, usize>;
    /// Timestamp from a frame index in the keyframe list.
    fn time_at_index(&self, k: usize) -> f32;
}

/// Frame indices at a fixed frame rate.
#[derive(Debug, Clone)]
pub struct FrameIndexFixedFPS<I> {
    /// Frame length in seconds.
    pub frame_len: f32,
    /// Keyframe indices.
    pub indices: Box<[I]>,
}

impl<I: Into<u64> + Into<f32> + Copy + Send + Sync + 'static> FrameIndex for FrameIndexFixedFPS<I> {
    fn duration(&self) -> f32 {
        <I as Into<f32>>::into(*self.indices.last().unwrap()) * self.frame_len
    }
    fn count(&self) -> usize { self.indices.len() }
    fn index_from_time(&self, time: f32) -> Result<usize, usize> {
        let k = (time / self.frame_len) as u64;
        self.indices.binary_search_by_key(&k, |k| <I as Into<u64>>::into(*k))
    }
    fn time_at_index(&self, k: usize) -> f32 {
        <I as Into<f32>>::into(self.indices[k]) * self.frame_len
    }
}

/// Keyframe animation track with a fixed frame rate.
#[derive(Derivative)]
#[derivative(Debug(bound = "F: Debug"))]
pub struct Track<S, F, I, C> {
    /// Target component type.
    pub _component_type: PhantomData<fn() -> S>,
    /// Field accessor from [`Reflect`].
    pub field_accessor: F,
    /// Keyframe indices.
    #[derivative(Debug = "ignore")]
    pub indices: I,
    /// Keyframe contents.
    #[derivative(Debug = "ignore")]
    pub frames: C,
}

impl<S, F: Display, I: FrameIndex, C: for<'a> TrackContent<'a>> Display for Track<S, F, I, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Track[{}: {}; len = {}, dur = {:.1}s]",
               self.field_accessor,
               std::any::type_name::<C::Keyframe>(),
               self.frames.track_len(), self.indices.duration())
    }
}

impl<S, F, I, T, C> Curve for Track<S, F, I, C>
    where I: FrameIndex + Send + Sync + 'static,
          C: for<'a> TrackContent<'a, Keyframe=T> + Send + Sync + 'static,
          S: Reflect, T: Send + Sync + Animatable + 'static,
          F: Send + Sync + 'static + for<'a> AffineFoldMut<'a, S, View=T, ViewLifeBound=T>,
          F::Error: Display {
    fn component_type(&self) -> TypeId { TypeId::of::<S>() }
    fn component_type_name(&self) -> &'static str { std::any::type_name::<S>() }
    fn duration(&self) -> f32 { self.indices.duration() }
    fn apply_sampled(&self, time: f32, output: &mut dyn Reflect) -> Result<(), Box<str>> {
        let output = self.access_field(output)?;
        let sampled = self.sample(time);
        *output = sampled;
        Ok(())
    }
}

impl<S, F, I, T, C> TypedCurve for Track<S, F, I, C>
    where I: FrameIndex + Send + Sync + 'static,
          C: for<'a> TrackContent<'a, Keyframe=T> + Send + Sync + 'static,
          S: Reflect, T: Send + Sync + Animatable + 'static,
          F: Send + Sync + 'static + for<'a> AffineFoldMut<'a, S, View=T, ViewLifeBound=T>,
          F::Error: Display {
    type Value = T;

    fn sample(&self, time: f32) -> Self::Value {
        let delta = |k: usize| self.indices.time_at_index(k + 1) - self.indices.time_at_index(k);
        let elapsed = |k: usize| time - self.indices.time_at_index(k);

        assert_eq!(self.indices.count(), self.frames.track_len());
        let n = self.frames.track_len();
        let (this, next, ratio) = match self.indices.index_from_time(time) {
            Ok(k) if k + 1 >= n => (k, k, 0.0),
            Ok(k) => (k, k + 1, elapsed(k) / delta(k)),
            Err(k) if k == n => (n - 1, n - 1, 0.0),
            Err(0) => (0, 0, 0.0),
            Err(k) => (k - 1, k, elapsed(k - 1) / delta(k - 1)),
        };

        C::Keyframe::interpolate(
            self.frames.track_get(this).borrow(),
            self.frames.track_get(next).borrow(),
            ratio,
        )
    }

    fn access_field<'a>(&self, data: &'a mut dyn Reflect) -> Result<&'a mut T, Box<str>> {
        Compose(_Reflect::<S>::default(), &self.field_accessor).to_str_err().preview_mut(data)
    }
}

/// Convenient builder for [`Curve`]s.
#[allow(missing_debug_implementations)]
pub struct CurveBuilder<C> {
    indices: Indices,
    contents: C,
}

impl<C: TrackContentBuilder> CurveBuilder<C> {
    /// Create a curve builder with a capacity.
    /// One shall not insert more keyframes than this capacity.
    pub fn with_capacity(capacity: usize) -> CurveBuilder<C> {
        CurveBuilder {
            indices: Indices::with_capacity(capacity),
            contents: C::with_capacity(capacity),
        }
    }

    /// Push one keyframe into this curve.
    pub fn push_keyframe(&mut self, k: usize, x: <C::Target as TrackContent<'static>>::Keyframe)
        where <C::Target as TrackContent<'static>>::Keyframe: Sized {
        self.indices.push(k);
        self.contents.push_keyframe(x);
    }

    /// Finish building this curve.
    pub fn finish<S, T, F>(self, frame_len: f32, field: F) -> Option<Box<dyn Curve>>
        where S: Reflect, T: Animatable + Send + Sync + 'static,
              C::Target: for<'a> TrackContent<'a, Keyframe=T>,
              F: Send + Sync + 'static + for<'a> AffineFoldMut<'a, S, View=T, ViewLifeBound=T>,
              F::Success: Display, F::Error: Display {
        self.indices.finish_with(frame_len, field, self.contents.finish())
    }
}

enum Indices {
    U8(Vec<u8>),
    U16(Vec<u16>),
}

impl Indices {
    fn with_capacity(capacity: usize) -> Indices {
        if capacity <= u8::MAX as usize {
            Indices::U8(Vec::with_capacity(capacity))
        } else if capacity <= u16::MAX as usize {
            Indices::U16(Vec::with_capacity(capacity))
        } else {
            panic!("too many keyframes: {capacity}")
        }
    }

    fn push(&mut self, t: usize) {
        match self {
            Indices::U8(ts) => ts.push(t as u8),
            Indices::U16(ts) => ts.push(t as u16),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Indices::U8(ts) => ts.is_empty(),
            Indices::U16(ts) => ts.is_empty(),
        }
    }

    fn finish_with<S, F, T, C>(self, frame_len: f32, field: F, frames: C) -> Option<Box<dyn Curve>>
        where S: Reflect, T: Animatable + Send + Sync + 'static,
              C: for<'a> TrackContent<'a, Keyframe=T>,
              F: Send + Sync + 'static + for<'a> AffineFoldMut<'a, S, View=T, ViewLifeBound=T>,
              F::Success: Display, F::Error: Display {
        if self.is_empty() { return None; }
        Some(match self {
            Indices::U8(indices) => Box::new(Track {
                _component_type: PhantomData,
                field_accessor: field,
                indices: FrameIndexFixedFPS { frame_len, indices: indices.into_boxed_slice() },
                frames,
            }),
            Indices::U16(indices) => Box::new(Track {
                _component_type: PhantomData,
                field_accessor: field,
                indices: FrameIndexFixedFPS { frame_len, indices: indices.into_boxed_slice() },
                frames,
            }),
        })
    }
}

/// Builder for track contents (keyframes).
/// Only intended for internal use and as an extension point.
pub trait TrackContentBuilder: Sized + 'static {
    /// Target type to build, a [`TrackContent`].
    type Target: TrackContent<'static> + Send + Sync;
    /// Create a new builder with some capacity (not to be exceeded).
    fn with_capacity(capacity: usize) -> Self;
    /// Push one keyframe to this builder.
    fn push_keyframe(&mut self, x: <Self::Target as TrackContent<'static>>::Keyframe);
    /// Finish building this track content.
    fn finish(self) -> Self::Target;
}

impl<T: Send + Sync + 'static> TrackContentBuilder for Vec<T> {
    type Target = Box<[T]>;
    fn with_capacity(capacity: usize) -> Self { Vec::with_capacity(capacity) }
    fn push_keyframe(&mut self, x: T) { self.push(x) }
    fn finish(self) -> Box<[T]> { self.into_boxed_slice() }
}

impl<T, O> TrackContentBuilder for BitVec<T, O>
    where T: BitStore + Send + Sync + 'static,
          O: BitOrder + Send + Sync + 'static {
    type Target = BitBox<T, O>;
    fn with_capacity(capacity: usize) -> Self { BitVec::with_capacity(capacity) }
    fn push_keyframe(&mut self, x: bool) { self.push(x) }
    fn finish(self) -> Self::Target { self.into_boxed_bitslice() }
}
