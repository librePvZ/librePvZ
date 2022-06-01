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

use std::marker::PhantomData;
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::asset::{Asset, LoadState};
use bevy::ecs::schedule::StateData;
use bevy_egui::EguiContext;
use derivative::Derivative;
use egui::Align2;

/// Pending asset for loading.
#[derive(Debug, Clone)]
pub struct PendingAsset {
    /// Path to this asset.
    pub path: Box<str>,
    /// Handle to the asset, to be checked for loading status.
    pub handle: HandleUntyped,
}

/// A series of pending asset for loading.
#[derive(Debug, Clone)]
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct PendingAssets<C> {
    /// Pending assets.
    pub pending: Vec<PendingAsset>,
    _marker: PhantomData<C>,
}

impl<C> PendingAssets<C> {
    /// Create a container for pending assets.
    pub fn new() -> Self { PendingAssets::default() }
    /// Load an asset from an asset server, and record it as pending.
    pub fn load_from<T: Asset>(&mut self, asset_server: &AssetServer, path: &str) -> Handle<T> {
        let handle = asset_server.load(path);
        self.pending.push(PendingAsset {
            path: path.to_string().into_boxed_str(),
            handle: handle.clone_untyped(),
        });
        handle
    }
}

/// Collection of assets.
pub trait AssetCollection: Sized + Send + Sync + 'static {
    /// Start loading the assets, and return the pending assets for checking.
    fn load(world: &World) -> (Self, PendingAssets<Self>);
}

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

/// Asset loader API.
#[allow(missing_debug_implementations)]
pub struct AssetLoader<S = AssetState> {
    initial_state: S,
    success_state: S,
    failure_state: S,
    collection_count: usize,
    start_loading: SystemSet,
    check_loading: SystemSet,
    _marker: PhantomData<S>,
}

/// Extend [`App`] with an `attach_loader` API.
pub trait AssetLoaderExt {
    /// Attach a specific asset loader.
    fn attach_loader<S: StateData>(&mut self, loader: AssetLoader<S>) -> &mut Self;
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, SystemLabel)]
enum LoaderSystem {
    Init,
    Wait,
    Exit,
}

impl AssetLoaderExt for App {
    fn attach_loader<S: StateData>(&mut self, loader: AssetLoader<S>) -> &mut Self {
        let status = Status {
            pending_collection_count: loader.collection_count,
            success_state: loader.success_state,
            failure_state: loader.failure_state,
            failures: Vec::new(),
        };
        self.add_state(loader.initial_state.clone())
            .insert_resource(status)
            .add_system_set(loader.start_loading.label(LoaderSystem::Init))
            .add_system_set(loader.check_loading
                .label(LoaderSystem::Wait)
                .before(LoaderSystem::Exit))
            .add_system_set(SystemSet::on_update(loader.initial_state)
                .with_system(finish_loading::<S>)
                .label(LoaderSystem::Exit))
    }
}

impl Default for AssetLoader<AssetState> {
    fn default() -> Self {
        AssetLoader::new(
            AssetState::AssetLoading,
            AssetState::AssetReady,
            AssetState::LoadFailure,
        )
    }
}

impl<S: StateData> AssetLoader<S> {
    /// Create an asset loader, with specified state transition.
    pub fn new(initial_state: S, success_state: S, failure_state: S) -> AssetLoader<S> {
        AssetLoader {
            collection_count: 0,
            start_loading: SystemSet::on_enter(initial_state.clone()),
            check_loading: SystemSet::on_update(initial_state.clone()),
            initial_state,
            success_state,
            failure_state,
            _marker: PhantomData,
        }
    }

    /// Add a collection to be loaded.
    pub fn with_collection<C: AssetCollection>(mut self) -> Self {
        self.collection_count += 1;
        self.start_loading = self.start_loading.with_system(start_loading::<S, C>);
        self.check_loading = self.check_loading.with_system(check_loading_status::<S, C>);
        self
    }
}

/// Status for asset loading.
/// Only present for use if any asset failed loading.
#[allow(missing_debug_implementations)]
pub struct Status<S> {
    pending_collection_count: usize,
    failures: Vec<Box<str>>,
    success_state: S,
    failure_state: S,
}

impl<S> Status<S> {
    /// Get all the assets that failed loading.
    pub fn failures(&self) -> impl Iterator<Item=&str> {
        self.failures.iter().map(|s| s.as_ref())
    }
    /// Get a failure message for display.
    pub fn failure_message(&self) -> AssetFailure {
        AssetFailure::from_names(4, self.failures.iter()).unwrap()
    }
}

fn start_loading<S: StateData, T: AssetCollection>(world: &World, mut commands: Commands) {
    let (collection, pending) = T::load(world);
    commands.insert_resource(collection);
    commands.insert_resource(pending);
}

fn check_loading_status<S: StateData, T: AssetCollection>(
    mut loading_status: ResMut<Status<S>>,
    asset_server: Res<AssetServer>,
    mut pending_assets: ResMut<PendingAssets<T>>,
    mut commands: Commands,
) {
    let mut current = 0;
    while current < pending_assets.pending.len() {
        let handle = &pending_assets.pending[current].handle;
        match asset_server.get_load_state(handle) {
            LoadState::Loading | LoadState::NotLoaded => {
                current += 1;
                continue;
            }
            LoadState::Loaded => {
                pending_assets.pending.swap_remove(current);
            }
            // treat unloaded assets as failures (be strict)
            LoadState::Failed | LoadState::Unloaded => {
                let path = pending_assets.pending.swap_remove(current).path;
                loading_status.failures.push(path);
            }
        }
    }
    if pending_assets.pending.is_empty() {
        loading_status.pending_collection_count -= 1;
        commands.remove_resource::<PendingAssets<T>>();
    }
}

fn finish_loading<S: StateData>(
    loading_status: Res<Status<S>>,
    mut state: ResMut<State<S>>,
    mut commands: Commands,
) {
    if loading_status.pending_collection_count == 0 {
        if loading_status.failures.is_empty() {
            state.set(loading_status.success_state.clone()).unwrap();
            commands.remove_resource::<Status<S>>();
        } else {
            state.set(loading_status.failure_state.clone()).unwrap();
            commands.insert_resource(loading_status.failure_message());
        }
    }
}

/// Asset loading failure message.
#[derive(Debug, Clone)]
pub struct AssetFailure(pub String);

// init; k * first_k; [0 => {}; 1 => first_k; n => rest(n)]
fn try_first_k_and_rest<T, E, I: IntoIterator>(
    k: usize, iter: I,
    init: impl FnOnce() -> T,
    mut first_k: impl FnMut(&mut T, I::Item) -> Result<(), E>,
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
        first_k(&mut state, x)?;
    }
    if let Some(x) = iter.next() {
        let remaining = iter.count() + 1;
        match remaining {
            1 => first_k(&mut state, x)?,
            _ => rest(&mut state, remaining)?,
        }
    }
    Ok(Some(state))
}

impl AssetFailure {
    /// Construct asset loading failure from names of assets failing to load.
    pub fn from_names(n: usize, names: impl IntoIterator<Item=impl AsRef<str>>) -> Option<AssetFailure> {
        use std::fmt::Write;
        let result = try_first_k_and_rest(
            n, names.into_iter(),
            || "Failed to load these assets:\n".to_string(),
            |msg, name| writeln!(msg, "• {}", name.as_ref()),
            |msg, n| writeln!(msg, "... and {n} others"),
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
pub fn failure_ui(
    mut context: ResMut<EguiContext>,
    windows: Res<Windows>,
    failure: Res<AssetFailure>,
    mut app_exit_events: EventWriter<'_, '_, AppExit>,
) {
    let width = f32::min(200.0, windows.primary().width() / 2.0);
    egui::Window::new("Error")
        .default_width(width)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(context.ctx_mut(), |ui| {
            ui.label(&failure.0);
            ui.vertical_centered(|ui| if ui.button("Exit").clicked() {
                app_exit_events.send(AppExit);
            });
        });
}
