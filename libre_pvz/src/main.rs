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
use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy_egui::{EguiContext, EguiPlugin};
use egui::Slider;
use libre_pvz::resources::bevy::{Animation, AnimationLoader};
use libre_pvz_animation::{AnimationPlugin, clip::AnimationPlayer};

fn main() {
    let anim_name = AnimName(std::env::args().into_iter().nth(1)
        .unwrap_or_else(|| "Peashooter.anim".to_string()));
    App::new()
        .insert_resource(WindowDescriptor {
            width: 500.0,
            height: 500.0,
            title: "librePvZ".to_string(),
            ..WindowDescriptor::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(AnimationPlugin)
        .add_plugin(EguiPlugin)
        .add_asset::<Animation>()
        .init_asset_loader::<AnimationLoader>()
        .add_startup_system(setup_camera)
        .insert_resource(anim_name)
        .add_startup_system(load_anim)
        .add_system(init_anim)
        .add_system(animation_ui)
        .add_system(respond_to_stage_change)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

struct Stage {
    animation: Handle<Animation>,
    scaling_factor: f32,
    selected_meta: usize,
    last_selected_meta: usize,
}

struct AnimName(String);

fn load_anim(server: Res<AssetServer>, anim_name: Res<AnimName>, mut commands: Commands) {
    let animation = server.load(&anim_name.0);
    commands.insert_resource(Stage {
        animation,
        scaling_factor: 3.0,
        selected_meta: 0,
        last_selected_meta: 0,
    });
}

#[derive(Component)]
struct Scaling;

fn init_anim(mut ev_anim: EventReader<AssetEvent<Animation>>,
             assets: Res<Assets<Animation>>,
             mut stage: ResMut<Stage>,
             mut commands: Commands) {
    for event in ev_anim.iter() {
        if let AssetEvent::Created { handle } = event {
            let anim = assets.get(handle).unwrap();
            let scaling = commands.spawn_bundle(TransformBundle {
                local: Transform::from_scale(Vec3::new(
                    stage.scaling_factor,
                    stage.scaling_factor,
                    1.0,
                )),
                ..TransformBundle::default()
            }).insert(Scaling).id();
            stage.selected_meta = anim.description
                .get_meta("anim_idle")
                .map(|(k, _)| k)
                .unwrap_or(0);
            stage.last_selected_meta = stage.selected_meta;
            let entity = anim.spawn_on(stage.selected_meta, &mut commands);
            commands.entity(scaling).add_child(entity);
        }
    }
}

fn animation_ui(mut context: ResMut<EguiContext>,
                diagnostics: Res<Diagnostics>,
                animations: Res<Assets<Animation>>,
                mut player: Query<&mut AnimationPlayer>,
                mut stage: ResMut<Stage>) {
    egui::Window::new("Frame Rate").show(context.ctx_mut(), |ui| {
        ui.label(format!("FPS = {:.2}", diagnostics
            .get(FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.average())
            .unwrap_or(f64::NAN)));
    });
    let stage = &mut *stage;
    if let Some(anim) = animations.get(&stage.animation) {
        egui::Window::new("Animation").show(context.ctx_mut(), |ui| {
            stage.last_selected_meta = stage.selected_meta;
            for (k, meta) in anim.description.meta.iter().enumerate() {
                ui.radio_value(&mut stage.selected_meta, k, &meta.name);
            }
        });
    }
    egui::Window::new("Metrics").show(context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Scale:");
            ui.add(Slider::new(&mut stage.scaling_factor, 0.5..=5.0));
        });
        if let Ok(mut player) = player.get_single_mut() {
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.checkbox(&mut player.paused, "paused");
            });
            ui.horizontal(|ui| {
                ui.label("Speed:");
                ui.add(Slider::from_get_set(0.0..=2.0, |val| match val {
                    None => player.speed(),
                    Some(val) => {
                        player.set_speed(val);
                        val
                    }
                }))
            });
            ui.horizontal(|ui| {
                ui.label("Progress:");
                ui.add_enabled(player.paused, Slider::from_get_set(
                    0.0..=100.0, |val| match val {
                        None => player.progress() * 100.0,
                        Some(val) => {
                            player.set_progress(val / 100.0);
                            val
                        }
                    },
                ));
            });
        }
    });
}

fn respond_to_stage_change(stage: Res<Stage>,
                           animations: Res<Assets<Animation>>,
                           mut scaling: Query<&mut Transform, With<Scaling>>,
                           mut player: Query<&mut AnimationPlayer>) {
    if let Ok(mut transform) = scaling.get_single_mut() {
        transform.scale.x = stage.scaling_factor;
        transform.scale.y = stage.scaling_factor;
    }
    if stage.selected_meta != stage.last_selected_meta {
        if let Ok(mut player) = player.get_single_mut() {
            let anim = animations.get(&stage.animation).unwrap();
            let clip = anim.clip_for(stage.selected_meta);
            let mut new_player = AnimationPlayer::new(clip, 1.0, true);
            new_player.set_progress(player.progress());
            new_player.paused = player.paused;
            *player = new_player;
        }
    }
}
