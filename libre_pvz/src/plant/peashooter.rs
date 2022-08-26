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
use libre_pvz_resources::cached::Cached;
use libre_pvz_resources::model::{MarkerRegistryExt, Model, ModelState, StateTransitionEvent};
use crate::core::kinematics::{Position, Velocity};
use crate::core::projectile::Projectile;
use crate::scene::loading::AssetState;

/// Peashooter plugin.
#[derive(Debug, Default, Copy, Clone)]
pub struct PeashooterPlugin;

impl Plugin for PeashooterPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_update(AssetState::AssetReady).with_system(peashooter_fire_system))
            .register_marker::<Peashooter>("Peashooter")
            .register_marker::<PeashooterHead>("PeashooterHead");
    }
}

/// Assets for peashooters.
#[derive(Debug, AssetCollection)]
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

#[allow(unused)]
static IDLE: Cached<&str, usize> = Cached::new("idle");
static SHOOTING: Cached<&str, usize> = Cached::new("shooting");

const PEA_VELOCITY: f32 = 9.9 * 30.0;

/// Fire a pea every time we enter the `SHOOTING` state.
pub fn peashooter_fire_system(
    head: Query<(&ModelState, &GlobalTransform), With<PeashooterHead>>,
    mut transitions: EventReader<StateTransitionEvent>,
    models: Res<Assets<Model>>,
    assets: Res<PeashooterAssets>,
    mut commands: Commands,
) {
    for &StateTransitionEvent { target_entity, .. } in transitions.iter() {
        let (state, trans) = head.get(target_entity).unwrap();
        let shooting = SHOOTING.get_handle_or_lazy_init(||
            &models.get(state.model()).unwrap().states).unwrap();
        // we want to keep that line aligned with other assignments
        #[allow(clippy::field_reassign_with_default)]
        if state.current_state == shooting {
            let trans = trans.translation();
            let x0 = trans.x + 24.0;
            let y0 = trans.y + 33.0;
            let p0 = Position(Vec3::new(x0, y0, 0.0));
            let vel = Velocity(Vec3::new(PEA_VELOCITY, 0.0, 0.0));
            let mut bundle = SpriteBundle2D::default();
            bundle.texture = assets.projectile_pea.clone();
            bundle.sprite.anchor = Anchor::TopLeft;
            bundle.transform.z_order = 100.0;
            commands.spawn_bundle(bundle).insert(p0).insert(vel).insert(Projectile);
        }
    }
}
