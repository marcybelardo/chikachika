//! Framework-independent application coordination.
//!
//! [`HeadlessCoordinator`] owns the application-facing overlay collection and
//! wires it to persistence and the loopback server.  It deliberately has no
//! GUI dependencies so the native view can be replaced or tested independently.

use std::error::Error;
use std::fmt;
use std::net::SocketAddr;

use crate::model::{CanvasSize, ModelError, Overlay, OverlayId};
use crate::persistence::{PersistenceError, Store};
use crate::server::{self, HubError, OverlayHub, PublishResult, ServerError, ServerHandle};

/// Errors reported by application coordination operations.
#[derive(Debug)]
pub enum CoordinatorError {
    Persistence(PersistenceError),
    Model(ModelError),
    Hub(HubError),
    Server(ServerError),
    UnknownOverlay { id: OverlayId },
    NoOverlaySelected,
    ConfirmationRequired,
    AlreadyRunning,
}

/// Short alias for callers that prefer the application-oriented name.
pub type AppError = CoordinatorError;

impl fmt::Display for CoordinatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Persistence(error) => error.fmt(formatter),
            Self::Model(error) => error.fmt(formatter),
            Self::Hub(error) => error.fmt(formatter),
            Self::Server(error) => error.fmt(formatter),
            Self::UnknownOverlay { id } => write!(formatter, "overlay {id} was not found"),
            Self::NoOverlaySelected => formatter.write_str("no overlay is selected"),
            Self::ConfirmationRequired => formatter.write_str("overlay deletion requires confirmation"),
            Self::AlreadyRunning => formatter.write_str("the local server is already running"),
        }
    }
}

impl Error for CoordinatorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Persistence(error) => Some(error),
            Self::Model(error) => Some(error),
            Self::Hub(error) => Some(error),
            Self::Server(error) => Some(error),
            Self::UnknownOverlay { .. }
            | Self::NoOverlaySelected
            | Self::ConfirmationRequired
            | Self::AlreadyRunning => None,
        }
    }
}

impl From<PersistenceError> for CoordinatorError {
    fn from(error: PersistenceError) -> Self {
        Self::Persistence(error)
    }
}

impl From<ModelError> for CoordinatorError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

impl From<HubError> for CoordinatorError {
    fn from(error: HubError) -> Self {
        Self::Hub(error)
    }
}

impl From<ServerError> for CoordinatorError {
    fn from(error: ServerError) -> Self {
        Self::Server(error)
    }
}

/// The result of loading application state before the GUI starts.
///
/// A blocked result retains the store and the original error so a UI can
/// display the failure and offer retry/recovery without silently replacing the
/// source with an empty workspace.
pub enum BootstrapOutcome {
    /// The persisted snapshot was validated and is ready for use.
    Ready(HeadlessCoordinator),
    /// Startup is blocked until the bootstrap failure is acknowledged or
    /// resolved.
    Blocked(BootstrapFailure),
}

impl BootstrapOutcome {
    /// Returns the loaded coordinator when startup succeeded.
    pub fn into_coordinator(self) -> Option<HeadlessCoordinator> {
        match self {
            Self::Ready(coordinator) => Some(coordinator),
            Self::Blocked(_) => None,
        }
    }

    /// Returns the blocked startup details, if startup was blocked.
    pub fn failure(&self) -> Option<&BootstrapFailure> {
        match self {
            Self::Ready(_) => None,
            Self::Blocked(failure) => Some(failure),
        }
    }

    /// Retries loading the retained store after a recovery action.
    pub fn retry(self) -> Result<HeadlessCoordinator, CoordinatorError> {
        match self {
            Self::Ready(coordinator) => Ok(coordinator),
            Self::Blocked(failure) => HeadlessCoordinator::bootstrap(failure.into_store()),
        }
    }
}

/// A non-destructive failure that blocks application bootstrap.
#[derive(Debug)]
pub struct BootstrapFailure {
    store: Store,
    error: CoordinatorError,
}

impl BootstrapFailure {
    /// Returns the store that can be retried after a recovery action.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Returns the underlying failure for visible presentation.
    pub fn error(&self) -> &CoordinatorError {
        &self.error
    }

    /// Returns the persistence failure when persistence blocked startup.
    pub fn persistence_error(&self) -> Option<&PersistenceError> {
        match &self.error {
            CoordinatorError::Persistence(error) => Some(error),
            _ => None,
        }
    }

    fn into_store(self) -> Store {
        self.store
    }
}

impl fmt::Display for BootstrapFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "application startup is blocked: {}", self.error)
    }
}

impl Error for BootstrapFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

/// The application state shared by the GUI and browser-serving adapters.
///
/// Construction loads and validates the complete persisted snapshot before a
/// coordinator is returned.  The server is optional until [`Self::start`] has
/// received a successful bind/readiness result; consequently URL access is
/// unavailable before the server is actually ready.
pub struct HeadlessCoordinator {
    store: Store,
    overlays: Vec<Overlay>,
    selected: Option<OverlayId>,
    hub: OverlayHub,
    server: Option<ServerHandle>,
    server_address: Option<SocketAddr>,
    dirty: bool,
    last_error: Option<String>,
}

/// Alias for code that calls the coordinator the application state.
pub type AppCoordinator = HeadlessCoordinator;

impl HeadlessCoordinator {
    /// Loads persisted overlays and prepares a coordinator without starting a
    /// server. Any persistence or model validation error blocks bootstrap and
    /// leaves the source file untouched.
    pub fn bootstrap(store: Store) -> Result<Self, CoordinatorError> {
        match Self::bootstrap_outcome(store) {
            BootstrapOutcome::Ready(coordinator) => Ok(coordinator),
            BootstrapOutcome::Blocked(failure) => Err(failure.error),
        }
    }

    /// Loads startup state as a usable or blocked result. A blocked outcome
    /// retains the store and failure for visible recovery or retry.
    pub fn bootstrap_outcome(store: Store) -> BootstrapOutcome {
        let retained_store = store.clone();
        match store.load() {
            Ok(overlays) => match Self::from_overlays(store, overlays) {
                Ok(coordinator) => BootstrapOutcome::Ready(coordinator),
                Err(error) => BootstrapOutcome::Blocked(BootstrapFailure {
                    store: retained_store,
                    error,
                }),
            },
            Err(error) => BootstrapOutcome::Blocked(BootstrapFailure {
                store: retained_store,
                error: CoordinatorError::Persistence(error),
            }),
        }
    }

    /// Alias for [`Self::bootstrap`] that reads naturally at call sites.
    pub fn restore(store: Store) -> Result<Self, CoordinatorError> {
        Self::bootstrap(store)
    }

    /// Creates an empty coordinator for a store.  This is useful for a new
    /// installation and deterministic tests; it does not perform file I/O.
    pub fn empty(store: Store) -> Self {
        Self {
            store,
            overlays: Vec::new(),
            selected: None,
            hub: OverlayHub::new(),
            server: None,
            server_address: None,
            dirty: false,
            last_error: None,
        }
    }

    /// Builds a coordinator from an already loaded snapshot.
    ///
    /// Each overlay is registered before the coordinator is returned, so the
    /// in-memory list and browser hub cannot begin in a partially restored
    /// state.
    pub fn from_overlays(store: Store, overlays: Vec<Overlay>) -> Result<Self, CoordinatorError> {
        let hub = OverlayHub::new();
        for overlay in &overlays {
            hub.register(overlay.clone())
                .map_err(CoordinatorError::Hub)?;
        }

        let selected = overlays.first().map(Overlay::id);
        Ok(Self {
            store,
            overlays,
            selected,
            hub,
            server: None,
            server_address: None,
            dirty: false,
            last_error: None,
        })
    }

    /// Returns the store used for this coordinator.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Returns the shared hub used by the server and future editor adapters.
    pub fn hub(&self) -> OverlayHub {
        self.hub.clone()
    }

    /// Returns all overlays in stable application-list order.
    pub fn overlays(&self) -> &[Overlay] {
        &self.overlays
    }

    /// Looks up one overlay by its stable identity.
    pub fn overlay(&self, id: OverlayId) -> Option<&Overlay> {
        self.overlays.iter().find(|overlay| overlay.id() == id)
    }

    /// Returns the selected overlay, if any.
    pub fn selected_overlay(&self) -> Option<&Overlay> {
        self.selected.and_then(|id| self.overlay(id))
    }

    /// Returns the selected overlay identity, if any.
    pub const fn selected_overlay_id(&self) -> Option<OverlayId> {
        self.selected
    }

    /// Alias for [`Self::selected_overlay_id`].
    pub const fn selected_id(&self) -> Option<OverlayId> {
        self.selected_overlay_id()
    }

    /// Selects an existing overlay without changing its document or dirty
    /// state.
    pub fn select_overlay(&mut self, id: OverlayId) -> Result<(), CoordinatorError> {
        if self.overlay(id).is_none() {
            return Err(self.reject(CoordinatorError::UnknownOverlay { id }));
        }
        self.selected = Some(id);
        Ok(())
    }

    /// Returns whether a successful mutation has not yet been saved.
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Alias for [`Self::is_dirty`].
    pub const fn dirty(&self) -> bool {
        self.is_dirty()
    }

    /// Returns the latest user-visible error message, if one was recorded.
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Clears the latest recorded error after the UI has acknowledged it.
    pub fn clear_last_error(&mut self) {
        self.last_error = None;
    }

    /// Starts the default loopback server and waits for its readiness result.
    ///
    /// If binding fails, the coordinator remains stopped and all restored or
    /// edited overlay state remains available to the caller.
    pub fn start(&mut self) -> Result<(), CoordinatorError> {
        self.start_with(server::start_with_hub)
    }

    /// Starts the server through an injected starter.  The seam is useful for
    /// lifecycle tests and keeps production port selection in `server.rs`.
    pub fn start_with<F>(&mut self, start_server: F) -> Result<(), CoordinatorError>
    where
        F: FnOnce(OverlayHub) -> Result<ServerHandle, ServerError>,
    {
        if self.server.is_some() {
            return Err(self.reject(CoordinatorError::AlreadyRunning));
        }

        match start_server(self.hub.clone()) {
            Ok(server) => {
                self.server_address = Some(server.local_addr());
                self.server = Some(server);
                Ok(())
            }
            Err(error) => Err(self.reject(CoordinatorError::Server(error))),
        }
    }

    /// Returns whether the loopback server has completed startup.
    pub fn is_running(&self) -> bool {
        self.server.is_some()
    }

    /// Returns the bound address only after server startup has completed.
    pub const fn server_address(&self) -> Option<SocketAddr> {
        self.server_address
    }

    /// Alias for [`Self::server_address`].
    pub const fn local_addr(&self) -> Option<SocketAddr> {
        self.server_address()
    }

    /// Returns the selected overlay's browser-source URL only when both an
    /// overlay is selected and the server is ready.
    pub fn selected_url(&self) -> Option<String> {
        let address = self.server_address?;
        let id = self.selected?;
        Some(format!("http://{address}/overlay/{id}"))
    }

    /// Alias for [`Self::selected_url`].
    pub fn browser_source_url(&self) -> Option<String> {
        self.selected_url()
    }

    /// Gracefully stops the server and joins its dedicated thread.  Stopping
    /// always clears readiness state, including when joining reports an error.
    pub fn shutdown(&mut self) -> Result<(), CoordinatorError> {
        self.server_address = None;
        let Some(server) = self.server.take() else {
            return Ok(());
        };

        match server.shutdown() {
            Ok(()) => Ok(()),
            Err(error) => Err(self.reject(CoordinatorError::Server(error))),
        }
    }

    /// Creates, registers, selects, and marks a new overlay dirty.
    pub fn create_overlay(
        &mut self,
        name: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<OverlayId, CoordinatorError> {
        self.create_overlay_with_canvas(name, CanvasSize::new(width, height)?)
    }

    /// Creates an overlay using an already validated canvas size.
    pub fn create_overlay_with_canvas(
        &mut self,
        name: impl Into<String>,
        canvas: CanvasSize,
    ) -> Result<OverlayId, CoordinatorError> {
        let overlay = match Overlay::new(name, canvas) {
            Ok(overlay) => overlay,
            Err(error) => return Err(self.reject(CoordinatorError::Model(error))),
        };
        let id = overlay.id();
        if let Err(error) = self.hub.register(overlay.clone()) {
            return Err(self.reject(CoordinatorError::Hub(error)));
        }
        self.overlays.push(overlay);
        self.selected = Some(id);
        self.dirty = true;
        Ok(id)
    }

    /// Renames one overlay while preserving its stable identity and URL.
    pub fn rename_overlay(
        &mut self,
        id: OverlayId,
        name: impl Into<String>,
    ) -> Result<(), CoordinatorError> {
        self.update_overlay(id, |overlay| overlay.rename(name))
    }

    /// Renames the selected overlay.
    pub fn rename_selected(&mut self, name: impl Into<String>) -> Result<(), CoordinatorError> {
        let Some(id) = self.selected else {
            return Err(self.reject(CoordinatorError::NoOverlaySelected));
        };
        self.rename_overlay(id, name)
    }

    /// Applies a model mutation, publishes it to the hub, and marks the
    /// document dirty only when the mutation changes the model.
    pub fn update_overlay<F>(
        &mut self,
        id: OverlayId,
        mutate: F,
    ) -> Result<(), CoordinatorError>
    where
        F: FnOnce(&mut Overlay) -> Result<(), ModelError>,
    {
        let Some(index) = self.overlays.iter().position(|overlay| overlay.id() == id) else {
            return Err(self.reject(CoordinatorError::UnknownOverlay { id }));
        };

        let current = self.overlays[index].clone();
        let mut updated = current.clone();
        if let Err(error) = mutate(&mut updated) {
            return Err(self.reject(CoordinatorError::Model(error)));
        }
        if updated == current {
            return Ok(());
        }

        match self.hub.publish(&updated) {
            Ok(PublishResult::Published) => {
                self.overlays[index] = updated;
                self.dirty = true;
                Ok(())
            }
            Ok(PublishResult::Unchanged) => Ok(()),
            Err(error) => Err(self.reject(CoordinatorError::Hub(error))),
        }
    }

    /// Deletes an overlay only when the caller explicitly confirms the action.
    pub fn delete_overlay(
        &mut self,
        id: OverlayId,
        confirmed: bool,
    ) -> Result<(), CoordinatorError> {
        if !confirmed {
            return Err(self.reject(CoordinatorError::ConfirmationRequired));
        }
        let Some(index) = self.overlays.iter().position(|overlay| overlay.id() == id) else {
            return Err(self.reject(CoordinatorError::UnknownOverlay { id }));
        };

        match self.hub.remove(id) {
            Ok(Some(_)) => {}
            Ok(None) => return Err(self.reject(CoordinatorError::UnknownOverlay { id })),
            Err(error) => return Err(self.reject(CoordinatorError::Hub(error))),
        }
        self.overlays.remove(index);
        if self.selected == Some(id) {
            self.selected = self
                .overlays
                .get(index)
                .or_else(|| self.overlays.last())
                .map(Overlay::id);
        }
        self.dirty = true;
        Ok(())
    }

    /// Deletes the selected overlay after explicit confirmation.
    pub fn delete_selected(&mut self, confirmed: bool) -> Result<(), CoordinatorError> {
        let Some(id) = self.selected else {
            return Err(self.reject(CoordinatorError::NoOverlaySelected));
        };
        self.delete_overlay(id, confirmed)
    }

    /// Saves a complete immutable snapshot.  A failed save records a visible
    /// error and leaves the dirty flag set, preserving the in-memory work.
    pub fn save(&mut self) -> Result<(), CoordinatorError> {
        if let Err(error) = self.store.save(&self.overlays) {
            let error = CoordinatorError::Persistence(error);
            return Err(self.reject(error));
        }
        self.dirty = false;
        Ok(())
    }

    /// Saves only when dirty and reports whether a save was attempted.
    pub fn save_if_dirty(&mut self) -> Result<bool, CoordinatorError> {
        if !self.dirty {
            return Ok(false);
        }
        self.save().map(|()| true)
    }

    fn reject(&mut self, error: CoordinatorError) -> CoordinatorError {
        self.last_error = Some(error.to_string());
        error
    }
}

impl Drop for HeadlessCoordinator {
    fn drop(&mut self) {
        // `ServerHandle`'s Drop implementation signals the server.  Explicit
        // shutdown remains the normal path because it also joins the thread.
        let _ = self.server.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence;
    use crate::server;
    use std::fs;
    use std::path::Path;

    fn coordinator(path: &Path) -> HeadlessCoordinator {
        HeadlessCoordinator::bootstrap(Store::at(path)).expect("bootstrap coordinator")
    }

    #[test]
    fn bootstrap_restores_and_registers_overlays_before_server_start() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("nested").join("overlays.json");
        let mut original = Overlay::with_dimensions("Starting Soon", 1920, 1080).expect("overlay");
        original.add_text_widget("hello").expect("widget");
        persistence::save(&path, &[original.clone()]).expect("persist overlay");

        let app = coordinator(&path);
        assert_eq!(app.overlays(), &[original]);
        assert_eq!(app.selected_overlay_id(), Some(original.id()));
        assert!(!app.is_running());
        assert_eq!(app.selected_url(), None);
        assert_eq!(app.hub().snapshot(original.id()).expect("hub lock"), Some(original));
    }

    #[test]
    fn create_rename_delete_and_save_preserve_lifecycle_invariants() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("overlays.json");
        let mut app = coordinator(&path);

        let id = app.create_overlay("Starting Soon", 1920, 1080).expect("create");
        assert!(app.is_dirty());
        assert_eq!(app.selected_overlay_id(), Some(id));
        assert!(matches!(
            app.delete_overlay(id, false),
            Err(CoordinatorError::ConfirmationRequired)
        ));
        app.rename_selected("Live Soon").expect("rename");
        assert_eq!(app.selected_overlay().expect("selection").id(), id);
        app.save().expect("save");
        assert!(!app.is_dirty());
        app.delete_selected(true).expect("confirmed delete");
        assert!(app.overlays().is_empty());
        assert!(app.is_dirty());
    }

    #[test]
    fn failed_save_keeps_dirty_state_and_records_visible_error() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("existing-directory");
        fs::create_dir(&path).expect("destination directory");
        let mut app = coordinator(&path);
        app.create_overlay("Unsaved", 320, 240).expect("create");

        let result = app.save();
        assert!(matches!(result, Err(CoordinatorError::Persistence(_))));
        assert!(app.is_dirty());
        assert!(app.last_error().is_some());
        assert_eq!(app.overlays()[0].name(), "Unsaved");
    }

    #[test]
    fn startup_failure_is_atomic_and_success_gates_url_on_readiness() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("overlays.json");
        let mut app = coordinator(&path);
        let id = app.create_overlay("Live", 320, 240).expect("create");

        let failure = app.start_with(|_hub| {
            Err(ServerError::Bind(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "occupied",
            )))
        });
        assert!(matches!(failure, Err(CoordinatorError::Server(_))));
        assert!(!app.is_running());
        assert_eq!(app.selected_url(), None);
        assert_eq!(app.selected_overlay_id(), Some(id));

        app.start_with(|hub| server::start_on_port_with_hub(0, hub))
            .expect("start ephemeral server");
        let address = app.server_address().expect("ready address");
        let url = app.selected_url().expect("ready selected URL");
        assert!(url.starts_with(&format!("http://{address}/overlay/")));
        assert!(url.ends_with(&id.to_string()));
        app.shutdown().expect("shutdown server");
        assert_eq!(app.server_address(), None);
        assert_eq!(app.selected_url(), None);
    }

    #[test]
    fn malformed_restore_blocks_bootstrap_without_replacing_source() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("overlays.json");
        let source = br"not json";
        fs::write(&path, source).expect("malformed source");

        let result = HeadlessCoordinator::bootstrap(Store::at(&path));
        assert!(matches!(result, Err(CoordinatorError::Persistence(_))));
        assert_eq!(fs::read(&path).expect("source remains"), source.as_slice());
    }
}
