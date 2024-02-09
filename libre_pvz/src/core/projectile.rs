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

//! Projectile definition.

use std::ops::RangeInclusive;
use bevy::prelude::*;
use crate::core::kinematics::Position;

/// Projectile plugin.
#[derive(Default, Copy, Clone)]
#[allow(missing_debug_implementations)]
pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, projectile_vanish_system);
    }
}

/// Tag component for projectiles.
#[derive(Debug, Copy, Clone, Component)]
pub struct Projectile;

/// Projectiles outside this bound will vanish.
#[derive(Debug, Copy, Clone, Resource)]
pub struct VanishingBound {
    /// Top of this rectangle, larger in value.
    pub top: f32,
    /// Bottom of this rectangle, smaller in value.
    pub bottom: f32,
    /// Left of this rectangle, smaller in value.
    pub left: f32,
    /// Right of this rectangle, larger in value.
    pub right: f32,
    /// Maximum height (minimum height is always zero, i.e. the height of ground).
    pub maximum_height: f32,
}

impl VanishingBound {
    /// Vertical range, from `bottom` to `top`.
    pub fn vertical_range(&self) -> RangeInclusive<f32> { self.bottom..=self.top }
    /// Horizontal range, from `bottom` to `top`.
    pub fn horizontal_range(&self) -> RangeInclusive<f32> { self.left..=self.right }
    /// Test if a point is within this bound.
    pub fn contains(&self, p: &Position) -> bool {
        self.horizontal_range().contains(&p.0.x)
            && self.vertical_range().contains(&p.0.y)
            && p.0.z <= self.maximum_height
    }
}

/// Remove projectiles outside the [`VanishingBound`] to release resources.
pub fn projectile_vanish_system(
    bound: Res<VanishingBound>,
    projectiles: Query<(Entity, &Position)>,
    mut commands: Commands,
) {
    for (projectile, position) in projectiles.iter() {
        if !bound.contains(position) {
            commands.entity(projectile).despawn_recursive();
        }
    }
}
