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

//! Peashooter.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_asset_loader::prelude::*;
use libre_pvz_animation::transform::SpriteBundle2D;
use libre_pvz_resources::cache_known_states;
use libre_pvz_resources::model::{
    MarkerRegistryExt, Model, ModelState, ModelSystem,
    CoolDown, StateTransitionEvent, TransitionTrigger,
};
use crate::core::kinematics::{Position, Velocity};
use crate::core::projectile::Projectile;
use crate::scene::loading::AssetState;

/// Peashooter plugin.
#[derive(Debug, Default, Copy, Clone)]
pub struct PeashooterPlugin;

impl Plugin for PeashooterPlugin {
    fn build(&self, app: &mut App) {
        app.register_marker::<Peashooter>("Peashooter")
            .register_marker::<PeashooterHead>("PeashooterHead")
            .add_system_set(SystemSet::on_enter(AssetState::AssetReady)
                .with_system(initialize_state_index_system))
            .add_system_set(SystemSet::on_update(AssetState::AssetReady)
                .with_system(peashooter_fire_system))
            .add_system_set(SystemSet::on_update(AssetState::AssetReady)
                .with_system(peashooter_force_shooting_system)
                .before(ModelSystem::TransitionTrigger)
                .after(ModelSystem::CoolDownTicking));
    }
}

/// Assets for peashooters.
#[derive(Debug, AssetCollection, Resource)]
pub struct PeashooterAssets {
    #[asset(path = "Peashooter.model.bin")]
    pub(crate) model: Handle<Model>,
    #[asset(path = "ProjectilePea.png")]
    projectile_pea: Handle<Image>,
}

/// Marker for the full plant of peashooter.
#[derive(Default, Debug, Copy, Clone, Component)]
pub struct Peashooter;

/// Marker for the head of a peashooter.
#[derive(Default, Debug, Copy, Clone, Component)]
pub struct PeashooterHead;

cache_known_states! {
    idle,
    shooting_1,
    shooting_2,
}

const PEA_VELOCITY: f32 = 9.9 * 30.0;

fn initialize_state_index_system(
    assets: Res<PeashooterAssets>,
    models: Res<Assets<Model>>,
    mut commands: Commands,
) {
    let root = models.get(&assets.model).unwrap();
    let head = root.attachments[0].child_model.get(&models).unwrap();
    match StateIndex::cache(&head.states) {
        Ok(indices) => commands.insert_resource(indices),
        Err(err) => error!("state name mismatch: {err}"),
    }
}

/// Force the Peashooters to shoot bullet peas (for debugging purposes).
fn peashooter_force_shooting_system(
    mut head: Query<(Entity, &mut CoolDown, &ModelState), With<PeashooterHead>>,
    mut triggers: EventWriter<TransitionTrigger>,
    models: Res<Assets<Model>>,
    states: Res<StateIndex>,
) {
    for (entity, mut cool_down, state) in &mut head {
        if state.current_state != states.idle { continue; }
        if let Some(trigger) = state.trigger_if_ready(
            entity, &models, &mut cool_down, "shoot") {
            triggers.send(trigger);
        }
    }
}

/// Fire a pea every time we enter the `SHOOTING` state.
fn peashooter_fire_system(
    head: Query<(&ModelState, &GlobalTransform), With<PeashooterHead>>,
    mut transitions: EventReader<StateTransitionEvent>,
    assets: Res<PeashooterAssets>,
    states: Res<StateIndex>,
    mut commands: Commands,
) {
    for &StateTransitionEvent { target_entity, .. } in transitions.iter() {
        let (state, trans) = head.get(target_entity).unwrap();
        // we want to keep that line aligned with other assignments
        #[allow(clippy::field_reassign_with_default)]
        if [states.shooting_1, states.shooting_2].contains(&state.current_state) {
            let trans = trans.translation();
            let x0 = trans.x + (20.0 + 40.0);
            let y0 = trans.y + (30.0 - 40.0);
            let p0 = Position(Vec3::new(x0, y0, 0.0));
            let vel = Velocity(Vec3::new(PEA_VELOCITY, 0.0, 0.0));
            let mut bundle = SpriteBundle2D::default();
            bundle.texture = assets.projectile_pea.clone();
            bundle.sprite.anchor = Anchor::TopLeft;
            bundle.transform.z_order = 100.0;
            commands.spawn((bundle, p0, vel, Projectile));
        }
    }
}
