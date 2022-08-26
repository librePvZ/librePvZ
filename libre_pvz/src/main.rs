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
use bevy_inspector_egui::WorldInspectorPlugin;
use libre_pvz::animation::AnimationPlugin;
use libre_pvz::core::kinematics::KinematicsPlugin;
use libre_pvz::core::projectile::ProjectilePlugin;
use libre_pvz::diagnostics::BoundingBoxPlugin;
use libre_pvz::plant::peashooter::PeashooterPlugin;
// use libre_pvz::scene::almanac::AlmanacPlugin;
use libre_pvz::scene::lawn::LawnPlugin;
use libre_pvz_resources::ResourcesPlugins;

fn main() {
    // let anim_name: Box<Path> = match std::env::args_os().into_iter().nth(1) {
    //     None => AsRef::<Path>::as_ref("Peashooter.anim.bin").into(),
    //     Some(path) => PathBuf::from(path).into(),
    // };

    App::new()
        .insert_resource(LawnPlugin::window_descriptor())
        // .insert_resource(AlmanacPlugin::window_descriptor())
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(BoundingBoxPlugin)
        .add_plugin(AnimationPlugin)
        .add_plugins(ResourcesPlugins)
        .add_plugin(ProjectilePlugin)
        .add_plugin(KinematicsPlugin)
        .add_plugin(PeashooterPlugin)
        .add_plugin(LawnPlugin)
        // .add_plugin(AlmanacPlugin::new(anim_name))
        .add_startup_system(setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}
