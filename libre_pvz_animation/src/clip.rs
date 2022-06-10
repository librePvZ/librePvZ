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

//! Full animation clips.

use std::collections::BTreeMap;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::utils::HashMap;
use bevy::utils::label::DynHash;
use optics::traits::{AffineFoldMut, AffineFoldRef, Optics, OpticsKnownSource};
use crate::curve::animatable::Animatable;
use crate::curve::AnyCurve;
use crate::curve::builder::{AnyCurveBuilder, CurveBuilder, CurveContentBuilder};
use crate::curve::concrete::CurveContentStatic;

/// Entity path is all the [`Name`]s along the path.
#[derive(Debug, Hash, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct EntityPath(pub Box<[Name]>);

impl<const N: usize> From<[Name; N]> for EntityPath {
    fn from(path: [Name; N]) -> Self { EntityPath(Box::new(path) as _) }
}

impl EntityPath {
    /// Get an iterator into the fragments.
    pub fn iter(&self) -> std::slice::Iter<Name> { self.0.iter() }
}

/// Animation clip, core to the animation system.
#[allow(missing_debug_implementations)]
#[derive(TypeUuid)]
#[uuid = "1b7309a7-7e0f-4b83-8232-55fab5056334"]
pub struct AnimationClip {
    path_mapping: Box<[(EntityPath, u16, u16)]>,
    curves: Box<[Box<dyn AnyCurve>]>,
}

impl AnimationClip {
    /// Get a builder to build an animation clip.
    pub fn builder() -> AnimationClipBuilder { AnimationClipBuilder::new() }
    /// Get an iterator of [`Curve`](crate::curve::Curve)s into this animation clip.
    pub fn iter(&self) -> std::slice::Iter<(EntityPath, u16, u16)> { self.path_mapping.iter() }
    /// Get the [`Curve`](crate::curve::Curve) at index `k`.
    pub fn get(&self, k: u16) -> &dyn AnyCurve { self.curves[k as usize].as_ref() }
    /// Get the [`Curve`](crate::curve::Curve)s.
    pub fn curves(&self) -> &[Box<dyn AnyCurve>] { self.curves.as_ref() }
}

/// Builder for [`AnimationClip`]s.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct AnimationClipBuilder {
    curves: BTreeMap<EntityPath, Vec<Box<dyn AnyCurve>>>,
}

impl AnimationClipBuilder {
    /// Get a builder to build an animation clip.
    pub fn new() -> AnimationClipBuilder { AnimationClipBuilder::default() }

    /// Add a new curve into the clip.
    pub fn add_curve(&mut self, path: EntityPath, curve: impl AnyCurve) {
        self.add_dyn_curve(path, Box::new(curve))
    }

    /// Add a new curve into the clip.
    pub fn add_dyn_curve(&mut self, path: EntityPath, curve: Box<dyn AnyCurve>) {
        self.curves.entry(path).or_insert_with(Vec::new).push(curve);
    }

    /// Add a whole new track into the clip.
    pub fn add_track(&mut self, path: EntityPath, track: Track) {
        let old = self.curves.insert(path, track.0);
        assert!(old.is_none());
    }

    /// Finish building the clip.
    pub fn build(self) -> AnimationClip {
        let mut path_mapping = Vec::new();
        let mut curves = Vec::new();
        for (path, mut curve) in self.curves {
            let start = curves.len();
            let end = start + curve.len();
            path_mapping.push((path, start as u16, end as u16));
            curve.sort_unstable_by_key(|c| c.descriptor());
            curves.extend(curve.into_iter());
        }
        AnimationClip {
            path_mapping: path_mapping.into_boxed_slice(),
            curves: curves.into_boxed_slice(),
        }
    }
}

/// An animation track, i.e. all curves for some entity.
#[allow(missing_debug_implementations)]
pub struct Track(Vec<Box<dyn AnyCurve>>);

/// Types suitable to be used as labels for curves in a [`TrackBuilder`].
pub trait CurveLabel: DynHash {}

impl<T: DynHash> CurveLabel for T {}

impl PartialEq for dyn CurveLabel {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other.as_dyn_eq())
    }
}

impl Eq for dyn CurveLabel {}

impl Hash for dyn CurveLabel {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state)
    }
}

/// Builder for animation [`Track`]s, building [`Curve`](crate::curve::Curve)s from scratch.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct TrackBuilder {
    curves: HashMap<Box<dyn CurveLabel>, Box<dyn AnyCurveBuilder>>,
}

impl TrackBuilder {
    /// Prepare a curve in this track, e.g., to use optimised storage.
    /// Main usage is to specify the use of `BitVec` (`BitBox`) for boolean curves.
    ///
    /// **Note:** panics if a curve already exists for the `field_path`.
    pub fn prepare_curve<C, F>(&mut self, field_path: F)
        where C: CurveContentBuilder,
              F::Source: Sized + 'static, F::Error: Display,
              F::View: PartialEq + Animatable + Send + Sync + 'static,
              F: OpticsKnownSource
              + Optics<F::Source, View=<C::Target as CurveContentStatic>::Keyframe>
              + for<'a> AffineFoldRef<'a, F::Source>
              + for<'a> AffineFoldMut<'a, F::Source>
              + Clone + Hash + Eq + Send + Sync + 'static {
        let old = self.curves.insert(
            Box::new(field_path.clone()),
            CurveBuilder::<C>::new().into_dynamic(field_path),
        );
        assert!(old.is_none(), "cannot prepare an existing curve");
    }

    /// Push a keyframe into this track.
    /// The frame will end up in a curve determined by `field_path`.
    pub fn push_keyframe<F>(&mut self, field_path: F, frame: usize, value: F::View)
        where F::Source: Sized + 'static, F::Error: Display,
              F::View: PartialEq + Animatable + Sized + Send + Sync + 'static,
              F: OpticsKnownSource
              + for<'a> AffineFoldRef<'a, F::Source>
              + for<'a> AffineFoldMut<'a, F::Source>
              + Clone + Hash + Eq + Send + Sync + 'static {
        self.curves.entry(Box::new(field_path.clone())).or_insert_with(||
            CurveBuilder::<Vec<F::View>>::new().into_dynamic(field_path)
        ).push_keyframe(frame as u16, &mut Some(value));
    }

    /// Finish building this track.
    pub fn finish(self) -> Track {
        Track(self.curves.into_iter()
            .filter_map(|(_, c)| c.finish_boxed())
            .collect())
    }
}
