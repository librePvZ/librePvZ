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

//! An (over-)simplified almanac scene.

use bevy::prelude::*;
use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::sprite::Anchor;
use bevy_egui::EguiContext;
use egui::{Align2, ComboBox, Frame, Grid, Slider, Ui, Visuals};
use libre_pvz_animation::curve::Segment;
use libre_pvz_animation::transform::TransformBundle2D;
use crate::animation::player::AnimationPlayer;
use crate::animation::transform::{SpriteBundle2D, Transform2D};
use crate::resources::bevy::Animation;
use crate::diagnostics::BoundingBoxRoot;
use crate::scene::loading::{AssetCollection, AssetLoader, AssetLoaderExt, AssetState, PendingAssets};

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

/// Plugin for almanac scene.
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

impl Plugin for AlmanacPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(self.0.clone())
            .attach_loader(AssetLoader::default()
                .with_collection::<Stage>()
                .enable_failure_ui())
            .add_system_set(SystemSet::on_enter(AssetState::AssetReady).with_system(init_anim))
            .add_system_set(SystemSet::on_update(AssetState::AssetReady).with_system(animation_ui))
            .add_system_set(SystemSet::on_update(AssetState::AssetReady).with_system(respond_to_stage_change));
    }
}

struct Stage {
    animation: Handle<Animation>,
    almanac_background: Handle<Image>,
    ground_background: Handle<Image>,
    scaling_factor: f32,
    show_bounding_box: bool,
    selected_meta: usize,
    last_selected_meta: usize,
}

impl AssetCollection for Stage {
    fn load(world: &World) -> (Self, PendingAssets<Self>) {
        let anim_name = world.resource::<AnimName>();
        let asset_server = world.resource::<AssetServer>();
        let mut pending = PendingAssets::new();
        (Stage {
            animation: pending.load_from(asset_server, anim_name.0.as_ref()),
            almanac_background: pending.load_from(asset_server, ALMANAC_IMAGE),
            ground_background: pending.load_from(asset_server, GROUND_IMAGE),
            scaling_factor: 1.5,
            show_bounding_box: false,
            selected_meta: 0,
            last_selected_meta: 0,
        }, pending)
    }
    fn track_dep(&self, handle: HandleUntyped, world: &World, pending: &mut PendingAssets<Self>) {
        if handle.id == self.animation.id {
            let anim = world.resource::<Assets<Animation>>().get(handle).unwrap();
            for (path, image) in &anim.images {
                pending.track(path, image.clone());
            }
        }
    }
}

#[derive(Component)]
struct Scaling;

fn init_anim(
    assets: Res<Assets<Animation>>,
    mut stage: ResMut<Stage>,
    mut context: ResMut<EguiContext>,
    mut commands: Commands,
) {
    let almanac = commands.spawn_bundle(SpriteBundle2D {
        texture: stage.almanac_background.clone(),
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
        texture: stage.ground_background.clone(),
        transform: Transform2D {
            z_order: -1.0,
            translation: Vec2::from(GROUND_CENTER),
            ..Transform2D::default()
        },
        ..SpriteBundle2D::default()
    }).id();
    commands.entity(almanac).add_child(ground);

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
    commands.entity(almanac).add_child(scaling);
    stage.selected_meta = anim.description
        .get_meta("anim_idle")
        .map(|(k, _)| k)
        .unwrap_or(0);
    stage.last_selected_meta = stage.selected_meta;
    let (entity, _) = anim.spawn_on(stage.selected_meta, &mut commands);
    commands.entity(scaling).add_child(entity);
}

fn animation_ui(
    mut context: ResMut<EguiContext>,
    diagnostics: Res<Diagnostics>,
    animations: Res<Assets<Animation>>,
    mut player: Query<&mut AnimationPlayer>,
    mut stage: ResMut<Stage>,
) {
    let anim = animations.get(&stage.animation).unwrap();
    let player = &mut player.get_single_mut().unwrap();
    egui::Window::new("Control Panel")
        .resizable(false)
        .title_bar(false)
        .frame(Frame::none())
        .default_width(DESCRIPTION_WIDTH)
        .anchor(Align2::LEFT_TOP, DESCRIPTION_TOP_LEFT)
        .show(context.ctx_mut(), |ui| {
            Grid::new("metrics")
                .num_columns(2)
                .spacing([15.0, 4.0])
                .show(ui, |ui|
                    metrics_ui(ui, &mut stage, &diagnostics, anim, player));
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
    let mut paused = player.paused();
    ui.checkbox(&mut paused, "paused");
    if paused != player.paused() { if paused { player.pause() } else { player.resume() } }
    ui.end_row();

    ui.label("Frame rate:");
    ui.add(Slider::from_get_set(1.0..=50.0, |val| match val {
        None => player.frame_rate() as f64,
        Some(val) => {
            player.set_frame_rate(val as f32);
            val
        }
    }));
    ui.end_row();

    ui.label("Progress:");
    ui.add_enabled(player.paused(), Slider::from_get_set(
        0.0..=player.frame_count() as f64, |val| match val {
            None => player.progress(),
            Some(val) => {
                player.set_progress(val);
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
        player.set_segment(Segment::from(&anim.description.meta[stage.selected_meta]))
    }
}
