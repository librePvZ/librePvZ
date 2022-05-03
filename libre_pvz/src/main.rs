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
use bevy::render::texture::DEFAULT_IMAGE_HANDLE;
use bevy::sprite::Anchor;
use libre_pvz::resources::sprite::{Transform, Element};
use libre_pvz::resources::bevy::{Animation, AnimationLoader};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_asset::<Animation>()
        .init_asset_loader::<AnimationLoader>()
        .add_startup_system(setup_camera)
        .add_startup_system(load_anim)
        .add_system(init_anim)
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
            let parent = commands.spawn_bundle(TransformBundle {
                local: bevy::prelude::Transform {
                    scale: Vec3::new(3.0, 3.0, 3.0),
                    translation: Vec3::new(-120.0, 120.0, 0.0),
                    ..Default::default()
                },
                ..TransformBundle::default()
            }).id();
            let meta_anim_idle = anim.description.get_meta("anim_idle").unwrap();
            for (track_id, track) in anim.description.tracks.iter().enumerate() {
                let frame = &track.frames[meta_anim_idle.start_frame as usize];
                let mut bundle = SpriteBundle::default();
                bundle.sprite.anchor = Anchor::TopLeft;
                for trans in frame.0.iter() {
                    match trans {
                        Transform::LoadElement(Element::Text { .. }) => todo!(),
                        Transform::LoadElement(Element::Image { image }) =>
                            bundle.texture = anim.images[image].clone(),
                        Transform::Show(visible) => bundle.visibility.is_visible = *visible,
                        Transform::Alpha(alpha) => { bundle.sprite.color.set_a(*alpha); }
                        Transform::Transform(mat) =>
                            bundle.transform = mat.as_transform_with_z(track_id as f32),
                    }
                }
                if bundle.texture.id == DEFAULT_IMAGE_HANDLE.id { continue; }
                let child = commands.spawn_bundle(bundle).id();
                commands.entity(parent).add_child(child);
            }
        }
    }
}
