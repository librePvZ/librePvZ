/*
 * librePvZ-animation: animation playing for librePvZ.
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

//! Dedicated 2D transformation.

use bevy::prelude::*;
use bevy::math::Affine3A;
use bevy::render::texture::DEFAULT_IMAGE_HANDLE;
use derivative::Derivative;

/// 2D transformation.
#[derive(Component, Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(Component, PartialEq)]
pub struct Transform2D {
    /// Translation.
    pub translation: Vec2,
    /// Rotation of both axes.
    pub rotation: Vec2,
    /// Relative z-order.
    pub z_order: f32,
    /// Scaling along x and y-axes.
    pub scale: Vec2,
}

impl Transform2D {
    /// Identity 2D transform.
    pub const IDENTITY: Transform2D = Transform2D {
        translation: Vec2::ZERO,
        rotation: Vec2::ZERO,
        z_order: 0.0,
        scale: Vec2::ONE,
    };

    /// Convert to an affine transformation for use in [`bevy`].
    pub fn to_affine(&self) -> Affine3A { self.into() }

    /// Creates a new [`Transform`], with `translation`. Rotation will be 0 and scale 1 on all axes.
    #[inline]
    pub const fn from_translation(translation: Vec2) -> Transform2D {
        Transform2D { translation, ..Self::IDENTITY }
    }

    /// Creates a new [`Transform`], with `scale`. Translation will be 0 and rotation 0 on all axes.
    #[inline]
    pub const fn from_scale(scale: Vec2) -> Transform2D {
        Transform2D { scale, ..Self::IDENTITY }
    }
}

impl Default for Transform2D {
    fn default() -> Transform2D { Transform2D::IDENTITY }
}

impl From<&Transform2D> for Affine3A {
    fn from(t: &Transform2D) -> Affine3A {
        let trans = Vec3::new(t.translation.x, t.translation.y, t.z_order);
        let mat = Mat3::from_cols_array_2d(&[
            [t.scale.x * t.rotation.x.cos(), t.scale.x * t.rotation.x.sin(), 0.0],
            [t.scale.y * t.rotation.y.sin(), t.scale.y * t.rotation.y.cos(), 0.0],
            [0.0, 0.0, 1.0],
        ]);
        Affine3A::from_mat3_translation(mat, trans)
    }
}

/// Similar to [`SpriteBundle`], but with a full-fledged [`Transform2D`].
#[derive(Derivative, Bundle, Debug, Clone)]
#[derivative(Default)]
pub struct SpriteBundle2D {
    /// Sprite information.
    pub sprite: Sprite,
    /// Local transform (relative to parent).
    pub transform: Transform2D,
    /// Global transform (relative to the stage), for rendering.
    pub global_transform: GlobalTransform,
    /// Texture image for this entity.
    #[derivative(Default(value = "DEFAULT_IMAGE_HANDLE.typed()"))]
    pub texture: Handle<Image>,
    /// User indication of whether an entity is visible.
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible.
    pub computed_visibility: ComputedVisibility,
}

/// Similar to [`TransformBundle`], but with a full-fledged [`Transform2D`].
#[derive(Bundle, Clone, Debug, Default)]
pub struct SpatialBundle2D {
    /// The transform of the entity.
    pub local: Transform2D,
    /// The global transform of the entity.
    pub global: GlobalTransform,
    /// User indication of whether an entity is visible.
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible.
    pub computed_visibility: ComputedVisibility,
}

/// Update [`GlobalTransform`] component of entities based on entity hierarchy and
/// [`Transform2D`] component.
#[allow(clippy::type_complexity)]
pub fn transform_propagate_system(
    mut root_query: Query<
        (
            Option<&Children>,
            &Transform2D,
            Changed<Transform2D>,
            &mut GlobalTransform,
        ),
        Without<Parent>,
    >,
    mut transform_query: Query<
        (&mut GlobalTransform, &Transform2D, Changed<Transform2D>),
        With<Parent>,
    >,
    children_query: Query<Option<&Children>, (With<Parent>, With<GlobalTransform>)>,
) {
    for (children, transform, changed, mut global_transform) in root_query.iter_mut() {
        if changed {
            *global_transform = GlobalTransform::from(transform.to_affine());
        }

        if let Some(children) = children {
            for child in children.iter() {
                propagate_recursive(
                    &global_transform,
                    &mut transform_query,
                    &children_query,
                    *child,
                    changed,
                );
            }
        }
    }
}

fn propagate_recursive(
    parent: &GlobalTransform,
    transform_query: &mut Query<
        (&mut GlobalTransform, &Transform2D, Changed<Transform2D>),
        With<Parent>,
    >,
    children_query: &Query<Option<&Children>, (With<Parent>, With<GlobalTransform>)>,
    entity: Entity,
    mut changed: bool,
) {
    let global_matrix = match transform_query.get_mut(entity) {
        Ok((mut global_transform, transform, transform_changed)) => {
            changed |= transform_changed;
            if changed {
                let t = parent.affine() * transform.to_affine();
                *global_transform = GlobalTransform::from(t);
            }
            *global_transform
        }
        _ => { return; }
    };

    if let Ok(Some(children)) = children_query.get(entity) {
        for child in children.iter() {
            propagate_recursive(
                &global_matrix,
                transform_query,
                children_query,
                *child,
                changed,
            );
        }
    }
}
