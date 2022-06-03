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
use egui::{Align2, WidgetText};

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
    pending: Vec<PendingAsset>,
    _marker: PhantomData<C>,
}

impl<C> PendingAssets<C> {
    /// Create a container for pending assets.
    pub fn new() -> Self { PendingAssets::default() }
    /// Track this handle as pending.
    pub fn track<T: Asset>(&mut self, path: &str, handle: Handle<T>) -> Handle<T> {
        self.pending.push(PendingAsset {
            path: path.to_string().into_boxed_str(),
            handle: handle.clone_untyped(),
        });
        handle
    }
    /// Load an asset from an asset server, and record it as pending.
    pub fn load_from<T: Asset>(&mut self, asset_server: &AssetServer, path: &str) -> Handle<T> {
        self.track(path, asset_server.load(path))
    }
}

/// Collection of assets.
pub trait AssetCollection: Sized + Send + Sync + 'static {
    /// Start loading the assets, and return the pending assets for checking.
    fn load(world: &World) -> (Self, PendingAssets<Self>);
    /// Track in addition the dependencies of some asset.
    fn track_dep(&self, _handle: HandleUntyped, _world: &World, _pending: &mut PendingAssets<Self>) {}
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
    enable_failure_ui: bool,
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
            failure_state: loader.failure_state.clone(),
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
                .label(LoaderSystem::Exit));
        if loader.enable_failure_ui {
            self.add_system_set(SystemSet::on_update(loader.failure_state)
                .with_system(failure_ui::<S>));
        }
        self
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
            enable_failure_ui: false,
            collection_count: 0,
            start_loading: SystemSet::on_enter(initial_state.clone()),
            check_loading: SystemSet::on_update(initial_state.clone()),
            initial_state,
            success_state,
            failure_state,
            _marker: PhantomData,
        }
    }

    /// Enable the default load failure UI (depends on [`bevy_egui`]).
    pub fn enable_failure_ui(mut self) -> Self {
        self.enable_failure_ui = true;
        self
    }

    /// Add a collection to be loaded.
    pub fn with_collection<C: AssetCollection>(mut self) -> Self {
        self.collection_count += 1;
        self.start_loading = self.start_loading.with_system(start_loading::<S, C>);
        self.check_loading = self.check_loading.with_system(
            check_loading_status::<S, C>
                .chain(track_dependencies::<C>)
                .chain(update_pending::<S, C>));
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
) -> Vec<HandleUntyped> {
    let mut current = 0;
    let mut finished = Vec::new();
    while current < pending_assets.pending.len() {
        let handle = &pending_assets.pending[current].handle;
        match asset_server.get_load_state(handle) {
            LoadState::Loading | LoadState::NotLoaded => {
                current += 1;
                continue;
            }
            LoadState::Loaded => {
                finished.push(pending_assets.pending.swap_remove(current).handle);
            }
            // treat unloaded assets as failures (be strict)
            LoadState::Failed | LoadState::Unloaded => {
                let asset = pending_assets.pending.swap_remove(current);
                loading_status.failures.push(asset.path);
            }
        }
    }
    finished
}

fn track_dependencies<T: AssetCollection>(
    finished: In<Vec<HandleUntyped>>,
    collection: Res<T>,
    world: &World,
) -> PendingAssets<T> {
    let mut pending_assets = PendingAssets::new();
    for handle in finished.0 {
        collection.track_dep(handle, world, &mut pending_assets);
    }
    pending_assets
}

fn update_pending<S: StateData, T: AssetCollection>(
    deps: In<PendingAssets<T>>,
    mut loading_status: ResMut<Status<S>>,
    mut pending_assets: ResMut<PendingAssets<T>>,
    mut commands: Commands,
) {
    if !deps.0.pending.is_empty() { // add even more
        pending_assets.pending.extend(deps.0.pending);
    } else if pending_assets.pending.is_empty() { // nothing left
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
    pub fn from_names(n: usize, names: impl IntoIterator<Item=impl AsRef<str>>) -> Option<AssetFailure> {
        use std::fmt::Write;
        let result = try_first_k_and_rest(
            n, names.into_iter(),
            || "Failed to load these assets:\n".to_string(),
            |msg, name| write!(msg, "• {}", name.as_ref()),
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
