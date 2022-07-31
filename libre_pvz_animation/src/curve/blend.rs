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

//! Blending two different animation segments.
//!
//! # References
//! - [_Smooth Transition Function (with fixed start & end points)_][smooth-transition]
//!   on Math.StackExchange
//! - [_Example of a smooth 'step'-function that is constant below 0 and constant above 1_][smooth-step]
//!   on Math.StackExchange
//!
//! [smooth-transition]: https://math.stackexchange.com/questions/3877887/smooth-transition-function-with-fixed-start-end-points
//! [smooth-step]: https://math.stackexchange.com/questions/846743/example-of-a-smooth-step-function-that-is-constant-below-0-and-constant-above

use std::ops::{Add, Mul};
use std::time::Duration;

/// The blend function for use. All functions below transitions from (0, 0) to (1, 1).
///
/// Below is a plot of the three supported blend methods:
/// - Linear: the orange line
/// - Smooth: the green line
/// - SmoothTanh (with <math><mi>α</mi><mo>=</mo><mn>1.5</mn></math>): the pink line
#[doc = include_str!("doc/transitions.svg")]
#[derive(Debug, Copy, Clone)]
pub enum BlendMethod {
    /// A simple linear transition function:
    #[doc = include_str!("doc/linear.mathml")]
    Linear,
    /// A smooth transition function using the exponential function:
    #[doc = include_str!("doc/smooth.mathml")]
    Smooth,
    /// A smooth transition function using the hyperbolic tangent function
    /// (with a parameter <math><mi>α</mi></math>):
    #[doc = include_str!("doc/smooth_tanh.mathml")]
    SmoothTanh(f32),
}

impl BlendMethod {
    /// Factor for blending, with `ratio` being the progress of transition
    /// (0 for departure, 1 for destination).
    ///
    /// All three blending methods are symmetric with respect to (0.5, 0.5):
    /// ```
    /// # use libre_pvz_animation::curve::blend::BlendMethod;
    /// use BlendMethod::*;
    /// for method in [Linear, Smooth, SmoothTanh(1.5), SmoothTanh(2.0)] {
    ///     assert_eq!(method.factor(0.5), 0.5);
    /// }
    /// ```
    pub fn factor(self, ratio: f32) -> f32 {
        if ratio <= 0.0 { return 0.0; }
        if ratio >= 1.0 { return 1.0; }
        match self {
            BlendMethod::Linear => ratio,
            BlendMethod::Smooth => {
                let x = (1.0 - 2.0 * ratio) / (ratio * (1.0 - ratio));
                1.0 / (1.0 + x.exp())
            }
            BlendMethod::SmoothTanh(alpha) => {
                let p = alpha * (2.0 * ratio - 1.0);
                let q = 2.0 * (ratio * (1.0 - ratio)).sqrt();
                0.5 * ((p / q).tanh() + 1.0)
            }
        }
    }

    /// Blend two values using the specified information.
    ///
    /// This function is implemented using multiplication and addition. For vector or matrix types
    /// with dedicated linear interpolation support, such as [`Vec2`], [`Vec3`], [`Mat2`], [`Mat3`],
    /// etc., one should prefer using [`BlendMethod::factor`] to calculate the factor manually, and
    /// use the inherent `lerp` method (e.g. [`Vec2::lerp`]) on those types.
    ///
    /// [`Vec2`]: bevy::math::Vec2
    /// [`Vec3`]: bevy::math::Vec3
    /// [`Mat2`]: bevy::math::Mat2
    /// [`Mat3`]: bevy::math::Mat3
    /// [`Vec2::lerp`]: bevy::math::Vec2::lerp
    pub fn blend<T>(self, start: T, end: T, progress: f32) -> T
        where T: Add<Output=T> + Mul<f32, Output=T> {
        let ratio = self.factor(progress);
        start * (1.0 - ratio) + end * ratio
    }
}

/// Information about blending.
#[derive(Debug, Copy, Clone)]
pub struct BlendInfo {
    /// Blending function to use.
    pub method: BlendMethod,
    /// Duration for transition.
    pub duration: Duration,
}
