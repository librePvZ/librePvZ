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

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

pub mod transform;
pub mod reflect;
pub mod key_frame;
pub mod clip;

use bevy::prelude::*;
use bevy::transform::TransformSystem;
use crate::transform::Transform2D;

/// Labels for animation systems.
#[derive(Clone, Debug, SystemLabel, PartialEq, Eq, Hash)]
pub enum AnimationSystem {
    /// Ticking the time in animation players.
    PlayerTicking,
    /// Initialize/update curve bindings.
    PlayerCurveBind,
    /// Sample the curves and apply to the entities.
    PlayerSampling,
}

/// Plugin for animation playing.
#[allow(missing_debug_implementations)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Transform2D>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                transform::transform_propagate_system
                    .label(TransformSystem::TransformPropagate))
            .add_asset::<clip::AnimationClip>()
            .add_system(clip::bind_curve_system.label(AnimationSystem::PlayerCurveBind))
            .add_system(clip::tick_animation_system.label(AnimationSystem::PlayerTicking))
            .add_system_to_stage(
                CoreStage::PostUpdate,
                clip::animate_entities_system
                    .exclusive_system().at_start());
    }
}
