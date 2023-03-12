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
use bevy_asset_loader::prelude::*;
use libre_pvz_resources::model::{MarkerRegistry, Model};
use crate::animation::transform::{SpriteBundle2D, Transform2D, SpatialBundle2D};
use crate::core::projectile::VanishingBound;
use crate::plant::peashooter::PeashooterAssets;
use crate::resources::animation::Animation;
use crate::scene::loading::AssetState;

#[allow(unused)]
const BKG_WIDTH: f32 = 1400.0;
const BKG_HEIGHT: f32 = 600.0;

// ensure 16:9 aspect ratio
const WIDTH: f32 = HEIGHT * 16.0 / 9.0;
const HEIGHT: f32 = BKG_HEIGHT;

/// Plugin for lawn scene.
#[derive(Default, Debug, Copy, Clone)]
pub struct LawnPlugin;

impl LawnPlugin {
    /// Mainly for setting the window size.
    pub fn window() -> Window {
        Window {
            resolution: (WIDTH, HEIGHT).into(),
            title: "librePvZ".to_string(),
            resizable: false,
            ..Window::default()
        }
    }
}

impl Plugin for LawnPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GridInfo::default())
            .insert_resource(VanishingBound {
                top: HEIGHT * 0.6,
                bottom: -HEIGHT * 0.6,
                left: -WIDTH * 0.6,
                right: WIDTH * 0.6,
                maximum_height: HEIGHT * 2.0,
            })
            .add_loading_state(LoadingState::new(AssetState::AssetLoading)
                .continue_to_state(AssetState::AssetReady)
                .on_failure_continue_to_state(AssetState::LoadFailure))
            .add_collection_to_loading_state::<_, LawnAssets>(AssetState::AssetLoading)
            .add_collection_to_loading_state::<_, PeashooterAssets>(AssetState::AssetLoading)
            .add_system(setup_lawn.in_schedule(OnEnter(AssetState::AssetReady)))
            .add_system(spawn_peashooter_system.in_schedule(OnEnter(AssetState::AssetReady)))
            .add_system(update_grid_system.in_set(OnUpdate(AssetState::AssetReady)));
    }
}

#[derive(AssetCollection, Resource)]
struct LawnAssets {
    #[asset(path = "background1.jpg")]
    lawn_background: Handle<Image>,
    #[asset(path = "plantshadow.png")]
    plant_shadow: Handle<Image>,
}

const PLANT_WIDTH: f32 = 80.0;
const PLANT_HALF_WIDTH: f32 = PLANT_WIDTH / 2.0;
const PLANT_TRANSLATION: Vec2 = Vec2::new(-PLANT_HALF_WIDTH, PLANT_HALF_WIDTH);

impl LawnAssets {
    /// Spawn a plant with shadow from its model.
    pub fn spawn_plant(&self, model: Handle<Model>,
                       animations: &Assets<Animation>, models: &Assets<Model>,
                       markers: &MarkerRegistry, commands: &mut Commands) {
        // the parent entity for the whole plant
        let mut trans = SpatialBundle2D::default();
        trans.local.z_order = 10.0;
        let parent = commands.spawn((trans, GridPos { x: 2, y: 2 })).id();
        // the shadow
        let mut shadow = SpriteBundle2D::default();
        shadow.transform.translation.y = -30.0;
        shadow.transform.z_order = -1.0;
        shadow.texture = self.plant_shadow.clone();
        let shadow = commands.spawn(shadow).id();
        commands.entity(parent).add_child(shadow);
        // the main part of the plant
        let plant = Model::spawn(model, PLANT_TRANSLATION, animations, models, markers, commands);
        let plant = match plant {
            Ok(plant) => plant,
            Err(err) => return error!("failed to spawn plant model: {err}"),
        };
        commands.entity(parent).add_child(plant);
    }
}

#[allow(unused)]
const GRID_X_COUNT: u8 = 9;
#[allow(unused)]
const GRID_Y_COUNT: u8 = 5;

const GRID_SIZE: f32 = 80.0;

fn setup_lawn(assets: Res<LawnAssets>, mut commands: Commands) {
    commands.spawn(SpriteBundle2D {
        sprite: Sprite {
            anchor: Anchor::TopLeft,
            ..Sprite::default()
        },
        texture: assets.lawn_background.clone(),
        transform: Transform2D::from_translation(Vec2::new(-WIDTH / 2.0, HEIGHT / 2.0)),
        ..SpriteBundle2D::default()
    });
}

#[derive(Copy, Clone, PartialEq, Resource)]
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

/// Position in the lawn grid.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Component)]
pub struct GridPos {
    /// Horizontal, larger means farther from the door (left to right).
    pub x: u8,
    /// Vertical, larger means farther from the seed slot (top down).
    pub y: u8,
}

impl GridInfo {
    fn translation_for(&self, pos: &GridPos) -> Vec2 {
        Vec2::new(
            -WIDTH / 2.0 + self.top_left.x + GRID_SIZE / 2.0 + pos.x as f32 * (self.separator.x + GRID_SIZE),
            HEIGHT / 2.0 - self.top_left.y - GRID_SIZE / 2.0 - pos.y as f32 * (self.separator.y + GRID_SIZE),
        )
    }
}

fn update_grid_system(
    grid_info: Res<GridInfo>,
    mut grids: Query<(Ref<GridPos>, &mut Transform2D)>,
) {
    for (pos, mut trans) in grids.iter_mut() {
        if grid_info.is_changed() || pos.is_changed() {
            trans.translation = grid_info.translation_for(&pos);
        }
    }
}

fn spawn_peashooter_system(
    lawn_assets: Res<LawnAssets>,
    peashooter_assets: Res<PeashooterAssets>,
    animations: Res<Assets<Animation>>,
    models: Res<Assets<Model>>,
    markers: Res<MarkerRegistry>,
    mut commands: Commands,
) {
    lawn_assets.spawn_plant(
        peashooter_assets.model.clone(),
        &animations, &models, &markers, &mut commands,
    );
}
