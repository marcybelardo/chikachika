//! Framework-independent application coordination.
//!
//! [`HeadlessCoordinator`] owns workspace selection, the last successful save
//! baseline, and atomic publication of model candidates to the hosting hub.

use std::error::Error;
use std::fmt;
use std::net::SocketAddr;

use crate::model::{
    CanvasSize, ModelError, Overlay, OverlayId, TextWidget, TextWidgetId, validate_collection,
};
use crate::persistence::{PersistenceError, Store};
use crate::server::{HubError, OverlayHub, PublishResult, ServerError};

pub use crate::settings::SettingsState;

/// The narrow hosting boundary consumed by the coordinator and test doubles.
pub trait HubOperations: Clone {
    fn register_overlay(&self, overlay: Overlay) -> Result<(), HubError>;
    fn publish_overlay(&self, overlay: &Overlay) -> Result<PublishResult, HubError>;
    fn remove_overlay(&self, id: OverlayId) -> Result<Option<Overlay>, HubError>;
    fn snapshot_overlay(&self, id: OverlayId) -> Result<Option<Overlay>, HubError>;
}

impl HubOperations for OverlayHub {
    fn register_overlay(&self, overlay: Overlay) -> Result<(), HubError> {
        self.register(overlay)
    }
    fn publish_overlay(&self, overlay: &Overlay) -> Result<PublishResult, HubError> {
        self.publish(overlay)
    }
    fn remove_overlay(&self, id: OverlayId) -> Result<Option<Overlay>, HubError> {
        self.remove(id)
    }
    fn snapshot_overlay(&self, id: OverlayId) -> Result<Option<Overlay>, HubError> {
        self.snapshot(id)
    }
}

#[derive(Debug)]
pub enum CoordinatorError {
    Persistence(PersistenceError),
    Model(ModelError),
    Hub(HubError),
    Server(ServerError),
    UnknownOverlay {
        id: OverlayId,
    },
    NoOverlaySelected,
    UnknownWidget {
        id: TextWidgetId,
    },
    NoWidgetSelected,
    ConfirmationRequired,
    BootstrapCleanup {
        primary: HubError,
        cleanup: Vec<HubError>,
    },
    HubWorkspaceDivergence {
        id: OverlayId,
    },
}

pub type AppError = CoordinatorError;

impl fmt::Display for CoordinatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Persistence(e) => e.fmt(f),
            Self::Model(e) => e.fmt(f),
            Self::Hub(e) => e.fmt(f),
            Self::Server(e) => e.fmt(f),
            Self::UnknownOverlay { id } => write!(f, "overlay {id} was not found"),
            Self::NoOverlaySelected => write!(f, "no overlay is selected"),
            Self::UnknownWidget { id } => write!(f, "widget {id} was not found"),
            Self::NoWidgetSelected => write!(f, "no widget is selected"),
            Self::ConfirmationRequired => write!(f, "overlay deletion requires confirmation"),
            Self::BootstrapCleanup { primary, cleanup } => {
                write!(f, "could not restore overlays: {primary}; cleanup failed")?;
                for error in cleanup {
                    write!(f, ": {error}")?;
                }
                Ok(())
            }
            Self::HubWorkspaceDivergence { id } => write!(
                f,
                "overlay {id} exists in the workspace but not in the hosting hub"
            ),
        }
    }
}
impl Error for CoordinatorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Persistence(e) => Some(e),
            Self::Model(e) => Some(e),
            Self::Hub(e) => Some(e),
            Self::Server(e) => Some(e),
            _ => None,
        }
    }
}
impl From<PersistenceError> for CoordinatorError {
    fn from(e: PersistenceError) -> Self {
        Self::Persistence(e)
    }
}
impl From<ModelError> for CoordinatorError {
    fn from(e: ModelError) -> Self {
        Self::Model(e)
    }
}
impl From<HubError> for CoordinatorError {
    fn from(e: HubError) -> Self {
        Self::Hub(e)
    }
}
impl From<ServerError> for CoordinatorError {
    fn from(e: ServerError) -> Self {
        Self::Server(e)
    }
}

pub enum BootstrapOutcome {
    Ready(HeadlessCoordinator<OverlayHub>),
    Blocked(BootstrapFailure),
}
impl BootstrapOutcome {
    pub fn into_coordinator(self) -> Option<HeadlessCoordinator<OverlayHub>> {
        match self {
            Self::Ready(c) => Some(c),
            Self::Blocked(_) => None,
        }
    }
    pub fn failure(&self) -> Option<&BootstrapFailure> {
        match self {
            Self::Ready(_) => None,
            Self::Blocked(f) => Some(f),
        }
    }
    pub fn with_settings(self, settings: SettingsState) -> ApplicationBootstrap {
        ApplicationBootstrap {
            outcome: self,
            settings,
        }
    }
}

pub struct ApplicationBootstrap {
    outcome: BootstrapOutcome,
    settings: SettingsState,
}
impl ApplicationBootstrap {
    pub fn new(outcome: BootstrapOutcome, settings: SettingsState) -> Self {
        Self { outcome, settings }
    }
    pub fn outcome(&self) -> &BootstrapOutcome {
        &self.outcome
    }
    pub fn settings(&self) -> &SettingsState {
        &self.settings
    }
    pub fn settings_mut(&mut self) -> &mut SettingsState {
        &mut self.settings
    }
    pub fn into_parts(self) -> (BootstrapOutcome, SettingsState) {
        (self.outcome, self.settings)
    }
}

#[derive(Debug)]
pub struct BootstrapFailure {
    store: Option<Store>,
    error: CoordinatorError,
}
impl BootstrapFailure {
    pub(crate) fn without_store(error: PersistenceError) -> Self {
        Self {
            store: None,
            error: error.into(),
        }
    }
    pub fn store(&self) -> Option<&Store> {
        self.store.as_ref()
    }
    pub fn error(&self) -> &CoordinatorError {
        &self.error
    }
    pub fn persistence_error(&self) -> Option<&PersistenceError> {
        match &self.error {
            CoordinatorError::Persistence(e) => Some(e),
            _ => None,
        }
    }
}
impl fmt::Display for BootstrapFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "application startup is blocked: {}", self.error)
    }
}
impl Error for BootstrapFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

pub struct HeadlessCoordinator<H: HubOperations = OverlayHub> {
    store: Store,
    overlays: Vec<Overlay>,
    saved_content: Vec<Overlay>,
    selected: Option<OverlayId>,
    selected_widget: Option<TextWidgetId>,
    hub: H,
    server_address: Option<SocketAddr>,
    operation_error: Option<String>,
    server_error: Option<String>,
}
pub type AppCoordinator = HeadlessCoordinator<OverlayHub>;

impl HeadlessCoordinator<OverlayHub> {
    pub fn bootstrap(store: Store) -> Result<Self, CoordinatorError> {
        match Self::bootstrap_outcome(store) {
            BootstrapOutcome::Ready(c) => Ok(c),
            BootstrapOutcome::Blocked(f) => Err(f.error),
        }
    }
    pub fn bootstrap_outcome(store: Store) -> BootstrapOutcome {
        let retained = store.clone();
        match store.load() {
            Ok(overlays) => match Self::from_overlays(store, overlays) {
                Ok(coordinator) => BootstrapOutcome::Ready(coordinator),
                Err(error) => BootstrapOutcome::Blocked(BootstrapFailure {
                    store: Some(retained),
                    error,
                }),
            },
            Err(error) => BootstrapOutcome::Blocked(BootstrapFailure {
                store: Some(retained),
                error: error.into(),
            }),
        }
    }
    pub fn restore(store: Store) -> Result<Self, CoordinatorError> {
        Self::bootstrap(store)
    }
    pub fn empty(store: Store) -> Self {
        Self {
            store,
            overlays: Vec::new(),
            saved_content: Vec::new(),
            selected: None,
            selected_widget: None,
            hub: OverlayHub::new(),
            server_address: None,
            operation_error: None,
            server_error: None,
        }
    }
    pub fn from_overlays(store: Store, overlays: Vec<Overlay>) -> Result<Self, CoordinatorError> {
        Self::from_overlays_with_hub(store, overlays, OverlayHub::new())
    }
}

impl<H: HubOperations> HeadlessCoordinator<H> {
    pub fn set_server_address(&mut self, address: SocketAddr) {
        self.server_address = Some(address);
        self.server_error = None;
    }
    pub fn from_overlays_with_hub(
        store: Store,
        overlays: Vec<Overlay>,
        hub: H,
    ) -> Result<Self, CoordinatorError> {
        validate_collection(&overlays)?;
        let mut registered = Vec::with_capacity(overlays.len());
        for overlay in &overlays {
            if let Err(primary) = hub.register_overlay(overlay.clone()) {
                let cleanup: Vec<HubError> = registered
                    .into_iter()
                    .filter_map(|id| hub.remove_overlay(id).err())
                    .collect();
                return Err(if cleanup.is_empty() {
                    CoordinatorError::Hub(primary)
                } else {
                    CoordinatorError::BootstrapCleanup { primary, cleanup }
                });
            }
            registered.push(overlay.id());
        }
        let selected = overlays.first().map(Overlay::id);
        Ok(Self {
            store,
            saved_content: overlays.clone(),
            overlays,
            selected,
            selected_widget: None,
            hub,
            server_address: None,
            operation_error: None,
            server_error: None,
        })
    }

    pub fn store(&self) -> &Store {
        &self.store
    }
    pub fn hub(&self) -> H {
        self.hub.clone()
    }
    pub fn overlays(&self) -> &[Overlay] {
        &self.overlays
    }
    pub fn overlay(&self, id: OverlayId) -> Option<&Overlay> {
        self.overlays.iter().find(|overlay| overlay.id() == id)
    }
    pub fn selected_overlay(&self) -> Option<&Overlay> {
        self.selected.and_then(|id| self.overlay(id))
    }
    pub const fn selected_overlay_id(&self) -> Option<OverlayId> {
        self.selected
    }
    pub const fn selected_id(&self) -> Option<OverlayId> {
        self.selected
    }
    pub fn selected_widget(&self) -> Option<&TextWidget> {
        let id = self.selected_widget?;
        self.selected_overlay()?.widget(id)
    }
    pub const fn selected_widget_id(&self) -> Option<TextWidgetId> {
        self.selected_widget
    }
    pub fn is_dirty(&self) -> bool {
        self.overlays != self.saved_content
    }
    pub fn dirty(&self) -> bool {
        self.is_dirty()
    }
    pub fn last_error(&self) -> Option<&str> {
        self.operation_error
            .as_deref()
            .or(self.server_error.as_deref())
    }
    pub fn server_error(&self) -> Option<&str> {
        self.server_error.as_deref()
    }
    pub fn operation_error(&self) -> Option<&str> {
        self.operation_error.as_deref()
    }
    pub fn clear_last_error(&mut self) {
        self.operation_error = None;
    }
    pub fn record_server_error(&mut self, error: ServerError) {
        self.server_error = Some(CoordinatorError::Server(error).to_string());
    }
    pub fn record_settings_error(&mut self, error: &crate::settings::SettingsError) {
        self.server_error = Some(format!("could not load application settings: {error}"));
    }
    pub const fn server_address(&self) -> Option<SocketAddr> {
        self.server_address
    }
    pub const fn local_addr(&self) -> Option<SocketAddr> {
        self.server_address
    }

    pub fn selected_url(&self) -> Option<String> {
        let address = self.server_address?;
        let id = self.selected?;
        let overlay = self.overlay(id)?;
        if self.hub.snapshot_overlay(id).ok().flatten()? != *overlay {
            return None;
        }
        Some(format!("http://{address}/overlay/{id}"))
    }
    pub fn browser_source_url(&self) -> Option<String> {
        self.selected_url()
    }

    pub fn select_overlay(&mut self, id: OverlayId) -> Result<(), CoordinatorError> {
        if self.overlay(id).is_none() {
            return Err(self.reject(CoordinatorError::UnknownOverlay { id }));
        }
        if self.selected != Some(id) {
            self.selected_widget = None;
        }
        self.selected = Some(id);
        self.operation_error = None;
        Ok(())
    }
    pub fn select(&mut self, id: OverlayId) -> Result<(), CoordinatorError> {
        self.select_overlay(id)
    }
    pub fn clear_widget_selection(&mut self) {
        self.selected_widget = None;
    }
    pub fn select_widget(&mut self, id: TextWidgetId) -> Result<(), CoordinatorError> {
        let Some(overlay) = self.selected_overlay() else {
            return Err(self.reject(CoordinatorError::NoOverlaySelected));
        };
        if overlay.widget(id).is_none() {
            return Err(self.reject(CoordinatorError::UnknownWidget { id }));
        }
        self.selected_widget = Some(id);
        self.operation_error = None;
        Ok(())
    }

    pub fn create_overlay(
        &mut self,
        name: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<OverlayId, CoordinatorError> {
        let canvas = CanvasSize::new(width, height).map_err(|error| self.reject(error.into()))?;
        self.create_overlay_with_canvas(name, canvas)
    }
    pub fn create_overlay_with_canvas(
        &mut self,
        name: impl Into<String>,
        canvas: CanvasSize,
    ) -> Result<OverlayId, CoordinatorError> {
        let overlay = Overlay::new(name, canvas).map_err(|error| self.reject(error.into()))?;
        let id = overlay.id();
        if let Err(error) = self.hub.register_overlay(overlay.clone()) {
            return Err(self.reject(error.into()));
        }
        self.overlays.push(overlay);
        self.selected = Some(id);
        self.selected_widget = None;
        self.operation_error = None;
        Ok(id)
    }

    pub fn rename_overlay(
        &mut self,
        id: OverlayId,
        name: impl Into<String>,
    ) -> Result<(), CoordinatorError> {
        self.update_overlay(id, |overlay| overlay.rename(name))
    }
    pub fn rename_selected(&mut self, name: impl Into<String>) -> Result<(), CoordinatorError> {
        self.selected
            .ok_or_else(|| self.reject(CoordinatorError::NoOverlaySelected))
            .and_then(|id| self.rename_overlay(id, name))
    }

    /// Applies a model candidate and publishes it before changing workspace or selection.
    pub fn update_overlay<F>(&mut self, id: OverlayId, mutate: F) -> Result<(), CoordinatorError>
    where
        F: FnOnce(&mut Overlay) -> Result<(), ModelError>,
    {
        let index = self
            .overlays
            .iter()
            .position(|overlay| overlay.id() == id)
            .ok_or_else(|| self.reject(CoordinatorError::UnknownOverlay { id }))?;
        let current = self.overlays[index].clone();
        let mut updated = current.clone();
        mutate(&mut updated).map_err(|error| self.reject(error.into()))?;
        if updated == current {
            self.operation_error = None;
            return Ok(());
        }
        if updated.id() != id {
            return Err(self.reject(CoordinatorError::Model(
                ModelError::OverlayIdentityChanged {
                    expected: id,
                    found: updated.id(),
                },
            )));
        }
        let mut candidate = self.overlays.clone();
        candidate[index] = updated.clone();
        if let Err(error) = validate_collection(&candidate) {
            return Err(self.reject(CoordinatorError::Model(error)));
        }
        let selected_id = (self.selected == Some(id))
            .then_some(self.selected_widget)
            .flatten();
        let selected_index = selected_id.and_then(|widget_id| current.widget_index(widget_id));
        match self.hub.publish_overlay(&updated) {
            Ok(PublishResult::Published { .. }) | Ok(PublishResult::Unchanged) => {
                match self.hub.snapshot_overlay(id) {
                    Ok(Some(snapshot)) if snapshot == updated => {}
                    Ok(_) => {
                        return Err(self.reject(CoordinatorError::HubWorkspaceDivergence { id }));
                    }
                    Err(error) => return Err(self.reject(CoordinatorError::Hub(error))),
                }
                self.overlays[index] = updated;
                if self.selected == Some(id) {
                    self.selected_widget =
                        repair_widget_selection(&self.overlays[index], selected_id, selected_index);
                }
                self.operation_error = None;
                Ok(())
            }
            Err(error) => Err(self.reject(error.into())),
        }
    }

    pub fn add_widget(
        &mut self,
        overlay_id: OverlayId,
        widget: impl Into<TextWidget>,
    ) -> Result<TextWidgetId, CoordinatorError> {
        let widget = widget.into();
        let id = widget.id();
        self.update_overlay(overlay_id, move |overlay| {
            overlay.add_widget(widget).map(|_| ())
        })?;
        self.selected = Some(overlay_id);
        self.selected_widget = Some(id);
        Ok(id)
    }
    pub fn add_selected_widget(
        &mut self,
        widget: impl Into<TextWidget>,
    ) -> Result<TextWidgetId, CoordinatorError> {
        let id = self
            .selected
            .ok_or_else(|| self.reject(CoordinatorError::NoOverlaySelected))?;
        self.add_widget(id, widget)
    }
    pub fn duplicate_widget(
        &mut self,
        overlay_id: OverlayId,
        widget_id: TextWidgetId,
    ) -> Result<TextWidgetId, CoordinatorError> {
        let mut inserted = None;
        self.update_overlay(overlay_id, |overlay| {
            inserted = Some(overlay.duplicate_widget(widget_id)?);
            Ok(())
        })?;
        let id = inserted.expect("duplicate operation sets ID");
        self.selected = Some(overlay_id);
        self.selected_widget = Some(id);
        Ok(id)
    }
    pub fn delete_widget(
        &mut self,
        overlay_id: OverlayId,
        widget_id: TextWidgetId,
    ) -> Result<(), CoordinatorError> {
        self.update_overlay(overlay_id, |overlay| {
            overlay.delete_widget(widget_id).map(|_| ())
        })
    }
    pub fn delete_selected_widget(&mut self) -> Result<(), CoordinatorError> {
        let overlay_id = self
            .selected
            .ok_or_else(|| self.reject(CoordinatorError::NoOverlaySelected))?;
        let widget_id = self
            .selected_widget
            .ok_or_else(|| self.reject(CoordinatorError::NoWidgetSelected))?;
        self.delete_widget(overlay_id, widget_id)
    }
    pub fn move_widget_forward(
        &mut self,
        overlay_id: OverlayId,
        widget_id: TextWidgetId,
    ) -> Result<(), CoordinatorError> {
        self.update_overlay(overlay_id, |o| o.move_widget_forward(widget_id))
    }
    pub fn move_widget_backward(
        &mut self,
        overlay_id: OverlayId,
        widget_id: TextWidgetId,
    ) -> Result<(), CoordinatorError> {
        self.update_overlay(overlay_id, |o| o.move_widget_backward(widget_id))
    }

    pub fn delete_overlay(
        &mut self,
        id: OverlayId,
        confirmed: bool,
    ) -> Result<(), CoordinatorError> {
        if !confirmed {
            return Err(self.reject(CoordinatorError::ConfirmationRequired));
        }
        let index = self
            .overlays
            .iter()
            .position(|overlay| overlay.id() == id)
            .ok_or_else(|| self.reject(CoordinatorError::UnknownOverlay { id }))?;
        match self.hub.remove_overlay(id) {
            Ok(Some(_)) => {}
            Ok(None) => return Err(self.reject(CoordinatorError::HubWorkspaceDivergence { id })),
            Err(error) => return Err(self.reject(error.into())),
        }
        self.overlays.remove(index);
        if self.selected == Some(id) {
            self.selected = self
                .overlays
                .get(index)
                .or_else(|| self.overlays.last())
                .map(Overlay::id);
            self.selected_widget = None;
        }
        self.operation_error = None;
        Ok(())
    }
    pub fn delete_selected(&mut self, confirmed: bool) -> Result<(), CoordinatorError> {
        self.selected
            .ok_or_else(|| self.reject(CoordinatorError::NoOverlaySelected))
            .and_then(|id| self.delete_overlay(id, confirmed))
    }

    pub fn save(&mut self) -> Result<(), CoordinatorError> {
        let snapshot = self.overlays.clone();
        self.store
            .save(&snapshot)
            .map_err(|error| self.reject(error.into()))?;
        self.saved_content = snapshot;
        self.operation_error = None;
        Ok(())
    }
    pub fn save_if_dirty(&mut self) -> Result<bool, CoordinatorError> {
        if !self.is_dirty() {
            return Ok(false);
        }
        self.save().map(|_| true)
    }

    fn reject(&mut self, error: CoordinatorError) -> CoordinatorError {
        self.operation_error = Some(error.to_string());
        error
    }
}

fn repair_widget_selection(
    overlay: &Overlay,
    selected: Option<TextWidgetId>,
    old_index: Option<usize>,
) -> Option<TextWidgetId> {
    let Some(selected) = selected else {
        return None;
    };
    if overlay.widget(selected).is_some() {
        return Some(selected);
    }
    old_index.and_then(|index| {
        overlay
            .widgets()
            .get(index)
            .or_else(|| overlay.widgets().last())
            .map(TextWidget::id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence;
    use std::fs;
    use std::path::Path;

    fn coordinator(path: &Path) -> HeadlessCoordinator {
        HeadlessCoordinator::bootstrap(Store::at(path)).unwrap()
    }

    #[test]
    fn widget_selection_lifecycle() {
        let d = tempfile::tempdir().unwrap();
        let mut app = coordinator(&d.path().join("overlays.json"));
        let overlay = app.create_overlay("Live", 320, 240).unwrap();
        assert_eq!(app.selected_widget_id(), None);
        let first = app.add_widget(overlay, "first").unwrap();
        let second = app.add_widget(overlay, "second").unwrap();
        assert_eq!(app.selected_widget_id(), Some(second));
        app.select_widget(first).unwrap();
        app.select_widget(first).unwrap();
        app.select_widget(second).unwrap();
        app.select_overlay(overlay).unwrap();
        assert_eq!(app.selected_widget_id(), Some(second));
        app.delete_widget(overlay, second).unwrap();
        assert_eq!(app.selected_widget_id(), Some(first));
    }

    #[test]
    fn overlay_deletion_selection_fallback() {
        let d = tempfile::tempdir().unwrap();
        let mut app = coordinator(&d.path().join("overlays.json"));
        let first = app.create_overlay("First", 1, 1).unwrap();
        let second = app.create_overlay("Second", 1, 1).unwrap();
        let third = app.create_overlay("Third", 1, 1).unwrap();
        app.select_overlay(second).unwrap();
        app.delete_overlay(second, true).unwrap();
        assert_eq!(app.selected_overlay_id(), Some(third));
        app.delete_overlay(third, true).unwrap();
        assert_eq!(app.selected_overlay_id(), Some(first));
    }

    #[test]
    fn generic_update_repairs_selection() {
        let d = tempfile::tempdir().unwrap();
        let mut app = coordinator(&d.path().join("overlays.json"));
        let id = app.create_overlay("Live", 100, 100).unwrap();
        let first = app.add_widget(id, "first").unwrap();
        let second = app.add_widget(id, "second").unwrap();
        app.select_widget(first).unwrap();
        app.update_overlay(id, |overlay| overlay.delete_widget(first).map(|_| ()))
            .unwrap();
        assert_eq!(app.selected_widget_id(), Some(second));
    }

    #[test]
    fn rejected_mutation_preserves_workspace_and_selection() {
        let d = tempfile::tempdir().unwrap();
        let mut app = coordinator(&d.path().join("overlays.json"));
        let id = app.create_overlay("Live", 100, 100).unwrap();
        let widget = app.add_widget(id, "x").unwrap();
        app.select_widget(widget).unwrap();
        app.save().unwrap();
        let before = app.overlays().to_vec();
        let baseline = app.saved_content.clone();
        let hub = app.hub();
        let mut receiver = hub.subscribe(id).unwrap();
        let revision = receiver.borrow_and_update().revision();

        assert!(
            app.update_overlay(id, |overlay| overlay
                .set_widget_position(widget, crate::model::Position::new(101.0, 0.0)))
                .is_err()
        );
        assert_eq!(app.overlays(), before);
        assert_eq!(app.saved_content, baseline);
        assert!(!app.is_dirty());
        assert_eq!(app.selected_widget_id(), Some(widget));
        assert_eq!(hub.snapshot(id).unwrap(), Some(before[0].clone()));
        assert_eq!(receiver.borrow().revision(), revision);
        assert!(!receiver.has_changed().unwrap());
    }

    #[test]
    fn rejected_candidate_identity_and_collection_duplicates_are_atomic() {
        let d = tempfile::tempdir().unwrap();
        let id_path = d.path().join("identity.json");
        let mut app = coordinator(&id_path);
        let id = app.create_overlay("Target", 100, 100).unwrap();
        let first = app.add_widget(id, "first").unwrap();
        let other = app.create_overlay("Other", 100, 100).unwrap();
        let other_widget = app.add_widget(other, "other").unwrap();
        app.select_overlay(id).unwrap();
        app.select_widget(first).unwrap();
        app.save().unwrap();
        let before = app.overlays().to_vec();
        let baseline = app.saved_content.clone();
        let hub = app.hub();
        let mut receiver = hub.subscribe(id).unwrap();
        let revision = receiver.borrow_and_update().revision();

        let replacement = Overlay::with_dimensions("Wrong ID", 100, 100).unwrap();
        let replacement_id = replacement.id();
        let identity_error = app
            .update_overlay(id, |overlay| {
                *overlay = replacement;
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(
            identity_error,
            CoordinatorError::Model(ModelError::OverlayIdentityChanged {
                expected,
                found
            }) if expected == id && found == replacement_id
        ));
        assert_eq!(app.overlays(), before);
        assert_eq!(app.saved_content, baseline);
        assert_eq!(app.selected_overlay_id(), Some(id));
        assert_eq!(app.selected_widget_id(), Some(first));
        assert_eq!(hub.snapshot(id).unwrap(), Some(before[0].clone()));
        assert_eq!(receiver.borrow().revision(), revision);
        assert!(!receiver.has_changed().unwrap());

        let other_duplicate = app
            .overlay(other)
            .unwrap()
            .widget(other_widget)
            .unwrap()
            .clone();
        let duplicate_error = app
            .update_overlay(id, move |overlay| {
                *overlay = Overlay::from_parts(
                    id,
                    "Target".to_owned(),
                    overlay.canvas(),
                    vec![other_duplicate],
                )?;
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(
            duplicate_error,
            CoordinatorError::Model(ModelError::DuplicateWidgetId { id: duplicate })
                if duplicate == other_widget
        ));
        assert_eq!(app.overlays(), before);
        assert_eq!(app.saved_content, baseline);
        assert_eq!(app.selected_overlay_id(), Some(id));
        assert_eq!(app.selected_widget_id(), Some(first));
        assert_eq!(hub.snapshot(id).unwrap(), Some(before[0].clone()));
        assert_eq!(receiver.borrow().revision(), revision);
        assert!(!receiver.has_changed().unwrap());
    }

    #[test]
    fn dirty_tracks_saved_content_not_delivery() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("overlays.json");
        let mut app = coordinator(&path);
        let id = app.create_overlay("Live", 100, 100).unwrap();
        app.save().unwrap();
        assert!(!app.is_dirty());
        let widget = app.add_widget(id, "x").unwrap();
        app.save().unwrap();
        app.update_overlay(id, |o| o.set_widget_content(widget, "y"))
            .unwrap();
        assert!(app.is_dirty());
        app.update_overlay(id, |o| o.set_widget_content(widget, "x"))
            .unwrap();
        assert!(!app.is_dirty());
    }

    #[test]
    fn blocked_bootstrap_preserves_incompatible_store() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("overlays.json");
        let bytes = br#"{"format_version":1,"overlays":[]}"#;
        fs::write(&path, bytes).unwrap();
        let outcome = HeadlessCoordinator::bootstrap_outcome(Store::at(&path));
        assert!(outcome.failure().is_some());
        assert!(outcome.into_coordinator().is_none());
        assert_eq!(fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn save_failure_preserves_content_and_dirty_baseline() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("overlays.json");
        let mut app = coordinator(&path);
        app.create_overlay("Live", 100, 100).unwrap();
        app.save().unwrap();
        let before = app.overlays().to_vec();
        app.store = Store::at(d.path().join("directory"));
        fs::create_dir(app.store.path()).unwrap();
        assert!(app.save().is_err());
        assert_eq!(app.overlays(), before);
        assert!(!app.is_dirty());
    }

    #[test]
    fn bootstrap_restores_empty_state_clean() {
        let d = tempfile::tempdir().unwrap();
        let app = coordinator(&d.path().join("overlays.json"));
        assert!(!app.is_dirty());
        assert!(app.selected_overlay_id().is_none());
        let mut original = Overlay::with_dimensions("Saved", 100, 100).unwrap();
        original.add_widget("x").unwrap();
        persistence::save(d.path().join("overlays.json"), &[original]).unwrap();
    }
}
