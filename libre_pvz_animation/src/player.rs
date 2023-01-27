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

//! Animation players.

use std::sync::Arc;
use std::time::Duration;
use itertools::Itertools;
use bevy::prelude::*;
use delegate::delegate;
use crate::clip::{AnimationClip, EntityPath};
use crate::curve::{AnyComponent, AnyCurve, CurveBinding, CurveBindingInfo, Segment};
use crate::curve::blend::{BlendInfo, BlendMethod};

/// Playing status of an animation.
#[derive(Debug, Clone)]
pub struct AnimationStatus {
    frame_rate: f32,
    segment: Segment,
    timer: Timer,
}

impl AnimationStatus {
    /// Create a new animation status (initial state).
    pub fn new(frame_rate: f32, segment: Segment, mode: TimerMode) -> Self {
        let len = if let TimerMode::Repeating = mode { segment.len_looping() } else { segment.len() };
        let timer = Timer::new(Duration::from_secs_f32(len as f32 / frame_rate), mode);
        AnimationStatus { frame_rate, segment, timer }
    }

    /// Frame count in one cycle (total frame count if not repeating).
    pub fn frame_count(&self) -> u16 {
        match self.timer.mode() {
            TimerMode::Repeating => self.segment.len_looping(),
            TimerMode::Once => self.segment.len(),
        }
    }

    /// Get the current frame rate of this animation player.
    pub fn frame_rate(&self) -> f32 { self.frame_rate }
    /// Set the frame rate of this animation player.
    pub fn set_frame_rate(&mut self, frame_rate: f32) {
        self.frame_rate = frame_rate;
        self.timer.set_duration(Duration::from_secs_f32(self.frame_count() as f32 / frame_rate));
    }

    delegate! {
        to self.timer {
            /// Returns mode (whether it is repeating) for the timer.
            pub fn mode(&self) -> TimerMode;
            /// Sets whether the animation is repeating or not.
            pub fn set_mode(&mut self, mode: TimerMode);

            /// Is the animation paused?
            pub fn paused(&self) -> bool;
            /// Pause the animation.
            pub fn pause(&mut self);
            /// Resume the animation.
            pub fn unpause(&mut self);

            /// Reset the animation playing status.
            pub fn reset(&mut self);
            /// Animation finished playing?
            pub fn finished(&self) -> bool;
            /// Animation just finished playing after last query?
            pub fn just_finished(&self) -> bool;

            /// Tick the time by several seconds.
            pub fn tick(&mut self, delta: Duration);
            /// Get elapsed time in seconds.
            pub fn elapsed_secs(&self) -> f32;
        }
    }

    /// Progress of this animation (in number of frames).
    pub fn progress(&self) -> f64 {
        self.timer.elapsed().as_secs_f64() * self.frame_rate as f64
    }
    /// Set the progress of this animation (in number of frames).
    pub fn set_progress(&mut self, progress: f64) {
        self.timer.set_elapsed(Duration::from_secs_f64(progress / self.frame_rate as f64));
    }

    /// Set the segment for playing.
    pub fn set_segment(&mut self, segment: Segment) {
        self.segment = segment;
        self.reset();
    }

    fn apply(&self, curve: &dyn AnyCurve, blending: Option<(BlendMethod, f32)>, target: &mut dyn AnyComponent) {
        let frame = self.timer.elapsed_secs() * self.frame_rate;
        if let Err(err) = curve.apply_sampled_any(self.segment, frame, blending, target) {
            warn!("cannot apply sampled curve to target: {err}");
        }
    }
}

#[derive(Debug, Clone)]
struct BlendLayer {
    blending: BlendMethod,
    progress: Timer,
    next: Box<BlendChain>,
}

#[derive(Debug, Clone)]
struct BlendChain {
    status: AnimationStatus,
    blending: Option<BlendLayer>,
}

impl BlendChain {
    fn new(status: AnimationStatus) -> BlendChain { BlendChain { status, blending: None } }
    fn tick(&mut self, delta: Duration) {
        if self.status.timer.paused() { return; }
        self.status.timer.tick(delta);
        if let Some(blending) = &mut self.blending {
            blending.progress.tick(delta);
            if blending.progress.finished() {
                self.blending = None;
            } else {
                blending.next.tick(delta);
            }
        }
    }
    fn apply(&self, curve: &dyn AnyCurve, target: &mut dyn AnyComponent) {
        let mut blending = None;
        if let Some(next) = &self.blending {
            next.next.apply(curve, target);
            blending = Some((next.blending, next.progress.percent()));
        }
        self.status.apply(curve, blending, target);
    }
}

/// Animation player.
#[derive(Component)]
#[allow(missing_debug_implementations)]
pub struct AnimationPlayer {
    blend_chain: BlendChain,
    clip: Arc<AnimationClip>,
}

impl AnimationPlayer {
    /// Create an animation player that plays the specific clip.
    pub fn new(clip: Arc<AnimationClip>, segment: Segment, frame_rate: f32, mode: TimerMode) -> Self {
        let status = AnimationStatus::new(frame_rate, segment, mode);
        AnimationPlayer { blend_chain: BlendChain::new(status), clip }
    }

    /// Start playing the specified animation segment without blending.
    pub fn play(&mut self, frame_rate: f32, segment: Segment, mode: TimerMode) {
        self.play_with_blending(frame_rate, segment, mode, None)
    }
    /// Start playing the specified animation segment with possibly blending information.
    pub fn play_with_blending(
        &mut self, frame_rate: f32,
        segment: Segment, mode: TimerMode,
        blending: Option<BlendInfo>,
    ) {
        let status = AnimationStatus::new(frame_rate, segment, mode);
        match blending {
            None => self.blend_chain = BlendChain::new(status),
            Some(blending) => {
                let tail = std::mem::replace(&mut self.blend_chain, BlendChain::new(status));
                self.blend_chain.blending = Some(BlendLayer {
                    blending: blending.method,
                    progress: Timer::new(blending.duration, TimerMode::Once),
                    next: Box::new(tail),
                });
            }
        }
    }

    /// Return a shared reference to the status of the "main" animation.
    pub fn main_status(&self) -> &AnimationStatus { &self.blend_chain.status }

    /// Return a shared reference to the animation status if there is no blending.
    pub fn single_status(&self) -> Option<&AnimationStatus> {
        match self.blend_chain.blending {
            None => Some(&self.blend_chain.status),
            Some(_) => None,
        }
    }

    /// Return a mutable reference to the animation status if there is no blending.
    pub fn single_status_mut(&mut self) -> Option<&mut AnimationStatus> {
        match self.blend_chain.blending {
            None => Some(&mut self.blend_chain.status),
            Some(_) => None,
        }
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn bind_curve_system(
    mut players: Query<
        (Entity, &AnimationPlayer),
        Added<AnimationPlayer>,
    >,
    children: Query<&Children>,
    names: Query<&Name>,
    mut commands: Commands,
) {
    for (root, player) in players.iter_mut() {
        for (path, start, end) in player.clip.iter() {
            if let Some(entity) = locate(root, path, &children, &names) {
                let curves = &player.clip.curves()[*start as usize..*end as usize];
                for (descriptor, mut curves) in &curves.iter().zip(*start..*end)
                    .group_by(|(c, _)| c.descriptor()) {
                    let (_, start) = curves.next().unwrap();
                    let end = curves.last().map_or(start, |(_, end)| end);
                    descriptor.attach_binding(commands.entity(entity), CurveBindingInfo {
                        player_entity: root,
                        curve_index_start: start,
                        curve_index_end: end + 1,
                    });
                }
            }
        }
    }
}

fn locate(
    root: Entity, path: &EntityPath,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    let mut current = root;
    'name: for name in path.iter() {
        for &child in children.get(current).ok()?.iter() {
            if names.get(child) != Ok(name) { continue; }
            current = child;
            continue 'name;
        }
        return None;
    }
    Some(current)
}

pub(crate) fn tick_animation_system(time: Res<Time>, mut players: Query<&mut AnimationPlayer>) {
    for mut player in players.iter_mut() {
        player.blend_chain.tick(time.delta());
    }
}

pub(crate) fn animate_entities_system<C: Component>(
    mut entities: Query<(&mut C, &CurveBinding<C>)>,
    players: Query<&AnimationPlayer>,
) {
    for (mut target, binding) in entities.iter_mut() {
        let player = players.get(binding.info.player_entity).unwrap();
        let range = binding.info.curve_index_start as usize..binding.info.curve_index_end as usize;
        for curve in &player.clip.curves()[range] {
            player.blend_chain.apply(curve.as_ref(), &mut target);
        }
    }
}
