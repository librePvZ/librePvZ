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

//! Almanac screen.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use libre_pvz_animation::transform::{SpriteBundle2D, Transform2D};

/// The almanac entity.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Almanac(pub Entity);

/// Width of almanac frame.
pub const WIDTH: f32 = 315.0;
/// Height of almanac frame.
pub const HEIGHT: f32 = 472.0;
/// Path to the almanac image.
pub const ALMANAC_IMAGE: &str = "Almanac_PlantCard.png";

/// Size of the ground image.
pub const GROUND_SIZE: f32 = 200.0;
/// Path to the ground image.
pub const GROUND_IMAGE: &str = "Almanac_GroundDay.jpg";

/// Rectangle for the preview window.
/// Note that our y-axis points up.
pub const WINDOW: Rect<f32> = Rect {
    left: 63.0,
    top: -22.0,
    right: 252.0,
    bottom: -165.0,
};
/// Window center position.
pub const WINDOW_CENTER: [f32; 2] = [
    (WINDOW.left + WINDOW.right) / 2.0,
    (WINDOW.top + WINDOW.bottom) / 2.0,
];

/// Ground center position.
pub const GROUND_CENTER: [f32; 2] = [158.0, -111.0];

/// Plant center position.
pub const PLANT_CENTER: [f32; 2] = [158.0, -93.0];

/// Description window top left corner.
/// This is UI element, the y-axis points down.
pub const DESCRIPTION_TOP_LEFT: [f32; 2] = [32.0, 231.0];
/// Description window width.
pub const DESCRIPTION_WIDTH: f32 = WIDTH - 2.0 * DESCRIPTION_TOP_LEFT[0];

/// Show almanac screen.
pub fn setup(server: Res<AssetServer>, mut commands: Commands) {
    let almanac = commands.spawn_bundle(SpriteBundle2D {
        texture: server.load(ALMANAC_IMAGE),
        sprite: Sprite {
            anchor: Anchor::TopLeft,
            ..Sprite::default()
        },
        transform: Transform2D {
            z_order: 100.0,
            translation: Vec2::new(-WIDTH / 2., HEIGHT / 2.),
            ..Transform2D::default()
        },
        ..SpriteBundle2D::default()
    }).id();
    let ground = commands.spawn_bundle(SpriteBundle2D {
        texture: server.load(GROUND_IMAGE),
        transform: Transform2D {
            z_order: -1.0,
            translation: Vec2::from(GROUND_CENTER),
            ..Transform2D::default()
        },
        ..SpriteBundle2D::default()
    }).id();
    commands.entity(almanac).add_child(ground);
    commands.insert_resource(Almanac(almanac));
}
