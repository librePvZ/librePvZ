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

//! Types that may be used in curves.

use bevy::prelude::*;
use bevy::asset::Asset;

/// Animatable types can be interpolated with `f32`s.
pub trait Animatable {
    /// Typically `a * (1 - time) + b * time`.
    fn interpolate(a: &Self, b: &Self, time: f32) -> Self;
}

impl Animatable for bool {
    fn interpolate(a: &bool, _b: &bool, _time: f32) -> bool { *a }
}

impl Animatable for Visibility {
    fn interpolate(a: &Visibility, _b: &Visibility, _time: f32) -> Visibility { *a }
}

impl Animatable for f32 {
    fn interpolate(a: &f32, b: &f32, time: f32) -> f32 {
        a * (1_f32 - time) + b * time
    }
}

impl Animatable for Vec2 {
    fn interpolate(a: &Vec2, b: &Vec2, time: f32) -> Vec2 {
        Vec2::lerp(*a, *b, time)
    }
}

impl Animatable for Vec3 {
    fn interpolate(a: &Vec3, b: &Vec3, time: f32) -> Vec3 {
        Vec3::lerp(*a, *b, time)
    }
}

impl Animatable for Quat {
    fn interpolate(a: &Quat, b: &Quat, time: f32) -> Quat {
        Quat::slerp(*a, *b, time)
    }
}

impl<T: Asset> Animatable for Handle<T> {
    fn interpolate(a: &Handle<T>, _b: &Handle<T>, _time: f32) -> Handle<T> { a.clone() }
}
