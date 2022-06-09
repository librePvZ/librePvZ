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

//! Variable curves in animation clips.

pub mod animatable;
pub mod concrete;
pub mod builder;

use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use derivative::Derivative;
use optics::traits::*;

/// A segment in a curve.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Segment {
    /// Index of the first frame, inclusive.
    pub start: u16,
    /// Index of the last frame, inclusive.
    ///
    /// **Note:** by "inclusive", we mean `end` as frame index points to the last frame in this
    /// segment (i.e., it is _not_ the off-by-one index). But when playing the animation, since
    /// this is actually the last keyframe, animation ends immediately after reaching this frame
    /// (for non-repeating animations), or wrap back to the first frame (for repeating animations).
    pub end: u16,
}

#[allow(clippy::len_without_is_empty)]
impl Segment {
    /// Number of frames in this segment.
    pub const fn len(self) -> u16 { self.end - self.start }
    /// Number of frames in this segment, if looping.
    pub const fn len_looping(self) -> u16 { self.len() + 1 }
}

/// Animation curve.
pub trait Curve: Send + Sync + 'static {
    /// Target component.
    type Component: 'static;
    /// Duration of this curve, in number of frames.
    ///
    /// **Note:** This does not necessarily mean there are exactly this many frames stored in this
    /// animation. It merely serves to indicate the maximum frame index for sampling. See also
    /// [`Curve::apply_sampled`] and [`TypedCurve::sample`].
    fn frame_count(&self) -> usize;
    /// Apply the sampled value to the target component as the result.
    fn apply_sampled(
        &self, segment: Segment, frame: f32,
        output: impl AnyComponent<Self::Component>,
    ) -> Result<(), String>;
}

/// Information about a curve binding.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct CurveBindingInfo {
    pub(crate) player_entity: Entity,
    // use u16, because it is unthinkable to have more than 65536 curves.
    pub(crate) curve_index_start: u16,
    pub(crate) curve_index_end: u16,
}

/// Bind a contiguous range of curves (on the same component) to some entity.
#[derive(Copy, Clone, Component, Derivative)]
#[derivative(Debug(bound = ""))]
pub struct CurveBinding<C> {
    /// Information about this binding.
    pub info: CurveBindingInfo,
    #[derivative(Debug = "ignore")]
    _marker: PhantomData<fn() -> C>,
}

impl<C> CurveBinding<C> {
    /// Create a new curve binding with specified information.
    pub fn new(info: CurveBindingInfo) -> Self { Self { info, _marker: PhantomData } }
}

/// Descriptor for the curves.
/// Different curves for the same component share the same descriptor.
#[derive(Copy, Clone, Derivative)]
#[derivative(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct CurveDescriptor {
    component_type_id: TypeId,
    #[derivative(Debug = "ignore", Ord = "ignore", PartialOrd = "ignore", PartialEq = "ignore")]
    attach_binding: fn(EntityCommands, CurveBindingInfo),
}

impl CurveDescriptor {
    /// Create a new curve descriptor for specified component type.
    pub fn new<C: 'static>() -> CurveDescriptor {
        CurveDescriptor {
            component_type_id: TypeId::of::<C>(),
            attach_binding: attach_binding::<C>,
        }
    }

    /// Get the target component type id for verification.
    pub fn component_type_id(&self) -> TypeId { self.component_type_id }

    /// Attach a curve binding to an entity.
    pub fn attach_binding(&self, entity: EntityCommands, info: CurveBindingInfo) {
        (self.attach_binding)(entity, info)
    }
}

/// Bevy tracks changes in mutable references like [`Mut`].
/// We use this interface to avoid unnecessarily marking the target component as changed.
pub trait AnyComponent<Target: ?Sized = dyn Any> {
    /// Read the current value of the component.
    fn component(&self) -> &Target;
    /// Borrow the component as mutable and mark it dirty.
    fn component_mut(&mut self) -> &mut Target;
}

impl<'a, C> AnyComponent<C> for Mut<'a, C> {
    fn component(&self) -> &C { self.deref() }
    fn component_mut(&mut self) -> &mut C { self.deref_mut() }
}

impl<'a, C: 'static> AnyComponent<dyn Any> for Mut<'a, C> {
    fn component(&self) -> &dyn Any { self.deref() }
    fn component_mut(&mut self) -> &mut dyn Any { self.deref_mut() }
}

/// Assert the target component type for some [`AnyComponent`].
#[allow(missing_debug_implementations)]
pub struct UnwrapAnyComponent<'a, C> {
    erased: &'a mut dyn AnyComponent,
    _marker: PhantomData<fn() -> C>,
}

impl<'a, C: 'static> TryFrom<&'a mut dyn AnyComponent> for UnwrapAnyComponent<'a, C> {
    type Error = String;
    fn try_from(value: &'a mut dyn AnyComponent) -> Result<Self, String> {
        if !value.component().is::<C>() {
            let ty = std::any::type_name::<C>();
            return Err(format!("AnyComponent: incompatible value type, expecting {ty}"));
        }
        Ok(UnwrapAnyComponent { erased: value, _marker: PhantomData })
    }
}

impl<'a, C: 'static> AnyComponent<C> for UnwrapAnyComponent<'a, C> {
    fn component(&self) -> &C { self.erased.component().downcast_ref().unwrap() }
    fn component_mut(&mut self) -> &mut C { self.erased.component_mut().downcast_mut().unwrap() }
}

/// Type-erased [`Curve`]. Prefer [`Curve`] whenever possible.
pub trait AnyCurve: Send + Sync + 'static {
    /// Get a descriptor for this [`Curve`].
    fn descriptor(&self) -> CurveDescriptor;
    /// Delegate to [`Curve::frame_count`].
    fn get_frame_count(&self) -> usize;
    /// Delegate to [`Curve::apply_sampled`].
    fn apply_sampled_any(&self, segment: Segment, frame: f32, output: &mut dyn AnyComponent) -> Result<(), String>;
}

fn attach_binding<C: 'static>(mut entity: EntityCommands, info: CurveBindingInfo) {
    entity.insert(CurveBinding::<C>::new(info));
}

impl<T: Curve> AnyCurve for T {
    fn descriptor(&self) -> CurveDescriptor { CurveDescriptor::new::<T::Component>() }
    fn get_frame_count(&self) -> usize { self.frame_count() }
    fn apply_sampled_any(&self, segment: Segment, frame: f32, output: &mut dyn AnyComponent) -> Result<(), String> {
        let output = UnwrapAnyComponent::try_from(output)?;
        self.apply_sampled(segment, frame, output)
    }
}

/// Animation curve with known single variable type.
pub trait TypedCurve: Curve {
    /// Value type for this curve.
    type Value: PartialEq + 'static;
    /// Field accessor.
    type FieldAccessor
    : for<'a> AffineFoldRef<'a, Self::Component, View=Self::Value, Error=String>
    + for<'a> AffineFoldMut<'a, Self::Component, View=Self::Value, Error=String>;
    /// Sample the curve at specific frame index.
    ///
    /// To support looping, we require `sample` be well-defined for any positive frame number. The
    /// curve should behave as if the first frame immediately follows the last frame. However, it
    /// is okay for this sampling function to assume `start + frame <= end + 1` will always hold.
    fn sample(&self, segment: Segment, frame: f32) -> Option<Self::Value>;
    /// Get a field accessor for the targeted field.
    fn field_accessor(&self) -> &Self::FieldAccessor;
    /// Update the field in the component with a new value.
    fn update_field(&self, mut target: impl AnyComponent<Self::Component>, value: Self::Value) -> Result<(), String> {
        let field = self.field_accessor();
        if *field.preview_ref(target.component())? != value {
            *field.preview_mut(target.component_mut())? = value;
        }
        Ok(())
    }
}
