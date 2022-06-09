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
use crate::clip::{EntityPath, AnimationClip};
use crate::curve::{CurveBinding, CurveBindingInfo, Segment};

/// Animation player.
#[derive(Component)]
#[allow(missing_debug_implementations)]
pub struct AnimationPlayer {
    frame_rate: f32,
    current_segment: Segment,
    timer: Timer,
    clip: Arc<AnimationClip>,
}

impl AnimationPlayer {
    /// Create an animation player that plays the specific clip.
    pub fn new(clip: Arc<AnimationClip>, segment: Segment, frame_rate: f32, repeat: bool) -> Self {
        let len = if repeat { segment.len_looping() } else { segment.len() };
        let timer = Timer::new(Duration::from_secs_f32(len as f32 / frame_rate), repeat);
        AnimationPlayer { frame_rate, current_segment: segment, timer, clip }
    }

    /// Frame count in one cycle (total frame count if not repeating).
    pub fn frame_count(&self) -> u16 {
        if self.timer.repeating() {
            self.current_segment.len_looping()
        } else {
            self.current_segment.len()
        }
    }

    /// Get the current frame rate of this animation player.
    pub fn frame_rate(&self) -> f32 { self.frame_rate }
    /// Set the frame rate of this animation player.
    pub fn set_frame_rate(&mut self, frame_rate: f32) {
        self.frame_rate = frame_rate;
        self.timer.set_duration(Duration::from_secs_f32(self.frame_count() as f32 / frame_rate));
    }

    /// Returns `true` if the timer is repeating.
    pub fn repeating(&self) -> bool { self.timer.repeating() }
    /// Sets whether the animation is repeating or not.
    pub fn set_repeating(&mut self, repeating: bool) { self.timer.set_repeating(repeating) }

    /// Is the animation paused?
    pub fn paused(&self) -> bool { self.timer.paused() }
    /// Pause the animation.
    pub fn pause(&mut self) { self.timer.pause() }
    /// Resume the animation.
    pub fn resume(&mut self) { self.timer.unpause() }

    /// Reset the animation playing status.
    pub fn reset(&mut self) { self.timer.reset() }
    /// Animation finished playing?
    pub fn finished(&self) -> bool { self.timer.finished() }
    /// Animation just finished playing after last query?
    pub fn just_finished(&self) -> bool { self.timer.just_finished() }

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
        self.current_segment = segment;
        self.reset();
    }

    /// Tick the time by several seconds.
    pub fn tick(&mut self, delta: Duration) { self.timer.tick(delta); }

    /// Get elapsed time in seconds.
    pub fn elapsed(&self) -> f32 { self.timer.elapsed_secs() }
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
        player.tick(time.delta());
    }
}

pub(crate) fn animate_entities_system<C: Component>(
    mut entities: Query<(&mut C, &CurveBinding<C>)>,
    players: Query<&AnimationPlayer>,
) {
    for (mut target, binding) in entities.iter_mut() {
        let player = players.get(binding.info.player_entity).unwrap();
        let frame = player.timer.elapsed_secs() * player.frame_rate;
        let range = binding.info.curve_index_start as usize..binding.info.curve_index_end as usize;
        for curve in &player.clip.curves()[range] {
            if let Err(err) = curve.apply_sampled_any(player.current_segment, frame, &mut target) {
                warn!("cannot apply sampled curve to target: {err}");
            }
        }
    }
}
