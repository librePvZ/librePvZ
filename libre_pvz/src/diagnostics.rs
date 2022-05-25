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
use bevy::sprite::{Anchor, Mesh2dHandle};
use bevy::transform::TransformSystem;
use bevy_prototype_lyon::prelude::*;
use bevy_prototype_lyon::render::Shape;
use libre_pvz_animation::transform::Transform2D;

/// Plugin for displaying bounding boxes for 2D sprite graphics.
#[derive(Debug, Copy, Clone)]
pub struct BoundingBoxPlugin;

impl Plugin for BoundingBoxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ShapePlugin)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                add_bounding_box_system
                    .before(TransformSystem::TransformPropagate))
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_bounding_box_system
                    .before(TransformSystem::TransformPropagate));
    }
}

/// Component marking the root entity for bounding boxes.
#[derive(Debug, Default, Component)]
pub struct BoundingBoxRoot {
    /// Z-order for the bounding boxes added later.
    pub z_order: f32,
    /// Visibility of all these bounding boxes.
    pub is_visible: bool,
}

/// Bounding box component.
#[derive(Debug, Component)]
pub struct BoundingBox(Entity, Vec2);

// home made ShapeBundle to use Transform2D.
#[derive(Bundle)]
struct ShapeBundle2D {
    path: Path,
    mode: DrawMode,
    shape: Shape,
    mesh2d: Mesh2dHandle,
    transform: Transform2D,
    global_transform: GlobalTransform,
    visibility: Visibility,
    computed_visibility: ComputedVisibility,
}

impl ShapeBundle2D {
    fn build(shape: &impl Geometry, mode: DrawMode, transform: Transform2D) -> Self {
        Self {
            path: ShapePath::build_as(shape),
            mode,
            shape: Shape,
            mesh2d: Mesh2dHandle::default(),
            transform,
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            computed_visibility: ComputedVisibility::default(),
        }
    }
}

fn add_bounding_box_system(
    roots: Query<(Entity, &BoundingBoxRoot), Added<BoundingBoxRoot>>,
    children: Query<&Children>,
    sprites: Query<(&Sprite, Option<&Handle<Image>>)>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
) {
    for (root, &BoundingBoxRoot { z_order, is_visible }) in roots.iter() {
        let mut pending = vec![root];
        let white_stroke = DrawMode::Stroke(StrokeMode::new(Color::ANTIQUE_WHITE, 0.5));
        let trans = Transform2D { z_order, ..Transform2D::default() };
        while let Some(current) = pending.pop() {
            if let Ok(children) = children.get(current) {
                pending.extend(children.iter());
            }
            if let Ok((sprite, texture)) = sprites.get(current) {
                if let Some(size) = sprite.custom_size.or_else(|| texture
                    .and_then(|texture| images.get(texture))
                    .map(|image| image.size())) {
                    let bb = rectangle(size, &sprite.anchor);
                    let mut bb = ShapeBundle2D::build(&bb, white_stroke, trans);
                    bb.visibility.is_visible = is_visible;
                    commands.entity(current).with_children(|builder| {
                        builder.spawn_bundle(bb).insert(BoundingBox(root, size));
                    });
                }
            }
        }
    }
}

fn update_bounding_box_system(
    roots: Query<&BoundingBoxRoot>,
    mut boxes: Query<(Entity, &mut BoundingBox, &mut Path, &Parent, &mut Visibility)>,
    sprites: Query<(&Sprite, Option<&Handle<Image>>, Option<&Visibility>), Without<BoundingBox>>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
) {
    for (this, mut bb, mut path, parent, mut vis) in boxes.iter_mut() {
        if let Ok(root) = roots.get(bb.0) {
            if let Ok((sprite, texture, parent_vis)) = sprites.get(parent.0) {
                let new_visible = root.is_visible && parent_vis.map_or(true, |v| v.is_visible);
                if vis.is_visible != new_visible {
                    vis.is_visible = new_visible;
                }
                if let Some(size) = sprite.custom_size.or_else(|| texture
                    .and_then(|texture| images.get(texture))
                    .map(|image| image.size())) {
                    if bb.1 != size {
                        bb.1 = size;
                        *path = ShapePath::build_as(&rectangle(size, &sprite.anchor));
                    }
                }
            }
        } else { // root is no longer BoundingBoxRoot
            commands.entity(this).despawn();
        }
    }
}

fn rectangle(size: Vec2, anchor: &Anchor) -> shapes::Rectangle {
    shapes::Rectangle {
        extents: size,
        origin: match anchor {
            Anchor::Center => RectangleOrigin::Center,
            Anchor::BottomLeft => RectangleOrigin::BottomLeft,
            Anchor::BottomRight => RectangleOrigin::BottomRight,
            Anchor::TopLeft => RectangleOrigin::TopLeft,
            Anchor::TopRight => RectangleOrigin::TopRight,
            anchor => RectangleOrigin::CustomCenter(anchor.as_vec() * size),
        },
    }
}
