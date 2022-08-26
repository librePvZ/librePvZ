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

use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use std::time::Duration;
use anyhow::Context;
use bevy::asset::{AssetPath, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::time::Stopwatch;
use bevy::utils::HashMap;
use bincode::{Encode, Decode};
use serde::{Serialize, Deserialize};
use itertools::Itertools;
use optics::traits::*;
use libre_pvz_animation::curve::blend::{BlendInfo, BlendMethod};
use libre_pvz_animation::player::AnimationPlayer;
use crate::asset_ext;
use crate::animation::{Animation, action::_Translation};
use crate::cached::{Cached, ContainerWithKey, EntryWithKey, SortedSlice};
use crate::loader::{AssetExtensions, TwoStageAsset};

/// Extend the [`App`] for registering marker components.
pub trait MarkerRegistryExt {
    /// Register a marker component in the global registry.
    fn register_marker<M: Component + Default>(&mut self, name: &str) -> &mut Self;
}

impl MarkerRegistryExt for App {
    fn register_marker<M: Component + Default>(&mut self, name: &str) -> &mut App {
        self.world.resource_mut::<MarkerRegistry>().register_marker::<M>(name);
        self
    }
}

/// Model: animation together with its association.
#[derive(Debug, Encode, Decode, Serialize, Deserialize, TypeUuid)]
#[uuid = "42c6a0d1-7add-4ef2-abe7-ca4d38252617"]
pub struct Model {
    /// Animation, the all-in-one source.
    pub animation: Cached<PathBuf, Handle<Animation>>,
    /// Marker components for instances of this model.
    #[serde(default, skip_serializing_if = "defaults::is_slice_empty")]
    pub markers: Box<[String]>,
    /// State machine for this model. Sorted by name.
    pub states: SortedSlice<State>,
    /// Default state, or start-up state.
    pub default_state: Cached<String, usize>,
    /// Attachment models.
    #[serde(default, skip_serializing_if = "defaults::is_slice_empty")]
    pub attachments: SortedSlice<Attachment>,
}

impl TwoStageAsset for Model {
    type Repr = Model;
    const EXTENSIONS: AssetExtensions = asset_ext!("model");
    fn post_process(repr: Model, load_context: &mut LoadContext) -> anyhow::Result<(Self, Vec<AssetPath<'static>>)> {
        let mut dep_paths = Vec::new();
        repr.animation.init_handle(load_context);
        dep_paths.push(AssetPath::from(repr.animation.raw_key.as_path()).to_owned());
        for attachment in repr.attachments.iter() {
            attachment.child_model.init_handle(load_context);
            dep_paths.push(AssetPath::from(attachment.child_model.raw_key.as_path()).to_owned());
        }
        Ok((repr, dep_paths))
    }
}

/// State controls the appearance and behaviours.
#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct State {
    /// Name of this state.
    pub name: String,
    /// Override the frame rate in this state. [`None`] for using the value in the animation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<f32>,
    /// Cool down time before any state transition can happen.
    #[serde(default, skip_serializing_if = "defaults::is_zero_duration", with = "duration_from_secs")]
    pub cool_down: Duration,
    /// These tracks should be hidden in this state.
    #[serde(default, skip_serializing_if = "defaults::is_slice_empty")]
    pub hidden_tracks: Box<[String]>,
    /// This state correspond to this meta range in the animation.
    pub state_meta: Cached<String, usize>,
    /// Transitions leaving this state.
    #[serde(default, skip_serializing_if = "defaults::is_slice_empty")]
    pub transitions: SortedSlice<StateTransition>,
}

impl EntryWithKey for State {
    type Key = str;
    fn key(&self) -> &str { &self.name }
}

/// Transition from one state to another.
#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct StateTransition {
    /// Triggering condition for this transition. [`None`] means this transition should be
    /// automatically triggered immediately the cool-down completes (or when the animation finishes
    /// if [`cool_down`] is set to zero). See also [`cool_down`].
    ///
    /// [`cool_down`]: StateTransition::cool_down
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    /// Cool down time before this state transition can happen. If `cool_down` is zero, the
    /// transition happens whenever the [`trigger`] is received. If additionally [`trigger`] is set
    /// to [`None`], we wait until the animation finishes playing.
    ///
    /// This value overrides the global value [`State::cool_down`].
    ///
    /// [`trigger`]: StateTransition::trigger
    #[serde(default, skip_serializing_if = "defaults::is_zero_duration", with = "duration_from_secs")]
    pub cool_down: Duration,
    /// Destination for this transition.
    pub dest: Cached<String, usize>,
    /// Duration in seconds for the blending.
    #[serde(default = "defaults::default_blending")]
    pub blending: Duration,
}

impl EntryWithKey for StateTransition {
    type Key = Option<String>;
    fn key(&self) -> &Option<String> { &self.trigger }
}

/// Attachment, useful for separating different movable parts in a single entity.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct Attachment {
    /// Target track to which this model is attached.
    pub target_track: String,
    /// The model to be attached.
    pub child_model: Cached<PathBuf, Handle<Model>>,
}

impl EntryWithKey for Attachment {
    type Key = str;
    fn key(&self) -> &str { &self.target_track }
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
    pub model: Cached<PathBuf, Handle<Model>>,
}

mod defaults {
    use std::time::Duration;
    pub const fn one() -> u8 { 1 }
    pub const fn is_one(x: &u8) -> bool { *x == 1 }
    pub const fn is_slice_empty<T>(x: &[T]) -> bool { x.is_empty() }
    pub const fn default_blending() -> Duration { Duration::from_millis(200) }
    pub const fn is_zero_duration(duration: &Duration) -> bool { duration.is_zero() }
}

mod duration_from_secs {
    use std::time::Duration;
    use serde::{Serializer, Deserializer, Serialize, Deserialize};

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        duration.as_secs_f32().serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        Ok(Duration::from_secs_f32(f32::deserialize(deserializer)?))
    }
}

/// Registry for marker components.
#[derive(Default, Clone)]
pub struct MarkerRegistry {
    entries: HashMap<Box<str>, fn(Entity, &mut Commands)>,
}

impl Debug for MarkerRegistry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        struct List<I>(I);
        impl<I: Iterator + Clone> Debug for List<I>
            where I::Item: Debug {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.debug_list().entries(self.0.clone()).finish()
            }
        }

        f.debug_struct("MarkerRegistry")
            .field("entries", &List(self.entries.keys()))
            .finish()
    }
}

impl MarkerRegistry {
    /// Register a marker component in this registry.
    pub fn register_marker<M: Component + Default>(&mut self, name: &str) {
        fn insert_marker<M: Component + Default>(entity: Entity, commands: &mut Commands) {
            commands.entity(entity).insert(M::default());
        }
        let old = self.entries.insert(name.into(), insert_marker::<M>);
        if old.is_some() { error!("overwriting a marker with name '{name}'"); }
    }

    /// Attach the marker with the given name to the specified target entity.
    pub fn attach_marker(&self, name: &str, target: Entity, commands: &mut Commands) {
        match self.entries.get(name).copied() {
            Some(attach) => attach(target, commands),
            None => error!("model references non-existent marker '{name}'"),
        }
    }
}

/// Cool down component for state transitions.
#[derive(Debug, Default, Clone, Component)]
pub struct CoolDown {
    /// The stopwatch for cool down logic.
    stopwatch: Stopwatch,
}

impl CoolDown {
    /// Are we already cooled down enough for this action? If tested positive, the respective
    /// cool down time is depleted from this cool down timer.
    ///
    /// **Note:** tests for readiness should be performed in priority order, because cool down time
    /// is accumulated automatically in the background, and automatically depleted by this method
    /// when the test succeeds.
    pub fn ready_for(&mut self, duration: Duration) -> bool {
        let ready = self.stopwatch.elapsed() > duration;
        if ready { self.stopwatch.set_elapsed(self.stopwatch.elapsed() - duration); }
        ready
    }
}

/// Tick the cool down timer.
pub fn cool_down_tick_system(mut cool_down: Query<&mut CoolDown>, time: Res<Time>) {
    for mut cool_down in &mut cool_down {
        cool_down.stopwatch.tick(time.delta());
    }
}

/// Keep track of the current state of the model.
#[derive(Debug, Clone, Component)]
pub struct ModelState {
    model: Handle<Model>,
    /// Current state index into [`Model::states`].
    pub current_state: usize,
}

impl ModelState {
    /// Get the parent model of this model state.
    #[inline(always)]
    pub fn model(&self) -> &Handle<Model> { &self.model }
}

/// Request to trigger a [`ModelState`] transition.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TransitionTrigger {
    /// The trigger will try to take effect on this [`Entity`].
    pub target_entity: Entity,
    /// The trigger, as specified in [`StateTransition::trigger`].
    pub trigger: Option<String>,
    /// Normally, if the current state does not recognize the `trigger`, an error will be emitted
    /// as log message. If `permissive` is `true`, this will not be treated as an error.
    pub permissive: bool,
}

struct PrettyTrigger<'a>(Option<&'a str>);

impl<'a> Display for PrettyTrigger<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            None => f.write_str("null"),
            Some(t) => write!(f, "'{t}'"),
        }
    }
}

/// Respond to [`TransitionTrigger`]s by performing state transitions.
pub fn transition_trigger_response_system(
    mut instances: Query<&mut ModelState>,
    mut triggers: EventReader<TransitionTrigger>,
    mut transition_events: EventWriter<StateTransitionEvent>,
    models: Res<Assets<Model>>,
) {
    for trigger in triggers.iter() {
        let mut state = instances.get_mut(trigger.target_entity).unwrap();
        let model = models.get(&state.model).unwrap();
        let current_state = &model.states[state.current_state];
        if let Some(trans) = current_state.transitions.get_by_key(&trigger.trigger) {
            transition_events.send(StateTransitionEvent {
                target_entity: trigger.target_entity,
                previous_state: state.current_state,
                transition_index: trans,
            });
            // NOTE: event acts as a synchronization point, but it is okay here
            // because we are actually claiming unique access to `ModelState`
            // therefore everyone should still only observe consistent states
            let trans = &current_state.transitions[trans];
            state.current_state = trans.dest.get_handle_or_init(&model.states).unwrap();
        } else if !trigger.permissive {
            // did not find the trigger, report the error
            let trigger = PrettyTrigger(trigger.trigger.as_deref());
            let expected = current_state.transitions.iter()
                .map(|t| PrettyTrigger(t.trigger.as_deref()));
            error!("unknown trigger {trigger}, expecting any of [{}]", expected.format(","));
        }
    }
}

/// Automatically apply the [`None`] trigger for the respective model.
#[derive(Default, Debug, Copy, Clone, Component)]
pub struct AutoNullTrigger;

/// Automatically apply the [`None`] trigger if required.
pub fn apply_null_trigger_system(
    instances: Query<(Entity, &ModelState), With<AutoNullTrigger>>,
    mut triggers: EventWriter<TransitionTrigger>,
    models: Res<Assets<Model>>,
) {
    for (instance, state) in &instances {
        let model = models.get(&state.model).unwrap();
        let current_state = &model.states[state.current_state];
        // if a `null` transition exists, it must be the first (sorted)
        if let Some(t) = current_state.transitions.first() {
            if t.trigger.is_none() {
                triggers.send(TransitionTrigger {
                    target_entity: instance,
                    trigger: None,
                    permissive: false,
                });
            }
        }
    }
}

/// [`ModelState`] transition events.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct StateTransitionEvent {
    /// The entity on which this state transition happened.
    pub target_entity: Entity,
    /// Previous state index into [`Model::states`].
    pub previous_state: usize,
    /// Index of the transition it took to reach the current state.
    pub transition_index: usize,
}

/// Reflect the state transition to animation transition.
pub fn state_transition_animation_system(
    mut instances: Query<(&ModelState, &mut AnimationPlayer)>,
    mut events: EventReader<StateTransitionEvent>,
    models: Res<Assets<Model>>,
    animations: Res<Assets<Animation>>,
) {
    for trans in events.iter() {
        let (state, mut player) = instances.get_mut(trans.target_entity).unwrap();
        let model = models.get(&state.model).unwrap();
        let previous_state = &model.states[trans.previous_state];
        let transition = &previous_state.transitions[trans.transition_index];
        assert_eq!(transition.dest.cached.get().copied().unwrap(), state.current_state);
        let current_state = &model.states[state.current_state];
        let anim = model.animation.get(&animations).unwrap();
        let frame_rate = current_state.frame_rate.unwrap_or(anim.description.fps);
        let segment = current_state.state_meta.get_or_init(&anim.description.meta).unwrap().into();
        let blending = (!transition.blending.is_zero()).then_some(BlendInfo {
            method: BlendMethod::SmoothTanh(1.5),
            duration: transition.blending,
        });
        player.play_with_blending(frame_rate, segment, true, blending);
    }
}

impl Model {
    /// Spawn an instance of this model using the given command queue.
    pub fn spawn(model: Handle<Model>, translation: Vec2,
                 animations: &Assets<Animation>, models: &Assets<Model>,
                 markers: &MarkerRegistry, commands: &mut Commands) -> anyhow::Result<Entity> {
        let this = models.get(&model).unwrap();
        let anim = this.animation.get(animations).unwrap();
        // init ModelState, locate the Meta
        let current_state = this.default_state.get_handle_or_init(&this.states)
            .context(format!("non-existent state '{}' set as default state", this.default_state.raw_key))?;
        let state = &this.states[current_state];
        let meta = state.state_meta.get_or_init(&anim.description.meta)
            .context(format!("non-existent meta '{}' associated to state '{}'",
                             state.state_meta.raw_key, state.name))?;
        // spawn the main model as an entity, locate target tracks for the attachments
        let mut targets = vec![None; this.attachments.len()];
        let main = anim.spawn_on(commands, translation, |n, name, entity| {
            if let Some(k) = this.attachments.get_by_key(name) {
                info!("visiting attachment '{name}' ...");
                assert!(targets[k].is_none(), "duplicated track");
                targets[k] = Some((n, entity));
            }
        });
        // attach ModelStatus & AnimationPlayer
        commands.entity(main)
            .insert(ModelState { model, current_state })
            .insert(AnimationPlayer::new(
                anim.clip(), meta.into(),
                anim.description.fps, true,
            ));
        // attach marker component
        for marker in this.markers.iter() {
            markers.attach_marker(marker, main, commands);
        }
        // attach cool down component (if deemed useful)
        if this.states.len() > 1 || !state.transitions.is_empty() {
            commands.entity(main).insert(CoolDown::default());
        }
        // spawn and attach attachments to the target tracks
        for (target, attachment) in std::iter::zip(targets, this.attachments.iter()) {
            let (k, target) = if let Some(target) = target { target } else {
                error!("trying to attach to non-existent track '{}'", attachment.target_track);
                continue;
            };
            let child = attachment.child_model.cached.get().unwrap().clone();
            let translation = anim
                .description.tracks[k]
                .frames[meta.start_frame as usize].0.iter()
                .find_map(|act| _Translation.preview_ref(act).ok().copied())
                .map_or(Vec2::ZERO, |[tx, ty]| Vec2::new(-tx, -ty));
            info!("translation = {translation:?}");
            let child = Model::spawn(child, translation, animations, models, markers, commands);
            match child {
                Ok(child) => { commands.entity(target).add_child(child); }
                Err(err) => error!(
                    "attachment '{}' failed to spawn: {err}",
                    attachment.child_model.raw_key.display()
                ),
            }
        }
        Ok(main)
    }
}
