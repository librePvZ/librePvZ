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

//! Asset loading logic (including the failure screen).

use bevy::prelude::*;

/// Default asset loading states.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash, States)]
pub enum AssetState {
    /// State where at least one asset in one asset collection is loading.
    #[default]
    AssetLoading,
    /// All assets in all asset collections have finished loading successfully.
    AssetReady,
    /// At least one asset in one asset collection failed loading.
    LoadFailure,
}
