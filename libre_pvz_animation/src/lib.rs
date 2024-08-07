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

//! librePvZ-animation: animation playing for librePvZ for [`bevy`].
#![doc = include_str!("../README.md")]

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

pub mod transform;
pub mod curve;
pub mod clip;
pub mod player;

use bevy::prelude::*;
use bevy::transform::TransformSystem;
use crate::transform::Transform2D;

/// Labels for animation systems.
#[derive(Clone, Debug, SystemSet, PartialEq, Eq, Hash)]
pub enum AnimationSystem {
    /// Ticking the time in animation players.
    PlayerTicking,
    /// Initialize/update curve bindings.
    PlayerCurveBind,
    /// Sample the curves and apply to the entities.
    PlayerSampling,
}

/// Extend [`App`] with an `register_for_animation` API.
pub trait AnimationExt {
    /// Register a [`Component`] for animation.
    fn register_for_animation<C: Component>(&mut self) -> &mut Self;
}

impl AnimationExt for App {
    fn register_for_animation<C: Component>(&mut self) -> &mut Self {
        self.add_systems(PostUpdate, player::animate_entities_system::<C>
            .in_set(AnimationSystem::PlayerSampling))
    }
}

/// Plugin for animation playing.
#[allow(missing_debug_implementations)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Transform2D>()
            .init_asset::<clip::AnimationClip>()
            .add_systems(PostUpdate, transform::transform_propagate_system.in_set(TransformSystem::TransformPropagate))
            .add_systems(PostUpdate, player::bind_curve_system.in_set(AnimationSystem::PlayerCurveBind))
            .add_systems(PostUpdate, player::tick_animation_system.in_set(AnimationSystem::PlayerTicking))
            .configure_sets(PostUpdate, (
                AnimationSystem::PlayerTicking,
                AnimationSystem::PlayerCurveBind,
                AnimationSystem::PlayerSampling.before(TransformSystem::TransformPropagate),
            ))
            .register_for_animation::<Transform2D>()
            .register_for_animation::<Sprite>()
            .register_for_animation::<Visibility>()
            .register_for_animation::<Handle<Image>>();
    }
}
