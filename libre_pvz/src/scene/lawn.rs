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

//! The lawn scene (the battlefield).

use bevy::prelude::*;
use bevy::sprite::Anchor;
use crate::animation::transform::{SpriteBundle2D, Transform2D, TransformBundle2D};
use crate::resources::bevy::Animation;
use crate::scene::loading::{AssetCollection, AssetLoader, AssetLoaderExt, AssetState, PendingAssets};

const BKG_IMAGE: &str = "background1.jpg";
#[allow(unused)]
const BKG_WIDTH: f32 = 1400.0;
const BKG_HEIGHT: f32 = 600.0;

// ensure 16:9 aspect ratio
const WIDTH: f32 = HEIGHT * 16.0 / 9.0;
const HEIGHT: f32 = BKG_HEIGHT;

const PEASHOOTER_ANIM: &str = "Peashooter.anim";

/// Plugin for lawn scene.
#[derive(Default, Debug, Copy, Clone)]
pub struct LawnPlugin;

impl LawnPlugin {
    /// Mainly for setting the window size.
    pub fn window_descriptor() -> WindowDescriptor {
        WindowDescriptor {
            width: WIDTH,
            height: HEIGHT,
            title: "librePvZ".to_string(),
            resizable: false,
            ..WindowDescriptor::default()
        }
    }
}

impl Plugin for LawnPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GridInfo::default())
            .attach_loader(AssetLoader::default()
                .with_collection::<LawnAssets>()
                .enable_failure_ui())
            .add_system_set(SystemSet::on_enter(AssetState::AssetReady).with_system(setup_lawn))
            .add_system_set(SystemSet::on_update(AssetState::AssetReady).with_system(update_grid_system));
    }
}

struct LawnAssets {
    lawn_background: Handle<Image>,
    peashooter_anim: Handle<Animation>,
}

impl AssetCollection for LawnAssets {
    fn load(world: &World) -> (Self, PendingAssets<Self>) {
        let asset_server = world.resource::<AssetServer>();
        let mut pending = PendingAssets::new();
        let assets = LawnAssets {
            lawn_background: pending.load_from(asset_server, BKG_IMAGE),
            peashooter_anim: pending.load_from(asset_server, PEASHOOTER_ANIM),
        };
        (assets, pending)
    }
    fn track_dep(&self, handle: HandleUntyped, world: &World, pending: &mut PendingAssets<Self>) {
        if handle.id == self.peashooter_anim.id {
            let anim = world.resource::<Assets<Animation>>().get(handle).unwrap();
            for (path, image) in &anim.images {
                pending.track(path, image.clone());
            }
        }
    }
}

const GRID_X_COUNT: u8 = 9;
const GRID_Y_COUNT: u8 = 5;

const GRID_SIZE: f32 = 80.0;

fn setup_lawn(assets: Res<LawnAssets>, animations: Res<Assets<Animation>>, mut commands: Commands) {
    commands.spawn_bundle(SpriteBundle2D {
        sprite: Sprite {
            anchor: Anchor::TopLeft,
            ..Sprite::default()
        },
        texture: assets.lawn_background.clone(),
        transform: Transform2D::from_translation(Vec2::new(-WIDTH / 2.0, HEIGHT / 2.0)),
        ..SpriteBundle2D::default()
    });
    let peashooter = animations.get(&assets.peashooter_anim).unwrap();
    for x in 0..GRID_X_COUNT {
        for y in 0..GRID_Y_COUNT {
            let mut trans = TransformBundle2D::default();
            trans.local.z_order = 10.0;
            let parent = commands.spawn_bundle(trans).insert(GridPos { x, y }).id();
            let (idle, _) = peashooter.description.get_meta("anim_full_idle").unwrap();
            let child = peashooter.spawn_on(idle, &mut commands);
            commands.entity(parent).add_child(child);
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
struct GridInfo {
    top_left: Vec2,
    separator: Vec2,
}

impl Default for GridInfo {
    fn default() -> Self {
        GridInfo {
            top_left: Vec2::new(255.0, 85.0),
            separator: Vec2::new(0.0, 20.0),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Component)]
struct GridPos {
    x: u8,
    y: u8,
}

impl GridInfo {
    fn translation_for(&self, pos: &GridPos) -> Vec2 {
        Vec2::new(
            -WIDTH / 2.0 + self.top_left.x + GRID_SIZE / 2.0 + pos.x as f32 * (self.separator.x + GRID_SIZE),
            HEIGHT / 2.0 - self.top_left.y - GRID_SIZE / 2.0 - pos.y as f32 * (self.separator.y + GRID_SIZE),
        )
    }
}

#[allow(clippy::type_complexity)]
fn update_grid_system(
    grid_info: Res<GridInfo>,
    mut grids: Query<
        (&GridPos, &mut Transform2D),
        Or<(Added<GridPos>, Changed<GridPos>)>,
    >,
) {
    if grid_info.is_added() || grid_info.is_changed() {
        for (pos, mut trans) in grids.iter_mut() {
            trans.translation = grid_info.translation_for(pos);
        }
    }
}
