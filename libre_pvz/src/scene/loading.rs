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

//! Asset loading logic (including the failure screen).

use std::path::Path;
use bevy::prelude::*;
use bevy::app::AppExit;
use bevy::ecs::schedule::StateData;
use bevy_egui::EguiContext;
use egui::{Align2, WidgetText};

/// Default asset loading states.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AssetState {
    /// State where at least one asset in one asset collection is loading.
    AssetLoading,
    /// All assets in all asset collections have finished loading successfully.
    AssetReady,
    /// At least one asset in one asset collection failed loading.
    LoadFailure,
}

/// Status for asset loading.
/// Only present for use if any asset failed loading.
#[derive(Resource)]
#[allow(missing_debug_implementations)]
pub struct Status<S> {
    failures: Vec<Box<Path>>,
    success_state: S,
}

impl<S> Status<S> {
    /// Get all the assets that failed loading.
    pub fn failures(&self) -> impl Iterator<Item=&Path> {
        self.failures.iter().map(|s| s.as_ref())
    }
    /// Get a failure message for display.
    pub fn failure_message(&self) -> AssetFailure {
        AssetFailure::from_names(4, self.failures.iter()).unwrap()
    }
}

/// Asset loading failure message.
#[derive(Debug, Clone, Resource)]
pub struct AssetFailure(pub String);

// init; k * first_k; [0 => {}; 1 => first_k; n => rest(n)]
fn try_first_k_and_rest<T, E, I: IntoIterator>(
    k: usize, iter: I,
    init: impl FnOnce() -> T,
    mut first_k: impl FnMut(&mut T, I::Item) -> Result<(), E>,
    mut sep: impl FnMut(&mut T) -> Result<(), E>,
    rest: impl FnOnce(&mut T, usize) -> Result<(), E>,
) -> Result<Option<T>, E> {
    assert_ne!(k, 0, "must at least require one element");
    let mut iter = iter.into_iter();
    let first = match iter.next() {
        None => return Ok(None),
        Some(first) => first,
    };
    let mut state = init();
    first_k(&mut state, first)?;
    for x in iter.by_ref().take(k - 1) {
        sep(&mut state)?;
        first_k(&mut state, x)?;
    }
    if let Some(x) = iter.next() {
        let remaining = iter.count() + 1;
        sep(&mut state)?;
        match remaining {
            1 => first_k(&mut state, x)?,
            _ => rest(&mut state, remaining)?,
        }
    }
    Ok(Some(state))
}

impl AssetFailure {
    /// Construct asset loading failure from names of assets failing to load.
    pub fn from_names(n: usize, names: impl IntoIterator<Item=impl AsRef<Path>>) -> Option<AssetFailure> {
        use std::fmt::Write;
        let result = try_first_k_and_rest(
            n, names.into_iter(),
            || "Failed to load these assets:\n".to_string(),
            |msg, name| write!(msg, "• {}", name.as_ref().display()),
            |msg| writeln!(msg),
            |msg, n| write!(msg, "... and {n} others"),
        );
        let msg = match result {
            Ok(None) => return None,
            Ok(Some(msg)) => msg,
            Err(std::fmt::Error) => "double failure:\n\
                • failed to load some assets\n\
                • cannot show which assets failed".to_string(),
        };
        Some(AssetFailure(msg))
    }
}

/// Shared asset loading failure UI.
pub fn failure_ui<S: StateData>(
    loading_status: Res<Status<S>>,
    mut context: ResMut<EguiContext>,
    windows: Res<Windows>,
    failure: Res<AssetFailure>,
    mut state: ResMut<State<S>>,
    mut app_exit_events: EventWriter<'_, '_, AppExit>,
) {
    let width = f32::min(300.0, windows.primary().width() / 2.0);
    egui::Window::new("Error")
        .default_width(width)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(context.ctx_mut(), |ui| {
            ui.label(&failure.0);
            ui.label(WidgetText::from("You may still proceed, but the program may crash.").italics());
            ui.horizontal(|ui| {
                if ui.button("Proceed Anyway").clicked() {
                    state.set(loading_status.success_state.clone()).unwrap();
                }
                if ui.button("Exit").clicked() {
                    app_exit_events.send(AppExit);
                }
            });
        });
}
