/*
 * librePvZ: game logic implementation.
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

//! Kinematics for projectiles and other movable entities.
//!
//! All the movable entities live in a right-handed pseudo-3D world. The XY plane lies on the
//! ground, with the X axis points right, and the Y axis points up. The Z axis is for height, and
//! coincides with Y axis on the viewport. This coordinate system has the following benefits:
//! - it easily translates to the viewport coordinate (`x <- x`, `y <- y + z`);
//! - it avoids false collision (it distinguishes between "at the back" and "high in the sky");
//! - it allows objects to correctly drop their shadows;
//!
//! All entities have a planar collide box in the 2D world. During collision detection, the collide
//! boxes are laid out parallel to the XZ plane, and then pushed along the Y axis to form a cuboid.

use bevy::prelude::*;
use bevy::transform::TransformSystem;
use crate::animation::transform::Transform2D;

/// Kinematics plugin for all movable entities.
#[derive(Default, Copy, Clone)]
#[allow(missing_debug_implementations)]
pub struct KinematicsPlugin;

impl Plugin for KinematicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .configure_sets((
                KineticsSystem::Acceleration,
                KineticsSystem::Movement,
                KineticsSystem::CoordinateTranslation
                    .before(TransformSystem::TransformPropagate),
            ).in_base_set(CoreSet::PostUpdate).chain())
            .add_system(coordinate_translation_system.in_set(KineticsSystem::CoordinateTranslation))
            .add_system(movement_system.in_set(KineticsSystem::Movement))
            .add_system(acceleration_system.in_set(KineticsSystem::Acceleration));
    }
}

/// System label for kinetics systems.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, SystemSet)]
pub enum KineticsSystem {
    /// Update velocity from acceleration.
    Acceleration,
    /// Update position from velocity.
    Movement,
    /// Coordinate translation from pseudo 3D to 2D.
    CoordinateTranslation,
}

/// Position.
#[derive(Debug, Copy, Clone, Component)]
pub struct Position(pub Vec3);

/// Coordinate translation from pseudo 3D to 2D.
#[allow(clippy::type_complexity)]
pub fn coordinate_translation_system(mut objects: Query<(&Position, &mut Transform2D), Changed<Position>>) {
    for (pos, mut trans) in objects.iter_mut() {
        trans.translation.x = pos.0.x;
        trans.translation.y = pos.0.y + pos.0.z;
    }
}

/// Velocity (in length per second).
#[derive(Debug, Copy, Clone, Component)]
pub struct Velocity(pub Vec3);

/// Update position from velocity.
pub fn movement_system(time: Res<Time>, mut objects: Query<(&Velocity, &mut Position)>) {
    for (vel, mut pos) in objects.iter_mut() {
        pos.0 += time.delta_seconds() * vel.0;
    }
}

/// Acceleration (in length per square second).
#[derive(Debug, Copy, Clone, Component)]
pub struct Acceleration(pub Vec3);

/// Update velocity from acceleration.
pub fn acceleration_system(time: Res<Time>, mut objects: Query<(&Acceleration, &mut Velocity)>) {
    for (acc, mut vel) in objects.iter_mut() {
        vel.0 += time.delta_seconds() * acc.0;
    }
}
