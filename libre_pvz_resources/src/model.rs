/*
 * librePvZ-resources: resource loading for librePvZ.
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

//! Models incorporating animations.

use std::path::PathBuf;
use bincode::{Encode, Decode};
use serde::{Serialize, Deserialize};

/// Model: animation together with its association.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct Model {
    /// Animation, the all-in-one source.
    pub animation: PathBuf,
    /// State machine for this model. Sorted by name.
    pub states: Box<[State]>,
    /// Attachment models.
    #[serde(default = "defaults::empty_slice", skip_serializing_if = "defaults::is_slice_empty")]
    pub attachments: Box<[Attachment]>,
}

/// State controls the appearance and behaviours.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct State {
    /// Name of this state.
    pub name: String,
    /// These tracks should be hidden in this state.
    #[serde(default = "defaults::empty_slice", skip_serializing_if = "defaults::is_slice_empty")]
    pub hidden_tracks: Box<[String]>,
    /// This state correspond to this meta range in the animation.
    pub state_meta: String,
    /// Transitions leaving this state.
    #[serde(default = "defaults::empty_slice", skip_serializing_if = "defaults::is_slice_empty")]
    pub transitions: Box<[StateTransition]>,
}

/// Transition from one state to another.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct StateTransition {
    /// Triggering condition for this transition. [`None`] means this transition should be
    /// automatically triggered immediately the animation finishes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Trigger>,
    /// Destination for this transition.
    pub dest: String,
    /// Duration in seconds for the blending.
    #[serde(default = "defaults::default_blending")]
    pub blending: f32,
}

/// Triggering condition, tested repeatedly to determine whether to leave a state.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct Trigger {
    /// Predicate to test for this triggering condition.
    pub predicate: String,
    /// Arguments for the predicate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<PathBuf>,
}

/// Attachment, useful for separating different movable parts in a single entity.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct Attachment {
    /// Target track to which this model is attached.
    pub target_track: String,
    /// The model to be attached.
    pub child_model: PathBuf,
}

/// Plant meta information.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct PlantMeta {
    /// Width, number of grids taken by this plant, along the X axis (1 by default).
    #[serde(default = "defaults::one", skip_serializing_if = "defaults::is_one")]
    pub width: u8,
    /// Breadth, number of grids taken by this plant, along the Y axis (1 by default).
    #[serde(default = "defaults::one", skip_serializing_if = "defaults::is_one")]
    pub breadth: u8,
    /// Model of this plant.
    pub model: PathBuf,
}

mod defaults {
    pub fn one() -> u8 { 1 }
    pub fn is_one(x: &u8) -> bool { *x == 1 }
    pub fn empty_slice<T>() -> Box<[T]> { Box::new([]) }
    pub fn is_slice_empty<T>(x: &[T]) -> bool { x.is_empty() }
    pub fn default_blending() -> f32 { 0.2 }
}
