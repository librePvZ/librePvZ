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

use bevy::prelude::*;
use bevy_egui::{EguiContext, EguiPlugin};
use libre_pvz::resources::bevy::{Animation, AnimationLoader};
use libre_pvz_animation::AnimationPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AnimationPlugin)
        .add_plugin(EguiPlugin)
        .add_asset::<Animation>()
        .init_asset_loader::<AnimationLoader>()
        .add_startup_system(setup_camera)
        .add_startup_system(load_anim)
        .add_system(init_anim)
        .add_system(animation_ui)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

struct Stage(Handle<Animation>);

fn load_anim(server: Res<AssetServer>, mut commands: Commands) {
    let anim = server.load("Cabbagepult.anim");
    commands.insert_resource(Stage(anim));
}

fn init_anim(mut ev_anim: EventReader<AssetEvent<Animation>>,
             assets: Res<Assets<Animation>>,
             mut commands: Commands) {
    for event in ev_anim.iter() {
        if let AssetEvent::Created { handle } = event {
            let anim = assets.get(handle).unwrap();
            anim.spawn_on("anim_idle", &mut commands);
        }
    }
}

fn animation_ui(mut context: ResMut<EguiContext>,
                animations: Res<Assets<Animation>>,
                stage: Res<Stage>) {
    if let Some(anim) = animations.get(&stage.0) {
        egui::Window::new("Animation").show(context.ctx_mut(), |ui| {
            for meta in anim.description.meta.iter() {
                ui.label(&meta.name);
            }
        });
    }
}
