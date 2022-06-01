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

//! Common behaviours for plants or zombies.

use bevy::prelude::*;

/// Periodic behaviours.
#[derive(Debug, Copy, Clone, Component)]
pub struct Periodic<B> {
    /// Time period for this behaviour.
    pub period: f32,
    /// Current cool down for this behaviour, down from `period` to 0.
    pub cool_down: f32,
    /// Base behaviour to be repeated periodically.
    pub base_behaviour: B,
}
