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

// use std::path::{Path, PathBuf};
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use libre_pvz::animation::AnimationPlugin;
use libre_pvz::core::kinematics::KinematicsPlugin;
use libre_pvz::core::projectile::ProjectilePlugin;
use libre_pvz::diagnostics::BoundingBoxPlugin;
use libre_pvz::plant::peashooter::PeashooterPlugin;
// use libre_pvz::scene::almanac::AlmanacPlugin;
use libre_pvz::scene::lawn::LawnPlugin;
use libre_pvz::resources::ResourcesPlugins;
use libre_pvz::scene::loading::AssetState;
use libre_pvz::seed_bank::SeedBankPlugin;

fn main() {
    // let anim_name: Box<Path> = match std::env::args_os().into_iter().nth(1) {
    //     None => AsRef::<Path>::as_ref("Peashooter.anim.bin").into(),
    //     Some(path) => PathBuf::from(path).into(),
    // };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(LawnPlugin::window()),
            // primary_window: Some(AlmanacPlugin::window()),
            ..default()
        }))
        .init_state::<AssetState>()
        .add_plugins((
            EguiPlugin,
            BoundingBoxPlugin,
            AnimationPlugin,
            ResourcesPlugins,
            ProjectilePlugin,
            KinematicsPlugin,
            PeashooterPlugin,
            LawnPlugin,
            SeedBankPlugin,
            // AlmanacPlugin::new(anim_name),
            WorldInspectorPlugin::new(),
        ))
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
