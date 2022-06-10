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

//! Builder API for [`Curve`](super::Curve)s.

use std::any::Any;
use std::fmt::Display;
use std::marker::PhantomData;
use bitvec::prelude::*;
use optics::traits::{AffineFoldRef, AffineFoldMut};
use crate::curve::concrete::KeyframeCurve;
use super::AnyCurve;
use super::animatable::Animatable;
use super::concrete::{CurveContent, CurveContentStatic};

/// An alternative dynamic interface for [`CurveBuilder`].
pub trait AnyCurveBuilder {
    /// Push one keyframe into this curve.
    ///
    /// **Note:** The value is expected to be of type `Option<T>`, where `T` is the actual value
    /// type for this curve. So effectively the parameter `x` has type `&mut Option<T>`, and
    /// a successful invoking of this method will [`take`] from this argument.
    ///
    /// [`take`]: Option::take
    fn push_keyframe(&mut self, k: u16, x: &mut dyn Any);
    /// Finish building this curve. Prefer [`CurveBuilder::finish`] whenever possible.
    fn finish_boxed(self: Box<Self>) -> Option<Box<dyn AnyCurve>>;
}

/// Convenient builder for [`Curve`](crate::curve::Curve)s.
#[derive(Default)]
#[allow(missing_debug_implementations)]
pub struct CurveBuilder<C> {
    indices: Vec<u16>,
    contents: C,
}

impl<C: CurveContentBuilder> CurveBuilder<C> {
    /// Create a curve builder.
    pub fn new() -> CurveBuilder<C> { CurveBuilder::default() }

    /// Push one keyframe into this curve.
    pub fn push_keyframe(&mut self, k: u16, x: <C::Target as CurveContentStatic>::Keyframe) {
        self.indices.push(k);
        self.contents.push_keyframe(x);
    }

    /// Convert to a dynamic [`AnyCurveBuilder`].
    pub fn into_dynamic<F, S>(self, field_accessor: F) -> Box<dyn AnyCurveBuilder>
        where S: 'static, C::Target: CurveContent<Keyframe=F::View>,
              F: Send + Sync + 'static
              + for<'a> AffineFoldRef<'a, S>
              + for<'a> AffineFoldMut<'a, S>,
              F::View: PartialEq + Animatable + Sized + Send + Sync + 'static,
              F::Error: Display {
        Box::new(DynCurveBuilder {
            builder: self,
            field_accessor,
            _marker: PhantomData,
        })
    }

    /// Finish building this curve.
    pub fn finish<F, S>(self, field_accessor: F) -> Option<Box<dyn AnyCurve>>
        where S: 'static, C::Target: CurveContent<Keyframe=F::View>,
              F: Send + Sync + 'static
              + for<'a> AffineFoldRef<'a, S>
              + for<'a> AffineFoldMut<'a, S>,
              F::View: PartialEq + Animatable + Sized + Send + Sync + 'static,
              F::Error: Display {
        if self.indices.is_empty() { return None; }
        Some(Box::new(KeyframeCurve::new(
            field_accessor.to_str_err(),
            self.indices.into_boxed_slice(),
            self.contents.finish(),
        )))
    }
}

struct DynCurveBuilder<C, F, S> {
    builder: CurveBuilder<C>,
    field_accessor: F,
    _marker: PhantomData<fn() -> S>,
}

impl<C, F, S> AnyCurveBuilder for DynCurveBuilder<C, F, S>
    where S: 'static, C: CurveContentBuilder,
          C::Target: CurveContent<Keyframe=F::View>,
          F: Send + Sync + 'static
          + for<'a> AffineFoldRef<'a, S>
          + for<'a> AffineFoldMut<'a, S>,
          F::View: PartialEq + Animatable + Sized + Send + Sync + 'static,
          F::Error: Display {
    fn push_keyframe(&mut self, k: u16, x: &mut dyn Any) {
        let x = x.downcast_mut::<Option<F::View>>()
            .expect("check the type before use").take()
            .expect("should call with '&mut Some(x)'");
        self.builder.push_keyframe(k, x);
    }
    fn finish_boxed(self: Box<Self>) -> Option<Box<dyn AnyCurve>> {
        self.builder.finish(self.field_accessor)
    }
}

/// Builder for track contents (keyframes).
/// Only intended for internal use and as an extension point.
pub trait CurveContentBuilder: Default + Sized + 'static {
    /// Target type to build, a [`CurveContent`].
    type Target: CurveContent + Send + Sync;
    /// Push one keyframe to this builder.
    fn push_keyframe(&mut self, x: <Self::Target as CurveContentStatic>::Keyframe);
    /// Finish building this track content.
    fn finish(self) -> Self::Target;
}

impl<T: Send + Sync + 'static> CurveContentBuilder for Vec<T> {
    type Target = Box<[T]>;
    fn push_keyframe(&mut self, x: T) { self.push(x) }
    fn finish(self) -> Box<[T]> { self.into_boxed_slice() }
}

impl<T, O> CurveContentBuilder for BitVec<T, O>
    where T: BitStore + Send + Sync + 'static,
          O: BitOrder + Send + Sync + 'static {
    type Target = BitBox<T, O>;
    fn push_keyframe(&mut self, x: bool) { self.push(x) }
    fn finish(self) -> Self::Target { self.into_boxed_bitslice() }
}
