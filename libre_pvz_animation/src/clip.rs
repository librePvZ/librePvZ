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
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use bevy::prelude::*;
use bevy::reflect::{TypeUuid, TypeRegistry, TypeRegistryInternal};
use bevy::tasks::ComputeTaskPool;
use bevy::utils::tracing::warn;
use thiserror::Error;
use crate::key_frame::Curve;

const NANOS_PER_SEC: u32 = 1_000_000_000;

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
    path_mapping: Box<[(EntityPath, u32, u32)]>,
    curves: Box<[Box<dyn Curve>]>,
    duration_nanos: u64,
}

impl AnimationClip {
    /// Get a builder to build an animation clip.
    pub fn builder() -> AnimationClipBuilder { AnimationClipBuilder::new() }
    /// Get the duration of this animation clip, i.e., the maximum duration of all curves.
    pub fn duration_nanos(&self) -> u64 { self.duration_nanos }
    /// Get an iterator of [`Curve`]s into this animation clip.
    pub fn iter(&self) -> std::slice::Iter<(EntityPath, u32, u32)> { self.path_mapping.iter() }
    /// Get the [`Curve`] at index `k`.
    pub fn get(&self, k: u32) -> &dyn Curve { self.curves[k as usize].as_ref() }
}

/// Builder for [`AnimationClip`]s.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct AnimationClipBuilder {
    curves: BTreeMap<EntityPath, Vec<Box<dyn Curve>>>,
}

impl AnimationClipBuilder {
    /// Get a builder to build an animation clip.
    pub fn new() -> AnimationClipBuilder { AnimationClipBuilder::default() }

    /// Add a new curve into the clip.
    pub fn add_curve(&mut self, path: EntityPath, curve: impl Curve) {
        self.add_dyn_curve(path, Box::new(curve))
    }

    /// Add a new curve into the clip.
    pub fn add_dyn_curve(&mut self, path: EntityPath, curve: Box<dyn Curve>) {
        self.curves.entry(path).or_insert_with(Vec::new).push(curve);
    }

    /// Finish building the clip.
    pub fn build(self) -> AnimationClip {
        let mut path_mapping = Vec::new();
        let mut curves = Vec::new();
        let mut duration = 0_f32;
        for (path, curve) in self.curves {
            let start = curves.len();
            let end = start + curve.len();
            path_mapping.push((path, start as u32, end as u32));
            curves.extend(curve.into_iter().inspect(|c|
                duration = duration.max(c.duration())));
        }
        AnimationClip {
            path_mapping: path_mapping.into_boxed_slice(),
            curves: curves.into_boxed_slice(),
            duration_nanos: (duration * NANOS_PER_SEC as f32).round() as u64,
        }
    }
}

/// Animation player.
#[derive(Component)]
#[allow(missing_debug_implementations)]
pub struct AnimationPlayer {
    paused: bool,
    repeat: bool,
    speed: u32,
    elapsed_nanos: u64,
    clip: Arc<AnimationClip>,
}

// speed is multiplied with this factor, and then rounded to integer.
const SPEED_FACTOR: u64 = 1_000_000;

impl AnimationPlayer {
    /// Create an animation player that plays the specific clip.
    pub fn new(clip: Arc<AnimationClip>, speed: f32, repeat: bool) -> Self {
        let speed = (speed * SPEED_FACTOR as f32).round() as u32;
        AnimationPlayer { paused: false, repeat, speed, elapsed_nanos: 0, clip }
    }

    /// Pause the animation.
    pub fn pause(&mut self) { self.paused = true }
    /// Resume the animation.
    pub fn resume(&mut self) { self.paused = false }

    /// Tick the time by several seconds.
    pub fn tick(&mut self, delta: Duration) {
        if self.paused { return; }
        // [correctness: truncate delta to u64]
        // assuming a speed of 1.0 (therefore self.speed = SPEED_FACTOR)
        // we need delta > 5 hours to overflow nanoseconds in u64
        // but valid input for this method is typically within 1 second
        self.elapsed_nanos += (delta.as_nanos() as u64 * self.speed as u64) / SPEED_FACTOR;
        if self.repeat {
            self.elapsed_nanos %= self.clip.duration_nanos;
        } else {
            self.elapsed_nanos = 0;
            self.paused = true;
        }
    }

    /// Get elapsed time in seconds.
    pub fn elapsed(&self) -> f32 {
        self.elapsed_nanos as f32 / NANOS_PER_SEC as f32
    }
}

#[derive(Component)]
pub(crate) struct CurveBinding(Entity, u32, u32);

pub(crate) fn bind_curve_system(
    mut players: Query<(Entity, &AnimationPlayer), Added<AnimationPlayer>>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut commands: Commands,
) {
    for (root, player) in players.iter_mut() {
        for (path, start, end) in player.clip.iter() {
            if let Some(entity) = locate(root, path, &children, &names) {
                commands.entity(entity).insert(CurveBinding(root, *start, *end));
            }
        }
    }
}

fn locate(
    root: Entity, path: &EntityPath,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    let mut current = root;
    'name: for name in path.iter() {
        for &child in children.get(current).ok()?.iter() {
            if names.get(child) != Ok(name) { continue; }
            current = child;
            continue 'name;
        }
        return None;
    }
    Some(current)
}

const BINDING_BATCH_SIZE: usize = 8;

pub(crate) fn tick_animation_system(time: Res<Time>, mut players: Query<&mut AnimationPlayer>) {
    for mut player in players.iter_mut() {
        player.tick(time.delta());
    }
}

pub(crate) fn animate_entities_system(
    world: &World,
    entities: Query<(Entity, &CurveBinding)>,
    players: Query<&AnimationPlayer>,
    type_registry: Res<TypeRegistry>,
    task_pool: Res<ComputeTaskPool>,
) {
    let type_registry = type_registry.read();
    entities.par_for_each(task_pool.as_ref(), BINDING_BATCH_SIZE, |(entity, binding)|
        animate_entity(entity, binding, &players, type_registry.deref(), world));
}

fn animate_entity(
    entity: Entity,
    binding: &CurveBinding,
    players: &Query<&AnimationPlayer>,
    type_registry: &TypeRegistryInternal,
    world: &World,
) {
    let player = players.get(binding.0).unwrap();
    let range = binding.1 as usize..binding.2 as usize;
    for curve in &player.clip.curves[range] {
        let result = unsafe {
            sample_curve_component(
                curve.as_ref(),
                player.elapsed(),
                entity, world,
                type_registry,
            )
        };
        if let Err(err) = result {
            warn!("animation error: {err}");
        }
    }
}

/// Error for access into entities with some path.
#[derive(Debug, Clone, Error)]
pub enum AccessPathError {
    /// Cannot retrieve a data about the target type being a [`Component`](bevy::ecs::component::Component).
    #[error("type '{0}' is not known as a component")]
    NotKnownAsComponent(&'static str),
    /// No such [`Component`](bevy::ecs::component::Component) present on specified entity.
    #[error("component with type '{0}' not found")]
    NoSuchComponent(&'static str),
    /// Error during a field access using the field descriptor in a curve.
    #[error("type '{0}' does not have required field: {1}")]
    NoSuchField(&'static str, Box<str>),
}

unsafe fn sample_curve_component<'a>(
    curve: &dyn Curve, time: f32,
    entity: Entity, world: &'a World,
    type_registry: &TypeRegistryInternal,
) -> Result<(), AccessPathError> {
    let component = type_registry
        .get_type_data::<ReflectComponent>(curve.component_type())
        .ok_or_else(|| AccessPathError::NotKnownAsComponent(curve.component_type_name()))?
        .reflect_component_unchecked_mut(world, entity)
        .ok_or_else(|| AccessPathError::NoSuchComponent(curve.component_type_name()))?;
    curve.apply_sampled(time, component.into_inner()).map_err(|err|
        AccessPathError::NoSuchField(curve.component_type_name(), err))
}
