/*
 * librePvZ-resources: resource loading for librePvZ.
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

//! librePvZ-resources: resource loading logics for librePvZ.

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

// utilities
pub mod dynamic;
pub mod cached;
pub mod loader;

// contents
pub mod animation;
pub mod model;

pub use once_cell;

use bevy::prelude::*;
use bevy::app::PluginGroupBuilder;

use animation::AnimationPlugin;
use model::ModelPlugin;

/// Resources plugin group.
#[derive(Default, Debug, Copy, Clone)]
pub struct ResourcesPlugins;

impl PluginGroup for ResourcesPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<ResourcesPlugins>()
            .add(AnimationPlugin)
            .add(ModelPlugin)
    }
}
