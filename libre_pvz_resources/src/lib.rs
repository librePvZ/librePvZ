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

use bevy::prelude::*;
use animation::Animation;
use loader::AddTwoStageAsset;

use crate::model::*;

/// Resources plugin.
#[derive(Default, Debug, Copy, Clone)]
pub struct ResourcesPlugin;

impl Plugin for ResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MarkerRegistry>()
            .add_two_stage_asset::<Animation>()
            .add_event::<StateTransitionEvent>()
            .add_event::<TransitionTrigger>()
            .add_two_stage_asset::<Model>()
            .register_marker::<AutoNullTrigger>("AutoNullTrigger");
    }
}
