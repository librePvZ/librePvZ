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
use bevy::app::AppExit;
use bevy::asset::LoadState;
use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::ecs::schedule::IntoSystemDescriptor;
use bevy::sprite::Anchor;
use bevy_egui::{EguiContext, EguiPlugin};
use egui::{Align2, ComboBox, Frame, Grid, Slider, Ui, Visuals};
use libre_pvz_animation::transform::TransformBundle2D;
use crate::animation::clip::AnimationPlayer;
use crate::animation::transform::{SpriteBundle2D, Transform2D};
use crate::resources::bevy::Animation;
use crate::diagnostics::{BoundingBoxPlugin, BoundingBoxRoot};

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

/// Plugin for almanac screen.
#[derive(Debug)]
pub struct AlmanacPlugin(AnimName);

#[derive(Debug, Clone)]
struct AnimName(Box<str>);

impl AlmanacPlugin {
    /// Create almanac plugin with specified animation name.
    pub fn new(anim_name: String) -> AlmanacPlugin {
        AlmanacPlugin(AnimName(anim_name.into_boxed_str()))
    }
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum AppState {
    AssetLoading,
    AssetReady,
    LoadFailure,
}

impl AppState {
    fn on_enter<Params>(self, system: impl IntoSystemDescriptor<Params>) -> SystemSet {
        SystemSet::on_enter(self).with_system(system)
    }
    fn on_update<Params>(self, system: impl IntoSystemDescriptor<Params>) -> SystemSet {
        SystemSet::on_update(self).with_system(system)
    }
}

impl Plugin for AlmanacPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_plugin(EguiPlugin)
            .add_plugin(BoundingBoxPlugin)
            .add_startup_system(setup)
            .insert_resource(self.0.clone())
            .add_startup_system(load_anim)
            .add_state(AppState::AssetLoading)
            .add_system_set(AppState::AssetLoading.on_update(wait_for_assets))
            .add_system_set(AppState::AssetReady.on_enter(init_anim))
            .add_system_set(AppState::AssetReady.on_enter(check_failure))
            .add_system_set(AppState::AssetReady.on_update(animation_ui))
            .add_system_set(AppState::AssetReady.on_update(respond_to_stage_change))
            .add_system_set(AppState::LoadFailure.on_update(failure_ui));
    }
}

fn setup(server: Res<AssetServer>, mut commands: Commands) {
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

struct Stage {
    animation: Handle<Animation>,
    scaling_factor: f32,
    show_bounding_box: bool,
    selected_meta: usize,
    last_selected_meta: usize,
}

fn load_anim(server: Res<AssetServer>, anim_name: Res<AnimName>, mut commands: Commands) {
    let animation = server.load(anim_name.0.as_ref());
    commands.insert_resource(Stage {
        animation,
        scaling_factor: 1.5,
        show_bounding_box: false,
        selected_meta: 0,
        last_selected_meta: 0,
    });
}

#[derive(Component)]
struct Scaling;

fn wait_for_assets(
    stage: Res<Stage>, assets: Res<AssetServer>,
    mut state: ResMut<State<AppState>>,
    mut commands: Commands,
) {
    use bevy::asset::LoadState::*;
    match assets.get_load_state(&stage.animation) {
        Loaded => state.set(AppState::AssetReady).unwrap(),
        Failed => {
            commands.spawn_bundle(SpriteBundle2D {
                sprite: Sprite {
                    color: Color::rgba(0.0, 0.0, 0.0, 0.9),
                    ..Sprite::default()
                },
                transform: Transform2D {
                    scale: Vec2::new(WIDTH, HEIGHT),
                    z_order: 200.0,
                    ..Transform2D::default()
                },
                ..SpriteBundle2D::default()
            });
            state.set(AppState::LoadFailure).unwrap();
        }
        _ => {}
    }
}

// init; k * first_k; [0 => {}; 1 => first_k; n => rest(n)]
fn try_first_k_and_rest<T, E, I: IntoIterator>(
    k: usize, iter: I,
    init: impl FnOnce() -> T,
    mut first_k: impl FnMut(&mut T, I::Item) -> Result<(), E>,
    rest: impl FnOnce(&mut T, usize) -> Result<(), E>,
) -> Result<Option<T>, E> {
    assert_ne!(k, 0, "must at least require one element");
    let mut iter = iter.into_iter();
    let first = match iter.next() {
        None => return Ok(None),
        Some(first) => first,
    };
    let mut state = init();
    first_k(&mut state, first)?;
    for x in iter.by_ref().take(k - 1) {
        first_k(&mut state, x)?;
    }
    if let Some(x) = iter.next() {
        let remaining = iter.count() + 1;
        match remaining {
            1 => first_k(&mut state, x)?,
            _ => rest(&mut state, remaining)?,
        }
    }
    Ok(Some(state))
}

struct AssetFailure(String);

fn check_failure(
    stage: Res<Stage>,
    animations: Res<Assets<Animation>>,
    server: Res<AssetServer>,
    mut commands: Commands,
) {
    let anim = animations.get(&stage.animation).unwrap();
    use std::fmt::Write;
    let result = try_first_k_and_rest(
        3, anim.images.iter().filter(|(_, image)|
            server.get_load_state(image.id) == LoadState::Failed),
        || "Failed to load these assets:\n".to_string(),
        |msg, (name, _)| writeln!(msg, "• {name}"),
        |msg, n| writeln!(msg, "... and {n} others"),
    );
    let msg = match result {
        Ok(None) => return,
        Ok(Some(msg)) => msg,
        Err(std::fmt::Error) => "double failure:\n\
            • failed to load some assets\n\
            • cannot show which assets failed".to_string(),
    };
    commands.insert_resource(AssetFailure(msg));
}

fn init_anim(
    assets: Res<Assets<Animation>>,
    mut stage: ResMut<Stage>,
    almanac: Res<Almanac>,
    mut context: ResMut<EguiContext>,
    mut commands: Commands,
) {
    // light theme fits better to the almanac scene
    context.ctx_mut().set_visuals(Visuals::light());
    let anim = assets.get(&stage.animation).unwrap();
    let scaling = commands
        .spawn_bundle(TransformBundle2D {
            local: Transform2D {
                scale: Vec2::new(stage.scaling_factor, stage.scaling_factor),
                translation: Vec2::from(PLANT_CENTER),
                z_order: 1.0,
                ..Transform2D::default()
            },
            ..TransformBundle2D::default()
        })
        .insert(Scaling)
        .insert(BoundingBoxRoot {
            z_order: 100.0,
            is_visible: stage.show_bounding_box,
        })
        .id();
    commands.entity(almanac.0).add_child(scaling);
    stage.selected_meta = anim.description
        .get_meta("anim_idle")
        .map(|(k, _)| k)
        .unwrap_or(0);
    stage.last_selected_meta = stage.selected_meta;
    let entity = anim.spawn_on(stage.selected_meta, &mut commands);
    commands.entity(scaling).add_child(entity);
}

fn animation_ui(
    mut context: ResMut<EguiContext>,
    diagnostics: Res<Diagnostics>,
    animations: Res<Assets<Animation>>,
    asset_failure: Option<Res<AssetFailure>>,
    mut player: Query<&mut AnimationPlayer>,
    mut stage: ResMut<Stage>,
) {
    let anim = animations.get(&stage.animation).unwrap();
    let player = &mut player.get_single_mut().unwrap();
    let sep = if asset_failure.is_some() { 1.0 } else { 4.0 };
    egui::Window::new("Control Panel")
        .resizable(false)
        .title_bar(false)
        .frame(Frame::none())
        .default_width(DESCRIPTION_WIDTH)
        .anchor(Align2::LEFT_TOP, DESCRIPTION_TOP_LEFT)
        .show(context.ctx_mut(), |ui| {
            Grid::new("metrics")
                .num_columns(2)
                .spacing([15.0, sep])
                .show(ui, |ui|
                    metrics_ui(ui, &mut stage, &diagnostics, anim, player));
            if let Some(asset_failure) = asset_failure {
                ui.separator();
                ui.label(&asset_failure.0);
            }
        });
}

fn metrics_ui(
    ui: &mut Ui, stage: &mut Stage,
    diagnostics: &Diagnostics,
    anim: &Animation,
    player: &mut AnimationPlayer,
) {
    ui.label("FPS:");
    ui.label(format!("{:.2}", diagnostics
        .get(FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.average())
        .unwrap_or(f64::NAN)));
    ui.end_row();

    ui.label("Animation:");
    stage.last_selected_meta = stage.selected_meta;
    ComboBox::from_label("(meta)")
        .selected_text(&anim.description.meta[stage.selected_meta].name)
        .show_ui(ui, |ui| for (k, meta) in anim.description.meta.iter().enumerate() {
            ui.selectable_value(&mut stage.selected_meta, k, &meta.name);
        });
    ui.end_row();

    ui.label("Scale:");
    ui.add(Slider::new(&mut stage.scaling_factor, 0.5..=5.0));
    ui.end_row();

    ui.label("Status:");
    ui.checkbox(&mut player.paused, "paused");
    ui.end_row();

    ui.label("Speed:");
    ui.add(Slider::from_get_set(0.0..=2.0, |val| match val {
        None => player.speed(),
        Some(val) => {
            player.set_speed(val);
            val
        }
    }));
    ui.end_row();

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
    ui.end_row();

    ui.label("Bounding:");
    ui.checkbox(&mut stage.show_bounding_box, "show boxes");
}

fn respond_to_stage_change(
    stage: Res<Stage>,
    animations: Res<Assets<Animation>>,
    mut scaling: Query<(&mut Transform2D, &mut BoundingBoxRoot), With<Scaling>>,
    mut player: Query<&mut AnimationPlayer>,
) {
    let (mut transform, mut bb) = scaling.get_single_mut().unwrap();
    if transform.scale.x != stage.scaling_factor {
        transform.scale.x = stage.scaling_factor;
        transform.scale.y = stage.scaling_factor;
    }
    if bb.is_visible != stage.show_bounding_box {
        bb.is_visible = stage.show_bounding_box;
    }

    if stage.selected_meta != stage.last_selected_meta {
        let mut player = player.get_single_mut().unwrap();
        let anim = animations.get(&stage.animation).unwrap();
        let clip = anim.clip_for(stage.selected_meta);
        let mut new_player = AnimationPlayer::new(clip, 1.0, true);
        new_player.set_progress(player.progress());
        new_player.paused = player.paused;
        *player = new_player;
    }
}

fn failure_ui(
    mut context: ResMut<EguiContext>,
    stage: Res<Stage>,
    anim_name: Res<AnimName>,
    server: Res<AssetServer>,
    mut app_exit_events: EventWriter<'_, '_, AppExit>,
) {
    egui::Window::new("Error")
        .default_width(WIDTH / 2.)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(context.ctx_mut(), |ui| {
            if server.get_load_state(&stage.animation) == LoadState::Failed {
                ui.label(format!("Failed to load animation:\n• {}", anim_name.0));
            }
            ui.vertical_centered(|ui| if ui.button("Exit").clicked() {
                app_exit_events.send(AppExit);
            });
        });
}
