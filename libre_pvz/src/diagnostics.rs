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

//! Diagnostics support for 2D graphics.

use bevy::prelude::*;
use bevy::render::view::VisibilitySystems;
use bevy::sprite::Anchor;

/// Plugin for displaying bounding boxes for 2D sprite graphics.
#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
pub struct BoundingBoxPlugin;

/// Gizmo group for bounding boxes.
#[derive(Default, Reflect, GizmoConfigGroup)]
#[allow(missing_debug_implementations)]
pub struct BoundingBoxGizmos;

/// Labels for the bounding box systems.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, SystemSet)]
pub enum BoundingBoxSystem {
    /// Set up for newly-[`Added`] [`BoundingBoxRoot`]s.
    AddBoundingBox,
    /// Update bounding boxes upon changes of sprites or textures.
    UpdateBoundingBox,
}

impl Plugin for BoundingBoxPlugin {
    fn build(&self, app: &mut App) {
        use BoundingBoxSystem::*;
        app.init_gizmo_group::<BoundingBoxGizmos>()
            .configure_sets(PostUpdate, (
                AddBoundingBox,
                UpdateBoundingBox,
            ).after(VisibilitySystems::CheckVisibility))
            .add_systems(PostUpdate, add_bounding_box_system.in_set(AddBoundingBox))
            .add_systems(PostUpdate, update_bounding_box_system.in_set(UpdateBoundingBox));
    }
}

/// Component marking the root entity for bounding boxes.
#[derive(Debug, Default, Component)]
pub struct BoundingBoxRoot {
    /// Visibility of all these bounding boxes.
    pub is_visible: bool,
}

/// Bounding box component.
#[derive(Debug, Component)]
pub struct BoundingBox {
    root: Entity,
    size: Vec2,
    anchor: Anchor,
}

fn add_bounding_box_system(
    roots: Query<Entity, Added<BoundingBoxRoot>>,
    children: Query<&Children>,
    sprites: Query<(&Sprite, Option<&Handle<Image>>)>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
) {
    for root in roots.iter() {
        let mut pending = vec![root];
        while let Some(current) = pending.pop() {
            if let Ok(children) = children.get(current) {
                pending.extend(children.iter());
            }
            if let Ok((sprite, texture)) = sprites.get(current) {
                if let Some(size) = sprite_size(&images, sprite, texture) {
                    commands.entity(current).insert(BoundingBox { root, size, anchor: sprite.anchor });
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_bounding_box_system(
    mut gizmos: Gizmos<BoundingBoxGizmos>,
    roots: Query<&BoundingBoxRoot>,
    mut boxes: Query<(
        Entity, &mut BoundingBox,
        &GlobalTransform, &ViewVisibility,
        &Sprite, Option<&Handle<Image>>,
    )>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
) {
    for (this, mut bb, global_transform, visible, sprite, texture) in boxes.iter_mut() {
        if let Ok(root) = roots.get(bb.root) {
            if let Some(size) = sprite_size(&images, sprite, texture) {
                if bb.size != size { bb.size = size; }
            }
            bb.anchor = sprite.anchor;
            if visible.get() && root.is_visible {
                let base = bb.anchor.as_vec();
                let make_vertex = |anchor: Anchor| {
                    let inner_pos = (anchor.as_vec() - base) * bb.size;
                    let pos = global_transform.transform_point(inner_pos.extend(0.0));
                    Vec2::new(pos.x, pos.y)
                };
                let top_left = make_vertex(Anchor::TopLeft);
                let top_right = make_vertex(Anchor::TopRight);
                let bottom_right = make_vertex(Anchor::BottomRight);
                let bottom_left = make_vertex(Anchor::BottomLeft);
                let vertices = [top_left, top_right, bottom_right, bottom_left, top_left];
                gizmos.linestrip_2d(vertices, Color::WHITE);
            }
        } else { // root is no longer BoundingBoxRoot
            commands.entity(this).remove::<BoundingBox>();
        }
    }
}

fn sprite_size(images: &Assets<Image>, sprite: &Sprite, texture: Option<&Handle<Image>>) -> Option<Vec2> {
    sprite.custom_size.or_else(|| texture
        .and_then(|texture| images.get(texture))
        .map(|image| image.size_f32()))
}
