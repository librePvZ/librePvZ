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

//! Bevy related utilities.

use bevy::prelude::*;

pub mod loader;
pub mod animation;

pub use loader::{AddTwoStageAsset, TwoStageAssetPlugin};
pub use animation::Animation;

use animation::AnimationLoader;

/// Resources plugin.
#[derive(Default, Debug, Copy, Clone)]
pub struct ResourcesPlugin;

impl Plugin for ResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Animation>()
            .init_two_stage_asset_loader::<AnimationLoader>();
    }
}
