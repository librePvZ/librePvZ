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

use std::time::Duration;
use bevy::prelude::*;
use bevy::core::Stopwatch;
use bevy::sprite::Anchor;
use libre_pvz_animation::curve::blend::{BlendInfo, BlendMethod};
use libre_pvz_animation::curve::Segment;
use libre_pvz_animation::player::AnimationPlayer;
use libre_pvz_resources::animation::Action;
use crate::animation::transform::{SpriteBundle2D, Transform2D, TransformBundle2D};
use crate::core::kinematics::{Position, Velocity};
use crate::core::projectile::{Projectile, VanishingBound};
use crate::resources::bevy::Animation;
use crate::scene::loading::{AssetCollection, AssetLoader, AssetLoaderExt, AssetState, PendingAssets};

const BKG_IMAGE: &str = "background1.jpg";
#[allow(unused)]
const BKG_WIDTH: f32 = 1400.0;
const BKG_HEIGHT: f32 = 600.0;

// ensure 16:9 aspect ratio
const WIDTH: f32 = HEIGHT * 16.0 / 9.0;
const HEIGHT: f32 = BKG_HEIGHT;

const PLANT_SHADOW: &str = "plantshadow.png";
const PROJECTILE_PEA: &str = "ProjectilePea.png";
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
            .insert_resource(VanishingBound {
                top: HEIGHT * 0.75,
                bottom: -HEIGHT * 0.75,
                left: -WIDTH * 0.75,
                right: WIDTH * 0.75,
                maximum_height: HEIGHT * 2.0,
            })
            .attach_loader(AssetLoader::default()
                .with_collection::<LawnAssets>()
                .enable_failure_ui())
            .add_system_set(SystemSet::on_enter(AssetState::AssetReady).with_system(setup_lawn))
            .add_system_set(SystemSet::on_enter(AssetState::AssetReady).with_system(spawn_peashooter_system))
            .add_system_set(SystemSet::on_update(AssetState::AssetReady).with_system(update_grid_system))
            .add_system_set(SystemSet::on_update(AssetState::AssetReady).with_system(peashooter_fire_system));
    }
}

struct LawnAssets {
    lawn_background: Handle<Image>,
    plant_shadow: Handle<Image>,
    projectile_pea: Handle<Image>,
    peashooter_anim: Handle<Animation>,
}

impl AssetCollection for LawnAssets {
    fn load(world: &World) -> (Self, PendingAssets<Self>) {
        let asset_server = world.resource::<AssetServer>();
        let mut pending = PendingAssets::new();
        let assets = LawnAssets {
            lawn_background: pending.load_from(asset_server, BKG_IMAGE),
            plant_shadow: pending.load_from(asset_server, PLANT_SHADOW),
            projectile_pea: pending.load_from(asset_server, PROJECTILE_PEA),
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

#[allow(unused)]
const GRID_X_COUNT: u8 = 9;
#[allow(unused)]
const GRID_Y_COUNT: u8 = 5;

const GRID_SIZE: f32 = 80.0;

fn setup_lawn(assets: Res<LawnAssets>, mut commands: Commands) {
    commands.spawn_bundle(SpriteBundle2D {
        sprite: Sprite {
            anchor: Anchor::TopLeft,
            ..Sprite::default()
        },
        texture: assets.lawn_background.clone(),
        transform: Transform2D::from_translation(Vec2::new(-WIDTH / 2.0, HEIGHT / 2.0)),
        ..SpriteBundle2D::default()
    });
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

#[derive(Copy, Clone, Component)]
struct PeashooterHead;

#[derive(Component)]
struct PeashooterStatus {
    phase: PeashooterPhase,
    stopwatch: Stopwatch,
    just_finished: bool,
}

impl PeashooterStatus {
    fn new(phase: PeashooterPhase) -> Self {
        PeashooterStatus {
            phase,
            stopwatch: Stopwatch::new(),
            just_finished: false,
        }
    }
    fn tick(&mut self, delta: Duration) -> &mut Self {
        self.stopwatch.tick(delta);
        let elapsed = self.stopwatch.elapsed();
        let expected = self.phase.duration();
        self.just_finished = elapsed > expected;
        if self.just_finished {
            self.stopwatch.set_elapsed(elapsed - expected);
            self.phase.goto_next();
        }
        self
    }
}

#[derive(Copy, Clone)]
enum PeashooterPhase {
    Fire1,
    Fire2,
    Rest,
}

impl PeashooterPhase {
    fn goto_next(&mut self) {
        *self = match *self {
            PeashooterPhase::Fire1 => PeashooterPhase::Fire2,
            PeashooterPhase::Fire2 => PeashooterPhase::Rest,
            PeashooterPhase::Rest => PeashooterPhase::Fire1,
        };
    }
    fn duration(self) -> Duration {
        match self {
            PeashooterPhase::Fire1 => Duration::from_secs_f32(0.2),
            PeashooterPhase::Fire2 => Duration::from_secs_f32(0.2),
            PeashooterPhase::Rest => Duration::from_secs_f32(2.0),
        }
    }
    fn is_shooting(self) -> bool { matches!(self, PeashooterPhase::Fire1 | PeashooterPhase::Fire2) }
    fn frame_rate(self) -> f32 {
        match self {
            PeashooterPhase::Fire1 | PeashooterPhase::Fire2 => REPEATER_SHOOTING_FRAME_RATE,
            PeashooterPhase::Rest => 12.0,
        }
    }
}

#[derive(Copy, Clone, Component)]
struct PeashooterStemTop;

const REPEATER_SHOOTING_FRAME_RATE: f32 = 45.0;

const PEA_VELOCITY: f32 = 9.9 * 30.0;

fn spawn_peashooter_system(
    assets: Res<LawnAssets>,
    animations: Res<Assets<Animation>>,
    mut commands: Commands,
) {
    let anim = animations.get(&assets.peashooter_anim).unwrap();
    // the whole plant
    let mut trans = TransformBundle2D::default();
    trans.local.z_order = 10.0;
    let plant = commands.spawn_bundle(trans).insert(GridPos { x: 2, y: 2 }).id();
    // the shadow
    let mut shadow = SpriteBundle2D::default();
    shadow.transform.translation.y = -30.0;
    shadow.transform.z_order = -1.0;
    shadow.texture = assets.plant_shadow.clone();
    let shadow = commands.spawn_bundle(shadow).id();
    commands.entity(plant).add_child(shadow);
    // the stem: anim_idle
    let (_, meta_idle) = anim.description.get_meta("anim_idle").unwrap();
    let (stem, stem_tracks) = anim.spawn_on(&mut commands);
    commands.entity(plant).add_child(stem);
    commands.entity(stem).insert(AnimationPlayer::new(
        anim.clip(), meta_idle.into(),
        anim.description.fps, true,
    ));
    // head should be attached to 'anim_stem'
    let (track_stem, anim_stem) = anim.description.tracks.iter().zip(stem_tracks)
        .find(|(track, _)| track.name == "anim_stem").unwrap();
    // spawn an anchor entity to fix up initial translation of 'anim_stem'
    let mut anchor_trans = SpriteBundle2D::default();
    if let Some(&Action::Translation([tx, ty])) = track_stem
        .frames[meta_idle.start_frame as usize].0.iter()
        .find(|act| matches!(act, Action::Translation(_))) {
        anchor_trans.transform.translation = Vec2::new(40.0 - tx, -40.0 - ty);
    }
    let anchor = commands.spawn_bundle(anchor_trans).insert(PeashooterStemTop).id();
    commands.entity(anim_stem).add_child(anchor);
    // the normal head: anim_head_idle
    let (_, anim_head_idle) = anim.description.get_meta("anim_head_idle").unwrap();
    let (head_idle, _) = anim.spawn_on(&mut commands);
    commands.entity(head_idle)
        .insert(PeashooterHead)
        .insert(PeashooterStatus::new(PeashooterPhase::Rest))
        .insert(AnimationPlayer::new(
            anim.clip(), anim_head_idle.into(),
            anim.description.fps, true,
        ));
    commands.entity(anchor).add_child(head_idle);
}

fn peashooter_fire_system(
    time: Res<Time>,
    mut head: Query<(&Parent, &mut AnimationPlayer, &mut PeashooterStatus), With<PeashooterHead>>,
    stem_top: Query<&GlobalTransform, With<PeashooterStemTop>>,
    lawn_assets: Res<LawnAssets>,
    animations: Res<Assets<Animation>>,
    mut commands: Commands,
) {
    let (parent, mut player, mut status) = head.single_mut();
    if status.tick(time.delta()).just_finished {
        // we want to keep that line aligned with other assignments
        #[allow(clippy::field_reassign_with_default)]
        if status.phase.is_shooting() {
            let stem_top_trans = stem_top.get(parent.0).unwrap().translation();
            let x0 = stem_top_trans.x + 24.0;
            let y0 = stem_top_trans.y + 33.0;
            let p0 = Position(Vec3::new(x0, y0, 0.0));
            let vel = Velocity(Vec3::new(PEA_VELOCITY, 0.0, 0.0));
            let mut bundle = SpriteBundle2D::default();
            bundle.texture = lawn_assets.projectile_pea.clone();
            bundle.sprite.anchor = Anchor::TopLeft;
            bundle.transform.z_order = 100.0;
            commands.spawn_bundle(bundle).insert(p0).insert(vel).insert(Projectile);
        }
        let anim = animations.get(&lawn_assets.peashooter_anim).unwrap();
        let (_, shooting) = anim.description.get_meta("anim_shooting").unwrap();
        let (_, idle) = anim.description.get_meta("anim_head_idle").unwrap();
        let segment = if status.phase.is_shooting() { shooting } else { idle };
        player.play_with_blending(
            status.phase.frame_rate(),
            Segment::from(segment),
            !status.phase.is_shooting(),
            Some(BlendInfo {
                duration: Duration::from_secs_f32(0.2),
                method: BlendMethod::SmoothTanh(1.5),
            }),
        );
    }
}
