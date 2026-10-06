//! Native egui adapter for the issue #4 overlay workspace.
//!
//! The adapter owns transient form and confirmation state plus the separate
//! application settings state. The overlay collection, selection, dirty state,
//! persistence status, and URL readiness all remain in
//! [`crate::app::HeadlessCoordinator`].

#[cfg(test)]
use std::collections::HashMap;

use eframe::egui;

use crate::app::{ApplicationBootstrap, BootstrapOutcome, EditTarget, HeadlessCoordinator};
use crate::model::{Alignment, Color, FontFamily, OverlayId, Position, TextWidget, TextWidgetId};
use crate::settings::{MAX_PORT, MIN_PORT, Settings, SettingsState, Store as SettingsStore};

const WINDOW_INITIAL_SIZE: [f32; 2] = [1280.0, 800.0];
const WINDOW_MINIMUM_SIZE: [f32; 2] = [1024.0, 640.0];
const WIDGET_LIST_INITIAL_WIDTH: f32 = 220.0;
const WIDGET_LIST_MINIMUM_WIDTH: f32 = 180.0;
const INSPECTOR_INITIAL_WIDTH: f32 = 280.0;
const INSPECTOR_MINIMUM_WIDTH: f32 = 260.0;
const WORKSPACE_ITEM_SPACING: f32 = 8.0;
const WORKSPACE_PANEL_PADDING: f32 = 12.0;
const WORKSPACE_BODY_TEXT_SIZE: f32 = 14.0;
const USER_DOCUMENTATION_URL: &str =
    "https://github.com/marcybelardo/chikachika/blob/main/docs/user/README.md";

#[derive(Default)]
struct TransientState {
    create_open: bool,
    create_name: String,
    create_width: String,
    create_height: String,
    rename_open: bool,
    rename_id: Option<OverlayId>,
    rename_name: String,
    delete_target: Option<OverlayId>,
    dialog_error: Option<String>,
    preview_drag: Option<PreviewDrag>,
    pending_widget_reveal: Option<(OverlayId, TextWidgetId)>,
    /// The target that owns the inspector's transient state. Keeping this
    /// separate from the coordinator makes it possible to discard stale UI
    /// state immediately when a selection or overlay changes.
    inspector_target: Option<(OverlayId, TextWidgetId)>,
    name_focus_target: Option<(OverlayId, TextWidgetId)>,
    settings_port_input: String,
    settings_save_error: Option<String>,
    settings_save_succeeded: bool,
    settings_expanded: bool,
    active_edit: Option<EditTarget>,
    gesture_edit: Option<EditTarget>,
    text_editor_ids: Vec<egui::Id>,
    seen_text_editor_ids: Vec<egui::Id>,
    editor_selection: Option<(OverlayId, TextWidgetId)>,
    editor_selection_initialized: bool,
    focused_text_editor: Option<egui::Id>,
    preview_error: Option<String>,
    close_prompt_open: bool,
    allow_close: bool,
    close_error: Option<String>,
    #[cfg(test)]
    close_prompt_count: usize,
    #[cfg(test)]
    widget_selector_rects: HashMap<TextWidgetId, egui::Rect>,
    #[cfg(test)]
    overlay_selector_rects: HashMap<OverlayId, egui::Rect>,
    #[cfg(test)]
    control_rects: HashMap<String, egui::Rect>,
    #[cfg(test)]
    layer_action_enabled: HashMap<&'static str, bool>,
    #[cfg(test)]
    menu_action_enabled: HashMap<&'static str, bool>,
    #[cfg(test)]
    preview_rect: Option<egui::Rect>,
    #[cfg(test)]
    name_field_focused: bool,
    #[cfg(test)]
    widget_panel_rect: Option<egui::Rect>,
    #[cfg(test)]
    inspector_panel_rect: Option<egui::Rect>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewDrag {
    overlay_id: OverlayId,
    widget_id: TextWidgetId,
    pointer_offset: egui::Vec2,
    start_position: Position,
}

#[derive(Clone, Debug)]
enum EditEvent {
    Begin(EditTarget, bool),
    Change(EditTarget, EditValue),
    Finish(EditTarget, bool),
}

fn collect_numeric_edit_events(
    response: &egui::Response,
    target: EditTarget,
    value: EditValue,
    events: &mut Vec<EditEvent>,
) {
    if response.drag_started() {
        events.push(EditEvent::Begin(target.clone(), true));
    }
    if response.changed() {
        events.push(EditEvent::Change(target.clone(), value));
    }
    if response.drag_stopped() || response.lost_focus() {
        events.push(EditEvent::Finish(target, false));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeferredAction {
    Save,
    Undo,
    Redo,
    Quit,
    DeleteWidget,
    DuplicateWidget,
    CloseSave,
    CloseDiscard,
    CloseCancel,
}

/// The native application adapter.
///
/// A blocked bootstrap has no coordinator and therefore cannot expose a Save
/// action or a replacement workspace. A usable bootstrap owns the coordinator
/// and routes every document mutation through it.
pub struct ChikachikaApp {
    coordinator: Option<HeadlessCoordinator>,
    blocked: Option<crate::app::BootstrapFailure>,
    settings: SettingsState,
    transient: TransientState,
}

impl ChikachikaApp {
    /// Creates the adapter from the overlay startup result.
    ///
    /// This compatibility constructor supplies a deterministic default settings
    /// state without filesystem I/O for headless callers. Production startup
    /// should use [`Self::from_application_bootstrap`] so the loaded settings
    /// path and any settings error remain visible to the GUI.
    pub fn from_bootstrap(outcome: BootstrapOutcome) -> Self {
        Self::from_parts(
            outcome,
            SettingsState::from_settings(SettingsStore::at("settings.json"), Settings::default()),
        )
    }

    /// Creates the adapter from the complete production startup state.
    ///
    /// The coordinator and settings state are independent: an invalid settings
    /// source can leave the overlay workspace usable while preventing server
    /// startup, and saving a port changes only the next launch.
    pub fn from_application_bootstrap(bootstrap: ApplicationBootstrap) -> Self {
        let (outcome, settings) = bootstrap.into_parts();
        Self::from_parts(outcome, settings)
    }

    fn from_parts(outcome: BootstrapOutcome, settings: SettingsState) -> Self {
        let settings_port_input = settings.display_port().to_string();
        match outcome {
            BootstrapOutcome::Ready(coordinator) => Self {
                coordinator: Some(coordinator),
                blocked: None,
                settings,
                transient: TransientState {
                    settings_port_input,
                    ..TransientState::default()
                },
            },
            BootstrapOutcome::Blocked(failure) => Self {
                coordinator: None,
                blocked: Some(failure),
                settings,
                transient: TransientState {
                    settings_port_input,
                    ..TransientState::default()
                },
            },
        }
    }

    /// Creates a usable adapter around an injected coordinator.
    pub fn from_coordinator(coordinator: HeadlessCoordinator) -> Self {
        Self::from_bootstrap(BootstrapOutcome::Ready(coordinator))
    }

    /// Returns the authoritative coordinator when startup is usable.
    #[cfg(test)]
    pub fn coordinator(&self) -> Option<&HeadlessCoordinator> {
        self.coordinator.as_ref()
    }

    /// Returns the authoritative coordinator mutably for deterministic tests.
    #[cfg(test)]
    pub fn coordinator_mut(&mut self) -> Option<&mut HeadlessCoordinator> {
        self.coordinator.as_mut()
    }

    /// Returns the settings state for deterministic adapter assertions.
    #[cfg(test)]
    pub fn settings(&self) -> &SettingsState {
        &self.settings
    }

    /// Returns whether this adapter is displaying blocked startup recovery.
    #[cfg(test)]
    pub fn is_blocked(&self) -> bool {
        self.blocked.is_some()
    }

    /// Renders one frame without requiring a native window.
    pub fn render(&mut self, context: &egui::Context) {
        if let Some(coordinator) = self.coordinator.as_mut() {
            let gesture_active =
                self.transient.preview_drag.is_some() || self.transient.gesture_edit.is_some();
            let cancel_gesture = gesture_active
                && context
                    .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            if cancel_gesture
                && let Err(error) = cancel_active_gesture(coordinator, &mut self.transient)
            {
                self.transient.close_error = Some(error);
            }
        }
        if self.blocked.is_some() {
            self.render_blocked(context);
        } else if self.transient.close_prompt_open {
            self.render_close_only(context);
        } else {
            self.render_workspace(context);
        }
    }

    fn render_close_only(&mut self, context: &egui::Context) {
        context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        let _ = consume_shortcuts(context, &self.transient, false);
        let mut actions = Vec::new();
        render_close_prompt(context, &mut self.transient, &mut actions);
        let Some(coordinator) = self.coordinator.as_mut() else {
            return;
        };
        apply_deferred_actions(context, coordinator, &mut self.transient, actions);
    }

    fn render_blocked(&mut self, context: &egui::Context) {
        let failure = self
            .blocked
            .as_ref()
            .expect("blocked state is present while rendering blocked view");
        let error = failure.error().to_string();
        let source = failure
            .store()
            .map(|store| store.path().display().to_string());
        egui::CentralPanel::default().show(context, |ui| {
            ui.heading("Chikachika cannot open this workspace");
            ui.colored_label(egui::Color32::from_rgb(183, 28, 28), "Startup is blocked");
            ui.separator();
            ui.label("The saved overlay source was not changed.");
            ui.label(error);
            if let Some(source) = source {
                ui.label(format!("Source: {source}"));
                ui.label("First copy the exact source file to a separate backup location. Then repair it or move it aside yourself, and restart Chikachika.");
                ui.label("No replacement Save action is available here.");
            } else {
                ui.label("No persistence path could be resolved. Fix the platform app-data configuration yourself, then restart Chikachika.");
                ui.label("No replacement Save action is available because no source path exists.");
            }
        });
    }

    #[cfg(test)]
    fn open_create(&mut self) {
        begin_create(&mut self.transient);
    }

    #[cfg(test)]
    fn submit_create(&mut self) -> Result<(), String> {
        create_overlay_from_form(self.coordinator.as_mut(), &mut self.transient)
    }

    #[cfg(test)]
    fn cancel_dialogs(&mut self) {
        cancel_dialogs(&mut self.transient);
    }

    #[cfg(test)]
    fn open_rename(&mut self) -> Result<(), String> {
        let coordinator = self
            .coordinator
            .as_ref()
            .ok_or_else(|| "workspace is not available".to_owned())?;
        begin_rename(coordinator, &mut self.transient)
    }

    #[cfg(test)]
    fn apply_rename(&mut self) -> Result<(), String> {
        apply_rename(self.coordinator.as_mut(), &mut self.transient)
    }

    #[cfg(test)]
    fn open_delete(&mut self) -> Result<(), String> {
        let coordinator = self
            .coordinator
            .as_ref()
            .ok_or_else(|| "workspace is not available".to_owned())?;
        begin_delete(coordinator, &mut self.transient)
    }

    #[cfg(test)]
    fn confirm_delete(&mut self) -> Result<(), String> {
        confirm_delete(self.coordinator.as_mut(), &mut self.transient)
    }

    #[cfg(test)]
    fn save_workspace(&mut self) -> Result<(), String> {
        save_workspace(self.coordinator.as_mut())
    }

    #[cfg(test)]
    fn save_settings_port(&mut self) -> Result<(), String> {
        save_port_for_next_launch(&mut self.settings, &mut self.transient)
    }

    #[cfg(test)]
    fn select_named(&mut self, name: &str) -> Result<(), String> {
        select_named(self.coordinator.as_mut(), name)
    }

    #[cfg(test)]
    fn activate(&mut self, label: &str) -> Result<(), String> {
        match label {
            "Create overlay" | "Create Overlay" => {
                self.open_create();
                Ok(())
            }
            "Create" => self.submit_create(),
            "Cancel" => {
                self.cancel_dialogs();
                Ok(())
            }
            "Rename" => self.open_rename(),
            "Apply rename" => self.apply_rename(),
            "Delete" => self.open_delete(),
            "Confirm delete" => self.confirm_delete(),
            "Save" => self.save_workspace(),
            "Save port for next launch" | "Save port" => self.save_settings_port(),
            "Add text widget" | "Add" | "Add Text" => {
                add_selected_text_widget(self.coordinator.as_mut()).map(|_| ())
            }
            "Remove text widget" | "Delete widget" | "Edit/Delete" => {
                remove_selected_text_widget(self.coordinator.as_mut())
            }
            "Duplicate" => duplicate_selected_text_widget(self.coordinator.as_mut()).map(|_| ()),

            "Fit Canvas" => Ok(()),

            "Forward" => move_selected_text_widget(self.coordinator.as_mut(), true),
            "Backward" => move_selected_text_widget(self.coordinator.as_mut(), false),
            other => self.select_named(other),
        }
    }

    fn render_workspace(&mut self, context: &egui::Context) {
        apply_workspace_style(context);
        let close_requested = context.input(|input| input.viewport().close_requested());
        let quit_requested = context.input_mut(|input| {
            cfg!(target_os = "macos") && input.consume_key(egui::Modifiers::COMMAND, egui::Key::Q)
        });
        let close_triggered = close_requested || quit_requested;
        if close_triggered {
            let keep_focused_text = focused_text_editor(context).is_some();
            context.input_mut(|input| {
                input
                    .events
                    .retain(|event| keep_focused_text && matches!(event, egui::Event::Text(_)));
            });
        }
        let style = context.style();
        let coordinator = self
            .coordinator
            .as_mut()
            .expect("usable state has a coordinator");
        let transient = &mut self.transient;
        let mut deferred_actions = Vec::new();

        transient.text_editor_ids.clear();
        transient.focused_text_editor = focused_text_editor(context);
        let target = coordinator
            .selected_overlay_id()
            .zip(coordinator.selected_widget_id());
        if transient.editor_selection_initialized && transient.editor_selection != target {
            reset_text_undo(context, transient.seen_text_editor_ids.iter().copied());
        }
        transient.editor_selection = target;
        transient.editor_selection_initialized = true;
        if transient.inspector_target != target {
            clear_inspector_state(transient);
            transient.inspector_target = target;
        }

        egui::TopBottomPanel::top("menu-bar").show(context, |ui| {
            ui.horizontal(|ui| {
                let _file = ui.menu_button("File", |ui| {
                    let create = ui.button("Create Overlay");
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("File/Create Overlay".to_owned(), create.rect);
                    if create.clicked() {
                        begin_create(transient);
                        ui.close_menu();
                    }
                    let save = ui.add_enabled(coordinator.is_dirty(), egui::Button::new("Save"));
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("File/Save".to_owned(), save.rect);
                    if save.clicked() {
                        deferred_actions.push(DeferredAction::Save);
                        ui.close_menu();
                    }
                    let quit = ui.button("Quit");
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("File/Quit".to_owned(), quit.rect);
                    if quit.clicked() {
                        deferred_actions.push(DeferredAction::Quit);
                        ui.close_menu();
                    }
                });
                let _edit = ui.menu_button("Edit", |ui| {
                    let undo = ui.add_enabled(
                        coordinator.can_undo(),
                        egui::Button::new(menu_label("Undo", "Z")),
                    );
                    #[cfg(test)]
                    {
                        transient
                            .control_rects
                            .insert("Edit/Undo".to_owned(), undo.rect);
                        transient.menu_action_enabled.insert("Undo", undo.enabled());
                    }
                    if undo.clicked() {
                        deferred_actions.push(DeferredAction::Undo);
                        ui.close_menu();
                    }
                    let redo = ui.add_enabled(
                        coordinator.can_redo(),
                        egui::Button::new(menu_label("Redo", "Shift+Z")),
                    );
                    #[cfg(test)]
                    {
                        transient
                            .control_rects
                            .insert("Edit/Redo".to_owned(), redo.rect);
                        transient.menu_action_enabled.insert("Redo", redo.enabled());
                    }
                    if redo.clicked() {
                        deferred_actions.push(DeferredAction::Redo);
                        ui.close_menu();
                    }
                    let can_edit_overlay = coordinator.selected_overlay_id().is_some();
                    let can_edit_widget = coordinator.selected_widget_id().is_some();
                    let add = ui.add_enabled(can_edit_overlay, egui::Button::new("Add Text"));
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("Edit/Add Text".to_owned(), add.rect);
                    if add.clicked() {
                        let _ = add_selected_text_widget(Some(coordinator));
                        clear_inspector_state(transient);
                        transient.inspector_target = coordinator
                            .selected_overlay_id()
                            .zip(coordinator.selected_widget_id());
                        ui.close_menu();
                    }
                    let duplicate = ui.add_enabled(can_edit_widget, egui::Button::new("Duplicate"));
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("Edit/Duplicate".to_owned(), duplicate.rect);
                    if duplicate.clicked() {
                        let _ = duplicate_selected_text_widget(Some(coordinator));
                        clear_inspector_state(transient);
                        transient.inspector_target = coordinator
                            .selected_overlay_id()
                            .zip(coordinator.selected_widget_id());
                        ui.close_menu();
                    }
                    let delete = ui.add_enabled(can_edit_widget, egui::Button::new("Delete"));
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("Edit/Delete".to_owned(), delete.rect);
                    if delete.clicked() {
                        let _ = remove_selected_text_widget(Some(coordinator));
                        clear_inspector_state(transient);
                        transient.inspector_target = coordinator
                            .selected_overlay_id()
                            .zip(coordinator.selected_widget_id());
                        ui.close_menu();
                    }
                    let selected_index = coordinator.selected_widget().and_then(|widget| {
                        coordinator
                            .selected_overlay()?
                            .widgets()
                            .iter()
                            .position(|item| item.id() == widget.id())
                    });
                    let count = coordinator
                        .selected_overlay()
                        .map_or(0, |overlay| overlay.widgets().len());
                    let forward = ui.add_enabled(
                        selected_index.is_some_and(|index| index > 0),
                        egui::Button::new("Forward"),
                    );
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("Edit/Forward".to_owned(), forward.rect);
                    if forward.clicked() {
                        let _ = move_selected_text_widget(Some(coordinator), true);
                        ui.close_menu();
                    }
                    let backward = ui.add_enabled(
                        selected_index.is_some_and(|index| index + 1 < count),
                        egui::Button::new("Backward"),
                    );
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("Edit/Backward".to_owned(), backward.rect);
                    if backward.clicked() {
                        let _ = move_selected_text_widget(Some(coordinator), false);
                        ui.close_menu();
                    }
                });
                let _view = ui.menu_button("View", |ui| {
                    let fit = ui.button("Fit Canvas");
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("View/Fit Canvas".to_owned(), fit.rect);
                    if fit.clicked() {
                        context.request_repaint();
                        ui.close_menu();
                    }
                });
                let _help = ui.menu_button("Help", |ui| {
                    let documentation = ui.button("User Documentation");
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("Help/User Documentation".to_owned(), documentation.rect);
                    if documentation.clicked() {
                        open_url(context, USER_DOCUMENTATION_URL);
                        ui.close_menu();
                    }
                });
                #[cfg(test)]
                {
                    transient
                        .control_rects
                        .insert("File menu".to_owned(), _file.response.rect);
                    transient
                        .control_rects
                        .insert("Edit menu".to_owned(), _edit.response.rect);
                    transient
                        .control_rects
                        .insert("View menu".to_owned(), _view.response.rect);
                    transient
                        .control_rects
                        .insert("Help menu".to_owned(), _help.response.rect);
                }
            });
        });

        egui::TopBottomPanel::top("overlay-switcher").show(context, |ui| {
            ui.horizontal(|ui| {
                ui.label("Overlay");
                let overlays: Vec<(OverlayId, String)> = coordinator
                    .overlays()
                    .iter()
                    .map(|overlay| (overlay.id(), overlay.name().to_owned()))
                    .collect();
                let selected = coordinator.selected_overlay_id();
                let selected_name = coordinator
                    .selected_overlay()
                    .map(|overlay| overlay.name())
                    .unwrap_or("Select an overlay");
                let _overlay_switcher = egui::ComboBox::from_id_salt("overlay-switcher")
                    .selected_text(selected_name)
                    .show_ui(ui, |ui| {
                        for (id, name) in overlays {
                            let response = ui.selectable_label(selected == Some(id), name);
                            #[cfg(test)]
                            transient.overlay_selector_rects.insert(id, response.rect);
                            if response.clicked() {
                                let _ = select_overlay(coordinator, id);
                                clear_inspector_state(transient);
                                transient.inspector_target = None;
                            }
                        }
                    });
                #[cfg(test)]
                transient.control_rects.insert(
                    "Overlay selector".to_owned(),
                    _overlay_switcher.response.rect,
                );
                let create = ui.button("Create overlay");
                #[cfg(test)]
                transient
                    .control_rects
                    .insert("Create overlay".to_owned(), create.rect);
                if create.clicked() {
                    begin_create(transient);
                }
                let has_overlay = coordinator.selected_overlay_id().is_some();
                let rename = ui.add_enabled(has_overlay, egui::Button::new("Rename"));
                #[cfg(test)]
                transient
                    .control_rects
                    .insert("Rename".to_owned(), rename.rect);
                if rename.clicked() {
                    let _ = begin_rename(coordinator, transient);
                }
                let delete = ui.add_enabled(has_overlay, egui::Button::new("Delete"));
                #[cfg(test)]
                transient
                    .control_rects
                    .insert("Delete".to_owned(), delete.rect);
                if delete.clicked() {
                    let _ = begin_delete(coordinator, transient);
                }
                if let Some(overlay) = coordinator.selected_overlay() {
                    ui.separator();
                    ui.label(format!(
                        "{} × {}",
                        overlay.canvas().width(),
                        overlay.canvas().height()
                    ));
                }
            });
        });

        egui::TopBottomPanel::bottom("workspace-status").show(context, |ui| {
            ui.horizontal_wrapped(|ui| {
                if coordinator.is_dirty() {
                    ui.colored_label(egui::Color32::from_rgb(255, 183, 77), "Unsaved changes");
                    let save = ui.button("Save");
                    #[cfg(test)]
                    transient.control_rects.insert("Save".to_owned(), save.rect);
                    if save.clicked() {
                        let _ = save_workspace(Some(coordinator));
                    }
                } else {
                    ui.colored_label(egui::Color32::from_rgb(129, 199, 132), "Saved");
                }
                ui.separator();
                if let Some(address) = coordinator.server_address() {
                    ui.label(format!("Server running · port {}", address.port()));
                } else {
                    ui.label("Server unavailable");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let url = coordinator.selected_url();
                    let open = ui.add_enabled(url.is_some(), egui::Button::new("Open output"));
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("Open output".to_owned(), open.rect);
                    if open.clicked()
                        && let Some(url) = url.as_deref()
                    {
                        open_url(ui.ctx(), url);
                    }
                    let copy = ui.add_enabled(url.is_some(), egui::Button::new("Copy URL"));
                    #[cfg(test)]
                    transient
                        .control_rects
                        .insert("Copy URL".to_owned(), copy.rect);
                    if copy.clicked()
                        && let Some(url) = url.as_deref()
                    {
                        copy_url(ui.ctx(), url);
                    }
                });
            });
            if let Some(error) = coordinator.last_error() {
                ui.colored_label(
                    egui::Color32::from_rgb(239, 83, 80),
                    format!("Server/workspace error: {error}"),
                );
            }
            if let Some(error) = self.settings.settings_error() {
                ui.colored_label(
                    egui::Color32::from_rgb(239, 83, 80),
                    format!("Settings error: {error}"),
                );
            }
            if let Some(error) = transient.settings_save_error.as_deref() {
                ui.colored_label(
                    egui::Color32::from_rgb(239, 83, 80),
                    format!("Settings save error: {error}"),
                );
                ui.label("The previous configured port remains unchanged.");
            }
            if transient.settings_save_succeeded {
                ui.colored_label(
                    egui::Color32::from_rgb(129, 199, 132),
                    "Port saved for next launch",
                );
            }
            let marker = if transient.settings_expanded {
                "▼"
            } else {
                "▶"
            };
            let settings_toggle = ui.button(format!("{marker} Local server settings"));
            if settings_toggle.clicked() {
                transient.settings_expanded = !transient.settings_expanded;
            }
            #[cfg(test)]
            transient
                .control_rects
                .insert("Local server settings".to_owned(), settings_toggle.rect);
            if transient.settings_expanded {
                render_settings(
                    ui,
                    &mut self.settings,
                    transient,
                    coordinator.server_address().map(|address| address.port()),
                );
            }
        });

        egui::SidePanel::left("widget-list")
            .resizable(true)
            .default_width(WIDGET_LIST_INITIAL_WIDTH)
            .min_width(WIDGET_LIST_MINIMUM_WIDTH)
            .max_width(320.0)
            .frame(workspace_side_panel_frame(&style))
            .show(context, |ui| {
                #[cfg(test)]
                {
                    transient.widget_panel_rect =
                        Some(ui.max_rect().expand(WORKSPACE_PANEL_PADDING));
                }
                ui.heading("Widgets");
                if let Some(id) = coordinator.selected_overlay_id() {
                    ui.separator();
                    render_widget_selector(ui, coordinator, transient, id);
                } else {
                    ui.label("Create or select an overlay to add text widgets.");
                }
            });

        egui::SidePanel::right("widget-inspector")
            .resizable(true)
            .default_width(INSPECTOR_INITIAL_WIDTH)
            .min_width(INSPECTOR_MINIMUM_WIDTH)
            .max_width(360.0)
            .frame(workspace_side_panel_frame(&style))
            .show(context, |ui| {
                #[cfg(test)]
                {
                    transient.inspector_panel_rect =
                        Some(ui.max_rect().expand(WORKSPACE_PANEL_PADDING));
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(id) = coordinator.selected_overlay_id() {
                        render_selected_widget_inspector(ui, coordinator, transient, id);
                    } else {
                        ui.heading("Overlay information");
                        ui.label("Create or select an overlay to view its properties.");
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(workspace_central_panel_frame(&style))
            .show(context, |ui| {
                let Some(overlay) = coordinator.selected_overlay() else {
                    transient.preview_drag = None;
                    transient.inspector_target = None;
                    ui.vertical_centered(|ui| {
                        ui.heading("Canvas preview");
                        ui.add_space(16.0);
                        ui.label("Create or select an overlay to begin composing.");
                    });
                    return;
                };

                let id = overlay.id();
                let canvas = overlay.canvas();
                ui.vertical_centered(|ui| {
                    ui.heading("Canvas preview");
                    ui.label("Drag the selected widget to move it");
                    ui.add_space(8.0);
                    render_collection_preview(ui, coordinator, transient, id);
                    if let Some(error) = transient.preview_error.as_deref() {
                        ui.colored_label(egui::Color32::from_rgb(239, 83, 80), error);
                    }
                    ui.add_space(4.0);
                    ui.weak(format!(
                        "{} × {} transparent output",
                        canvas.width(),
                        canvas.height()
                    ));
                });
            });

        render_create_dialog(context, coordinator, transient);
        render_rename_dialog(context, coordinator, transient);
        render_delete_dialog(context, coordinator, transient);

        let modal_open = transient.close_prompt_open;
        if close_triggered && !modal_open {
            if transient.allow_close {
                transient.allow_close = false;
            } else if let Err(error) = request_close(context, coordinator, transient) {
                transient.close_error = Some(error);
            }
        }
        let modal_open = transient.close_prompt_open;
        let (cancel_gesture, mut shortcut_actions) =
            consume_shortcuts(context, transient, !modal_open && !close_triggered);
        if modal_open {
            shortcut_actions.clear();
            deferred_actions.clear();
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if cancel_gesture && let Err(error) = cancel_active_gesture(coordinator, transient) {
            transient.close_error = Some(error);
        }
        deferred_actions.extend(shortcut_actions);
        render_close_prompt(context, transient, &mut deferred_actions);
        for id in transient.text_editor_ids.iter().copied() {
            if !transient.seen_text_editor_ids.contains(&id) {
                transient.seen_text_editor_ids.push(id);
            }
        }
        apply_deferred_actions(context, coordinator, transient, deferred_actions);
    }
}

fn apply_deferred_actions(
    context: &egui::Context,
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
    actions: Vec<DeferredAction>,
) {
    for action in actions {
        if transient.close_prompt_open
            && !matches!(
                action,
                DeferredAction::CloseSave
                    | DeferredAction::CloseDiscard
                    | DeferredAction::CloseCancel
            )
        {
            continue;
        }
        match action {
            DeferredAction::Save => {
                if resolve_active_edit(coordinator, transient).is_ok()
                    && let Err(error) = save_workspace(Some(coordinator))
                {
                    transient.close_error = Some(error);
                }
            }
            DeferredAction::Undo | DeferredAction::Redo => {
                let is_redo = action == DeferredAction::Redo;
                let result = resolve_active_edit(coordinator, transient).and_then(|()| {
                    if is_redo {
                        coordinator.redo()
                    } else {
                        coordinator.undo()
                    }
                    .map_err(|error| error.to_string())
                });
                match result {
                    Ok(true) => {
                        reset_text_undo(
                            context,
                            transient
                                .seen_text_editor_ids
                                .iter()
                                .chain(transient.text_editor_ids.iter())
                                .copied(),
                        );
                        transient.editor_selection = coordinator
                            .selected_overlay_id()
                            .zip(coordinator.selected_widget_id());
                        transient.editor_selection_initialized = true;
                        if let Some(focused) = transient.focused_text_editor
                            && transient.text_editor_ids.contains(&focused)
                        {
                            context.memory_mut(|memory| memory.request_focus(focused));
                        }
                        transient.active_edit = None;
                        transient.gesture_edit = None;
                        clear_inspector_state(transient);
                        transient.inspector_target = coordinator
                            .selected_overlay_id()
                            .zip(coordinator.selected_widget_id());
                    }
                    Ok(false) => {}
                    Err(error) => transient.close_error = Some(error),
                }
            }
            DeferredAction::DeleteWidget | DeferredAction::DuplicateWidget => {
                if resolve_active_edit(coordinator, transient).is_ok() {
                    let result = if action == DeferredAction::DuplicateWidget {
                        duplicate_selected_text_widget(Some(coordinator)).map(|_| ())
                    } else {
                        remove_selected_text_widget(Some(coordinator))
                    };
                    if let Err(error) = result {
                        transient.close_error = Some(error);
                    }
                    clear_inspector_state(transient);
                    transient.inspector_target = coordinator
                        .selected_overlay_id()
                        .zip(coordinator.selected_widget_id());
                }
            }
            DeferredAction::Quit => {
                if let Err(error) = request_close(context, coordinator, transient) {
                    transient.close_error = Some(error);
                }
            }
            DeferredAction::CloseSave => match resolve_active_edit(coordinator, transient) {
                Ok(()) => match save_workspace(Some(coordinator)) {
                    Ok(()) => allow_close(context, transient),
                    Err(error) => transient.close_error = Some(error),
                },
                Err(error) => transient.close_error = Some(error),
            },
            DeferredAction::CloseDiscard => allow_close(context, transient),
            DeferredAction::CloseCancel => {
                transient.close_prompt_open = false;
                transient.close_error = None;
            }
        }
    }
}

fn apply_workspace_style(context: &egui::Context) {
    let mut style = context.style().as_ref().clone();
    style.visuals = egui::Visuals::dark();
    style.spacing.item_spacing = egui::vec2(WORKSPACE_ITEM_SPACING, WORKSPACE_ITEM_SPACING);
    style.spacing.window_margin = egui::Margin::same(WORKSPACE_PANEL_PADDING);
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(WORKSPACE_BODY_TEXT_SIZE),
    );
    context.set_style(style);
}

fn workspace_side_panel_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::side_top_panel(style).inner_margin(WORKSPACE_PANEL_PADDING)
}

fn workspace_central_panel_frame(style: &egui::Style) -> egui::Frame {
    egui::Frame::central_panel(style).inner_margin(WORKSPACE_PANEL_PADDING)
}

impl Default for ChikachikaApp {
    fn default() -> Self {
        Self::from_coordinator(HeadlessCoordinator::empty(crate::persistence::Store::at(
            "overlays.json",
        )))
    }
}

impl eframe::App for ChikachikaApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(context);
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TextEditorValues {
    id: TextWidgetId,
    name: String,
    content: String,
    font_family: FontFamily,
    position: Position,
    font_size: f32,
    color: Color,
    alignment: Alignment,
}

impl TextEditorValues {
    fn from_widget(widget: &TextWidget) -> Self {
        Self {
            id: widget.id(),
            name: widget.name().to_owned(),
            content: widget.content().to_owned(),
            font_family: widget.font_family(),
            position: widget.position(),
            font_size: widget.font_size(),
            color: widget.color(),
            alignment: widget.alignment(),
        }
    }
}

fn clear_inspector_state(transient: &mut TransientState) {
    transient.preview_drag = None;
    transient.pending_widget_reveal = None;
    transient.name_focus_target = None;
}

fn add_selected_text_widget(
    coordinator: Option<&mut HeadlessCoordinator>,
) -> Result<TextWidgetId, String> {
    let coordinator = coordinator.ok_or_else(|| "workspace is not available".to_owned())?;
    coordinator
        .add_selected_widget(TextWidget::new("Text"))
        .map_err(|error| error.to_string())
}

fn add_text_widget(
    coordinator: &mut HeadlessCoordinator,
    overlay_id: OverlayId,
) -> Result<TextWidgetId, String> {
    coordinator
        .add_widget(overlay_id, TextWidget::new("Text"))
        .map_err(|error| error.to_string())
}

fn remove_selected_text_widget(
    coordinator: Option<&mut HeadlessCoordinator>,
) -> Result<(), String> {
    coordinator
        .ok_or_else(|| "workspace is not available".to_owned())?
        .delete_selected_widget()
        .map_err(|error| error.to_string())
}

fn duplicate_selected_text_widget(
    coordinator: Option<&mut HeadlessCoordinator>,
) -> Result<TextWidgetId, String> {
    let coordinator = coordinator.ok_or_else(|| "workspace is not available".to_owned())?;
    let overlay_id = coordinator
        .selected_overlay_id()
        .ok_or_else(|| "no overlay is selected".to_owned())?;
    let widget_id = coordinator
        .selected_widget_id()
        .ok_or_else(|| "no widget is selected".to_owned())?;
    coordinator
        .duplicate_widget(overlay_id, widget_id)
        .map_err(|error| error.to_string())
}

fn move_selected_text_widget(
    coordinator: Option<&mut HeadlessCoordinator>,
    forward: bool,
) -> Result<(), String> {
    let coordinator = coordinator.ok_or_else(|| "workspace is not available".to_owned())?;
    let overlay_id = coordinator
        .selected_overlay_id()
        .ok_or_else(|| "no overlay is selected".to_owned())?;
    let widget_id = coordinator
        .selected_widget_id()
        .ok_or_else(|| "no widget is selected".to_owned())?;
    let result = if forward {
        coordinator.move_widget_forward(overlay_id, widget_id)
    } else {
        coordinator.move_widget_backward(overlay_id, widget_id)
    };
    result.map_err(|error| error.to_string())
}

#[derive(Clone, Debug)]
enum EditValue {
    Name(String),
    Content(String),
    FontSize(f32),
    Color(Color),
    Position(Position),
}

fn edit_target(overlay_id: OverlayId, widget_id: TextWidgetId, field: &str) -> EditTarget {
    EditTarget {
        overlay_id,
        widget_id,
        field: field.to_owned(),
    }
}

fn apply_edit_value(
    coordinator: &mut HeadlessCoordinator,
    target: &EditTarget,
    value: EditValue,
) -> Result<(), String> {
    coordinator
        .begin_edit(target.clone())
        .and_then(|()| {
            coordinator.update_edit(target, move |overlay| match value {
                EditValue::Name(value) => overlay.rename_widget(target.widget_id, value),
                EditValue::Content(value) => overlay.set_widget_content(target.widget_id, value),
                EditValue::FontSize(value) => overlay.set_widget_font_size(target.widget_id, value),
                EditValue::Color(value) => overlay.set_widget_color(target.widget_id, value),
                EditValue::Position(value) => overlay.set_widget_position(target.widget_id, value),
            })
        })
        .map_err(|error| error.to_string())
}

fn finish_edit(
    coordinator: &mut HeadlessCoordinator,
    target: &EditTarget,
    cancel: bool,
) -> Result<(), String> {
    if cancel {
        coordinator.cancel_edit(target)
    } else {
        coordinator.commit_edit(target)
    }
    .map_err(|error| error.to_string())
}

fn cancel_active_gesture(
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
) -> Result<(), String> {
    let target = transient.gesture_edit.clone().or_else(|| {
        transient
            .preview_drag
            .map(|drag| edit_target(drag.overlay_id, drag.widget_id, "canvas_position"))
    });
    if let Some(target) = target {
        finish_edit(coordinator, &target, true)?;
        transient.active_edit = None;
        transient.gesture_edit = None;
        transient.preview_drag = None;
        clear_inspector_state(transient);
        transient.inspector_target = coordinator
            .selected_overlay_id()
            .zip(coordinator.selected_widget_id());
    }
    Ok(())
}

fn resolve_active_edit(
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
) -> Result<(), String> {
    let target = transient
        .active_edit
        .clone()
        .or_else(|| coordinator.pending_edit_target().cloned());
    if let Some(target) = target {
        finish_edit(coordinator, &target, false)?;
        transient.active_edit = None;
        transient.gesture_edit = None;
    }
    Ok(())
}

fn focused_text_editor(context: &egui::Context) -> Option<egui::Id> {
    let focused = context.memory(|memory| memory.focused())?;
    egui::widgets::text_edit::TextEditState::load(context, focused).map(|_| focused)
}

fn reset_text_undo(context: &egui::Context, ids: impl IntoIterator<Item = egui::Id>) {
    for id in ids {
        if let Some(mut state) = egui::widgets::text_edit::TextEditState::load(context, id) {
            state.clear_undoer();
            state.store(context, id);
        }
    }
}

fn consume_shortcuts(
    context: &egui::Context,
    transient: &TransientState,
    allow_actions: bool,
) -> (bool, Vec<DeferredAction>) {
    let focused_text = focused_text_editor(context).is_some();
    context.input_mut(|input| {
        let primary = if cfg!(target_os = "macos") {
            egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::CTRL
        };
        let mut actions = Vec::new();
        let save = input.consume_key(primary, egui::Key::S);
        let (redo, undo, duplicate, delete, backspace) = if focused_text {
            (false, false, false, false, false)
        } else {
            (
                input.consume_key(primary | egui::Modifiers::SHIFT, egui::Key::Z),
                input.consume_key(primary, egui::Key::Z),
                input.consume_key(primary, egui::Key::D),
                input.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
                input.consume_key(egui::Modifiers::NONE, egui::Key::Backspace),
            )
        };
        let quit = cfg!(target_os = "macos") && input.consume_key(primary, egui::Key::Q);
        if allow_actions && save {
            actions.push(DeferredAction::Save);
        }
        if allow_actions && !focused_text {
            if redo {
                actions.push(DeferredAction::Redo);
            } else if undo {
                actions.push(DeferredAction::Undo);
            }
            if duplicate {
                actions.push(DeferredAction::DuplicateWidget);
            }
            if delete || backspace {
                actions.push(DeferredAction::DeleteWidget);
            }
        }
        if allow_actions && quit {
            actions.push(DeferredAction::Quit);
        }
        let cancel_gesture = (transient.preview_drag.is_some() || transient.gesture_edit.is_some())
            && input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
        (cancel_gesture, actions)
    })
}

fn render_inspector_color_control(
    ui: &mut egui::Ui,
    transient: &mut TransientState,
    overlay_id: OverlayId,
    widget_id: TextWidgetId,
    color: &mut Color,
    edit_events: &mut Vec<EditEvent>,
) {
    let popup_id = ui.make_persistent_id(("inspector-color-popup", overlay_id, widget_id));
    let swatch_color = egui::Color32::from_rgba_unmultiplied(
        color.red(),
        color.green(),
        color.blue(),
        color.alpha(),
    );
    let swatch_size = ui.spacing().interact_size;
    let (swatch_rect, _) = ui.allocate_exact_size(swatch_size, egui::Sense::hover());
    let swatch = ui.interact(
        swatch_rect,
        ui.make_persistent_id(("inspector-color-swatch", overlay_id, widget_id)),
        egui::Sense::click(),
    );
    ui.painter().rect_filled(swatch.rect, 2.0, swatch_color);
    if swatch.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(popup_id));
    }
    #[cfg(test)]
    transient
        .control_rects
        .insert("Color".to_owned(), swatch.rect);

    if ui.memory(|memory| memory.is_popup_open(popup_id)) {
        let mut channel_values = [
            color.red() as f32,
            color.green() as f32,
            color.blue() as f32,
            color.alpha() as f32,
        ];
        let popup = egui::Area::new(popup_id)
            .kind(egui::UiKind::Picker)
            .order(egui::Order::Foreground)
            .fixed_pos(swatch.rect.right_bottom())
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(220.0);
                    for (index, (name, field)) in [
                        ("R", "color_r"),
                        ("G", "color_g"),
                        ("B", "color_b"),
                        ("A", "color_a"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        ui.push_id(
                            ("inspector-color-field", overlay_id, widget_id, field),
                            |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(name);
                                    let slider = ui.add(
                                        egui::Slider::new(&mut channel_values[index], 0.0..=255.0)
                                            .show_value(false),
                                    );
                                    #[cfg(test)]
                                    transient
                                        .control_rects
                                        .insert(format!("Color {name} slider"), slider.rect);
                                    if slider.drag_started() {
                                        edit_events.push(EditEvent::Begin(
                                            edit_target(overlay_id, widget_id, "color"),
                                            true,
                                        ));
                                    }
                                    if slider.changed() {
                                        *color = Color::rgba(
                                            channel_values[0].round() as u8,
                                            channel_values[1].round() as u8,
                                            channel_values[2].round() as u8,
                                            channel_values[3].round() as u8,
                                        );
                                        edit_events.push(EditEvent::Change(
                                            edit_target(overlay_id, widget_id, "color"),
                                            EditValue::Color(*color),
                                        ));
                                    }
                                    if slider.drag_stopped() {
                                        edit_events.push(EditEvent::Finish(
                                            edit_target(overlay_id, widget_id, "color"),
                                            false,
                                        ));
                                    }
                                    let numeric = ui.add(
                                        egui::DragValue::new(&mut channel_values[index])
                                            .range(0.0..=255.0)
                                            .speed(1.0),
                                    );
                                    #[cfg(test)]
                                    transient
                                        .control_rects
                                        .insert(format!("Color {name}"), numeric.rect);
                                    if numeric.has_focus() {
                                        transient.text_editor_ids.push(numeric.id);
                                        transient.focused_text_editor = Some(numeric.id);
                                    }
                                    if numeric.changed() {
                                        *color = Color::rgba(
                                            channel_values[0].round() as u8,
                                            channel_values[1].round() as u8,
                                            channel_values[2].round() as u8,
                                            channel_values[3].round() as u8,
                                        );
                                        edit_events.push(EditEvent::Change(
                                            edit_target(overlay_id, widget_id, "color"),
                                            EditValue::Color(*color),
                                        ));
                                    }
                                    if numeric.drag_started() {
                                        edit_events.push(EditEvent::Begin(
                                            edit_target(overlay_id, widget_id, "color"),
                                            true,
                                        ));
                                    }
                                    if numeric.drag_stopped() || numeric.lost_focus() {
                                        edit_events.push(EditEvent::Finish(
                                            edit_target(overlay_id, widget_id, "color"),
                                            false,
                                        ));
                                    }
                                });
                            },
                        );
                    }
                });
            })
            .response;
        let escape_pressed = ui.input(|input| input.key_pressed(egui::Key::Escape));
        if escape_pressed || (!swatch.clicked() && popup.clicked_elsewhere()) {
            ui.memory_mut(|memory| memory.close_popup());
        }
    }
}

fn render_widget_selector(
    ui: &mut egui::Ui,
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
    overlay_id: OverlayId,
) {
    ui.horizontal(|ui| {
        let response = ui.button("Add text");
        #[cfg(test)]
        transient
            .control_rects
            .insert("Add text widget".to_owned(), response.rect);
        if response.clicked() {
            if add_text_widget(coordinator, overlay_id).is_ok() {
                clear_inspector_state(transient);
                transient.inspector_target = coordinator
                    .selected_overlay_id()
                    .zip(coordinator.selected_widget_id());
            }
        }
    });
    ui.label("Frontmost first");
    let selected_widget = coordinator.selected_widget_id();
    let rows: Vec<(TextWidgetId, String)> = coordinator
        .overlay(overlay_id)
        .map(|overlay| {
            overlay
                .widgets()
                .iter()
                .map(|widget| (widget.id(), widget.name().to_owned()))
                .collect()
        })
        .unwrap_or_default();
    let reveal_widget =
        transient
            .pending_widget_reveal
            .take()
            .and_then(|(target_overlay, target_widget)| {
                (target_overlay == overlay_id
                    && rows
                        .iter()
                        .any(|(widget_id, _)| *widget_id == target_widget))
                .then_some(target_widget)
            });
    if rows.is_empty() {
        ui.label("No text widgets yet.");
    } else {
        egui::ScrollArea::vertical()
            .id_salt(("widget-rows", overlay_id))
            .animated(false)
            .show(ui, |ui| {
                for (widget_id, name) in rows {
                    ui.push_id(("widget-row", overlay_id, widget_id), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("TXT").weak().monospace());
                            let selected = selected_widget == Some(widget_id);
                            let marker = if selected { "▶ " } else { "  " };
                            let name_width = (ui.available_width() - 88.0).max(32.0);
                            let response = ui.add_sized(
                                [name_width, ui.spacing().interact_size.y],
                                egui::Button::new(format!("{marker}{name}")).selected(selected),
                            );
                            #[cfg(test)]
                            transient
                                .widget_selector_rects
                                .insert(widget_id, response.rect);
                            if response.clicked() {
                                if coordinator.select_widget(widget_id).is_ok() {
                                    clear_inspector_state(transient);
                                    transient.inspector_target = Some((overlay_id, widget_id));
                                }
                            }
                            if reveal_widget == Some(widget_id) {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                            let rename = ui.small_button("Rename");
                            #[cfg(test)]
                            transient
                                .control_rects
                                .insert(format!("Rename widget {widget_id}"), rename.rect);
                            if rename.clicked() {
                                if coordinator.select_widget(widget_id).is_ok() {
                                    clear_inspector_state(transient);
                                    transient.inspector_target = Some((overlay_id, widget_id));
                                    transient.name_focus_target = Some((overlay_id, widget_id));
                                }
                            }
                        });
                    });
                }
            });
    }
}

fn render_selected_widget_inspector(
    ui: &mut egui::Ui,
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
    overlay_id: OverlayId,
) {
    let Some(overlay) = coordinator.overlay(overlay_id) else {
        clear_inspector_state(transient);
        return;
    };
    let canvas = overlay.canvas();
    let Some(widget) = coordinator.selected_widget().cloned() else {
        clear_inspector_state(transient);
        ui.heading("Overlay information");
        ui.label("No widget selected.");
        ui.label(format!("Canvas: {} × {}", canvas.width(), canvas.height()));
        ui.label("Select a widget from the frontmost-first list to edit it.");
        return;
    };
    let widget_count = overlay.widgets().len();
    let selected_index = overlay
        .widgets()
        .iter()
        .position(|item| item.id() == widget.id());
    let can_move_forward = selected_index.is_some_and(|index| index > 0);
    let can_move_backward = selected_index.is_some_and(|index| index + 1 < widget_count);

    let mut values = TextEditorValues::from_widget(&widget);
    let original_values = values.clone();
    let mut edit_events = Vec::new();
    ui.heading("Widget inspector");
    ui.label(format!("Stable widget identity: {}", values.id));
    let mut command_changed_selection = false;
    ui.horizontal(|ui| {
        let duplicate = ui.button("Duplicate");
        #[cfg(test)]
        transient
            .control_rects
            .insert("Duplicate".to_owned(), duplicate.rect);
        if duplicate.clicked() {
            let _ = duplicate_selected_text_widget(Some(coordinator));
            command_changed_selection = true;
        }
        let delete = ui.button("Delete widget");
        #[cfg(test)]
        transient
            .control_rects
            .insert("Delete widget".to_owned(), delete.rect);
        if delete.clicked() {
            let _ = remove_selected_text_widget(Some(coordinator));
            command_changed_selection = true;
        }
    });
    ui.horizontal(|ui| {
        let forward = ui.add_enabled(can_move_forward, egui::Button::new("Forward"));
        #[cfg(test)]
        transient
            .layer_action_enabled
            .insert("Forward", forward.enabled());
        #[cfg(test)]
        transient
            .control_rects
            .insert("Forward".to_owned(), forward.rect);
        if forward.clicked() {
            let _ = move_selected_text_widget(Some(coordinator), true);
            command_changed_selection = true;
        }
        let backward = ui.add_enabled(can_move_backward, egui::Button::new("Backward"));
        #[cfg(test)]
        transient
            .layer_action_enabled
            .insert("Backward", backward.enabled());
        #[cfg(test)]
        transient
            .control_rects
            .insert("Backward".to_owned(), backward.rect);
        if backward.clicked() {
            let _ = move_selected_text_widget(Some(coordinator), false);
            command_changed_selection = true;
        }
    });
    if command_changed_selection {
        clear_inspector_state(transient);
        transient.inspector_target = coordinator
            .selected_overlay_id()
            .zip(coordinator.selected_widget_id());
        return;
    }

    ui.label("Name");
    let name_response = ui.add(
        egui::TextEdit::singleline(&mut values.name).id(egui::Id::new((
            "widget-name",
            overlay_id,
            values.id,
        ))),
    );
    #[cfg(test)]
    transient
        .control_rects
        .insert("Widget name".to_owned(), name_response.rect);
    if transient.name_focus_target == Some((overlay_id, values.id)) {
        name_response.request_focus();
        transient.focused_text_editor = Some(name_response.id);
        transient.name_focus_target = None;
    }
    #[cfg(test)]
    {
        transient.name_field_focused = name_response.has_focus();
    }
    if name_response.changed() {
        edit_events.push(EditEvent::Change(
            edit_target(overlay_id, values.id, "name"),
            EditValue::Name(values.name.clone()),
        ));
    }
    if name_response.lost_focus() {
        edit_events.push(EditEvent::Finish(
            edit_target(overlay_id, values.id, "name"),
            false,
        ));
    }
    transient.text_editor_ids.push(name_response.id);
    if name_response.has_focus() {
        transient.focused_text_editor = Some(name_response.id);
    }
    ui.label("Content");
    let _content_response = ui.add(
        egui::TextEdit::multiline(&mut values.content)
            .id(egui::Id::new(("widget-content", overlay_id, values.id)))
            .desired_rows(4)
            .desired_width(f32::INFINITY),
    );
    #[cfg(test)]
    transient
        .control_rects
        .insert("Widget content".to_owned(), _content_response.rect);
    transient.text_editor_ids.push(_content_response.id);
    if _content_response.has_focus() {
        transient.focused_text_editor = Some(_content_response.id);
    }
    if _content_response.changed() {
        edit_events.push(EditEvent::Change(
            edit_target(overlay_id, values.id, "content"),
            EditValue::Content(values.content.clone()),
        ));
    }
    if _content_response.lost_focus() {
        edit_events.push(EditEvent::Finish(
            edit_target(overlay_id, values.id, "content"),
            false,
        ));
    }
    ui.horizontal(|ui| {
        ui.label("Font family");
        egui::ComboBox::from_id_salt(("font-family", overlay_id, values.id))
            .selected_text(values.font_family.display_name())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut values.font_family,
                    FontFamily::NotoSans,
                    FontFamily::NotoSans.display_name(),
                );
                ui.selectable_value(
                    &mut values.font_family,
                    FontFamily::JetBrainsMono,
                    FontFamily::JetBrainsMono.display_name(),
                );
            });
    });
    ui.horizontal(|ui| {
        ui.label("Font size");
        let _font_size_response = ui
            .push_id(("font-size", overlay_id, values.id), |ui| {
                ui.add(
                    egui::DragValue::new(&mut values.font_size)
                        .speed(0.5)
                        .suffix(" px"),
                )
            })
            .inner;
        #[cfg(test)]
        transient
            .control_rects
            .insert("Font size".to_owned(), _font_size_response.rect);
        transient.text_editor_ids.push(_font_size_response.id);
        if _font_size_response.has_focus() {
            transient.focused_text_editor = Some(_font_size_response.id);
        }
        collect_numeric_edit_events(
            &_font_size_response,
            edit_target(overlay_id, values.id, "font_size"),
            EditValue::FontSize(values.font_size),
            &mut edit_events,
        );
    });
    ui.horizontal(|ui| {
        ui.label("Color");
        render_inspector_color_control(
            ui,
            transient,
            overlay_id,
            values.id,
            &mut values.color,
            &mut edit_events,
        );
        ui.label(format!(
            "RGBA({}, {}, {}, {})",
            values.color.red(),
            values.color.green(),
            values.color.blue(),
            values.color.alpha()
        ));
    });
    ui.horizontal(|ui| {
        ui.label("Alignment");
        let _left = ui.selectable_value(&mut values.alignment, Alignment::Left, "Left");
        let _center = ui.selectable_value(&mut values.alignment, Alignment::Center, "Center");
        let _right = ui.selectable_value(&mut values.alignment, Alignment::Right, "Right");
        #[cfg(test)]
        {
            transient
                .control_rects
                .insert("Alignment Left".to_owned(), _left.rect);
            transient
                .control_rects
                .insert("Alignment Center".to_owned(), _center.rect);
            transient
                .control_rects
                .insert("Alignment Right".to_owned(), _right.rect);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Position");
        let mut x = values.position.x();
        let mut y = values.position.y();
        ui.label("X");
        let _x_response = ui
            .push_id(("position-x", overlay_id, values.id), |ui| {
                ui.add(egui::DragValue::new(&mut x).range(0.0..=canvas.width() as f32))
            })
            .inner;
        transient.text_editor_ids.push(_x_response.id);
        if _x_response.has_focus() {
            transient.focused_text_editor = Some(_x_response.id);
        }
        ui.label("Y");
        let _y_response = ui
            .push_id(("position-y", overlay_id, values.id), |ui| {
                ui.add(egui::DragValue::new(&mut y).range(0.0..=canvas.height() as f32))
            })
            .inner;
        transient.text_editor_ids.push(_y_response.id);
        if _y_response.has_focus() {
            transient.focused_text_editor = Some(_y_response.id);
        }
        #[cfg(test)]
        {
            transient
                .control_rects
                .insert("Position X".to_owned(), _x_response.rect);
            transient
                .control_rects
                .insert("Position Y".to_owned(), _y_response.rect);
        }
        values.position = Position::new(
            x.clamp(0.0, canvas.width() as f32),
            y.clamp(0.0, canvas.height() as f32),
        );
        collect_numeric_edit_events(
            &_x_response,
            edit_target(overlay_id, values.id, "position_x"),
            EditValue::Position(Position::new(x, values.position.y())),
            &mut edit_events,
        );
        collect_numeric_edit_events(
            &_y_response,
            edit_target(overlay_id, values.id, "position_y"),
            EditValue::Position(Position::new(values.position.x(), y)),
            &mut edit_events,
        );
    });

    for event in edit_events {
        match event {
            EditEvent::Begin(target, is_gesture) => {
                if transient.active_edit.as_ref() != Some(&target) {
                    if let Err(error) = resolve_active_edit(coordinator, transient) {
                        transient.close_error = Some(error);
                        continue;
                    }
                    if let Err(error) = coordinator.begin_edit(target.clone()) {
                        transient.close_error = Some(error.to_string());
                        continue;
                    }
                    transient.active_edit = Some(target.clone());
                }
                transient.gesture_edit = is_gesture.then_some(target);
            }
            EditEvent::Change(target, value) => {
                if transient.active_edit.as_ref() != Some(&target) {
                    if let Err(error) = resolve_active_edit(coordinator, transient) {
                        transient.close_error = Some(error);
                        continue;
                    }
                    transient.active_edit = Some(target.clone());
                }
                if let Err(error) = apply_edit_value(coordinator, &target, value) {
                    transient.close_error = Some(error);
                }
            }
            EditEvent::Finish(target, cancel) => {
                if transient.active_edit.as_ref() == Some(&target) {
                    if let Err(error) = finish_edit(coordinator, &target, cancel) {
                        transient.close_error = Some(error);
                    } else {
                        transient.active_edit = None;
                        if transient.gesture_edit.as_ref() == Some(&target) {
                            transient.gesture_edit = None;
                        }
                    }
                }
            }
        }
    }
    if values.font_family != original_values.font_family {
        match resolve_active_edit(coordinator, transient) {
            Ok(()) => {
                let family = values.font_family;
                if let Err(error) = coordinator.update_overlay(overlay_id, |overlay| {
                    overlay.set_widget_font_family(values.id, family)
                }) {
                    transient.close_error = Some(error.to_string());
                }
            }
            Err(error) => transient.close_error = Some(error),
        }
    }
    if values.alignment != original_values.alignment {
        match resolve_active_edit(coordinator, transient) {
            Ok(()) => {
                let alignment = values.alignment;
                if let Err(error) = coordinator.update_overlay(overlay_id, |overlay| {
                    overlay.set_widget_alignment(values.id, alignment)
                }) {
                    transient.close_error = Some(error.to_string());
                }
            }
            Err(error) => transient.close_error = Some(error),
        }
    }
}

const PREVIEW_MIN_HANDLE: f32 = 12.0;
const PREVIEW_MAX_PAINT_FONT: f32 = 512.0;
const CHECKER_TILE_SIZE: f32 = 16.0;
const HOVER_OUTLINE_COLOR: egui::Color32 = egui::Color32::from_rgb(224, 224, 224);
const SELECTION_OUTLINE_COLOR: egui::Color32 = egui::Color32::from_rgb(38, 198, 218);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PreviewSelection {
    Widget(TextWidgetId),
    Clear,
}

fn preview_scale(
    canvas: crate::model::CanvasSize,
    available_width: f32,
    available_height: f32,
) -> f32 {
    (available_width.max(1.0) / canvas.width() as f32)
        .min(available_height.max(1.0) / canvas.height() as f32)
        .min(1.0)
}

fn canvas_to_preview(origin: egui::Pos2, position: Position, scale: f32) -> egui::Pos2 {
    origin + egui::vec2(position.x() * scale, position.y() * scale)
}

fn preview_to_canvas(
    origin: egui::Pos2,
    pointer: egui::Pos2,
    pointer_offset: egui::Vec2,
    scale: f32,
    canvas: crate::model::CanvasSize,
) -> Position {
    let top_left = pointer - pointer_offset;
    Position::new(
        ((top_left.x - origin.x) / scale).clamp(0.0, canvas.width() as f32),
        ((top_left.y - origin.y) / scale).clamp(0.0, canvas.height() as f32),
    )
}

fn alignment_to_egui(alignment: Alignment) -> egui::Align {
    match alignment {
        Alignment::Left => egui::Align::LEFT,
        Alignment::Center => egui::Align::Center,
        Alignment::Right => egui::Align::RIGHT,
    }
}

fn aligned_paint_origin(
    region_origin: egui::Pos2,
    region_width: f32,
    alignment: Alignment,
) -> egui::Pos2 {
    let offset = match alignment {
        Alignment::Left => 0.0,
        Alignment::Center => region_width / 2.0,
        Alignment::Right => region_width,
    };
    region_origin + egui::vec2(offset, 0.0)
}

fn text_region_rect(
    canvas_rect: egui::Rect,
    position: Position,
    scale: f32,
    text_height: f32,
) -> egui::Rect {
    let origin = canvas_to_preview(canvas_rect.min, position, scale);
    egui::Rect::from_min_max(
        origin,
        egui::pos2(canvas_rect.right(), origin.y + text_height),
    )
}

fn widget_hitbox(
    canvas_rect: egui::Rect,
    widget_origin: egui::Pos2,
    visual_rect: egui::Rect,
) -> egui::Rect {
    let fallback = egui::Rect::from_min_size(widget_origin, egui::Vec2::splat(PREVIEW_MIN_HANDLE));
    let hitbox = if visual_rect.is_positive() {
        visual_rect
    } else {
        fallback
    };
    // Empty text and text fully clipped at the canvas edge have no glyph bounds,
    // so the editor exposes a small handle at the model's top-left position.
    hitbox.intersect(canvas_rect)
}

fn drag_matches(drag: Option<PreviewDrag>, overlay_id: OverlayId, widget_id: TextWidgetId) -> bool {
    drag.is_some_and(|drag| drag.overlay_id == overlay_id && drag.widget_id == widget_id)
}

fn select_preview_widget(
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
    widget_id: TextWidgetId,
) -> bool {
    match coordinator.select_widget(widget_id) {
        Ok(()) => {
            transient.preview_error = None;
            true
        }
        Err(error) => {
            transient.preview_drag = None;
            transient.preview_error = Some(error.to_string());
            false
        }
    }
}

fn begin_preview_drag(
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
    drag: PreviewDrag,
) -> Result<(), String> {
    let target = edit_target(drag.overlay_id, drag.widget_id, "canvas_position");
    coordinator
        .begin_edit(target.clone())
        .map_err(|error| error.to_string())?;
    transient.active_edit = Some(target);
    transient.preview_error = None;
    Ok(())
}

fn update_preview_drag(
    coordinator: &mut HeadlessCoordinator,
    drag: PreviewDrag,
    position: Position,
) -> Result<(), String> {
    let target = edit_target(drag.overlay_id, drag.widget_id, "canvas_position");
    coordinator
        .update_edit(&target, |overlay| {
            overlay.set_widget_position(drag.widget_id, position)
        })
        .map_err(|error| error.to_string())
}

fn render_collection_preview(
    ui: &mut egui::Ui,
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
    overlay_id: OverlayId,
) {
    let Some(overlay) = coordinator.overlay(overlay_id) else {
        clear_inspector_state(transient);
        return;
    };
    let canvas = overlay.canvas();
    let widgets = overlay.widgets().to_vec();
    let selected_widget_id = coordinator.selected_widget_id();
    let (selection, moved, stopped) = render_canvas_preview(
        ui,
        canvas,
        overlay_id,
        &widgets,
        selected_widget_id,
        &mut transient.preview_drag,
        #[cfg(test)]
        &mut transient.control_rects,
        #[cfg(test)]
        &mut transient.preview_rect,
    );
    match selection {
        Some(PreviewSelection::Widget(widget_id)) => {
            if select_preview_widget(coordinator, transient, widget_id) {
                if !drag_matches(transient.preview_drag, overlay_id, widget_id) {
                    if let Some(drag) = transient.preview_drag {
                        if let Err(error) = commit_preview_drag(coordinator, transient, drag) {
                            transient.preview_error = Some(error);
                        }
                    }
                }
                transient.inspector_target = Some((overlay_id, widget_id));
                transient.name_focus_target = None;
                transient.pending_widget_reveal = Some((overlay_id, widget_id));
            }
        }
        Some(PreviewSelection::Clear) => {
            coordinator.clear_widget_selection();
            transient.preview_error = None;
            clear_inspector_state(transient);
            transient.inspector_target = None;
        }
        None => {}
    }
    if selection.is_some() {
        // Selection is resolved after the inspector panel has rendered, so
        // repaint once to bring its contents and the canvas outline up to date.
        ui.ctx().request_repaint();
    }
    if let Some(drag) = transient
        .preview_drag
        .filter(|drag| drag.overlay_id == overlay_id)
    {
        let target = edit_target(overlay_id, drag.widget_id, "canvas_position");
        if coordinator.pending_edit_target() != Some(&target)
            && let Err(error) = begin_preview_drag(coordinator, transient, drag)
        {
            transient.preview_error = Some(error);
        }
    }
    if let Some((_, position)) = moved
        && let Some(drag) = transient.preview_drag
    {
        match update_preview_drag(coordinator, drag, position) {
            Ok(()) => transient.preview_error = None,
            Err(error) => transient.preview_error = Some(error),
        }
    }
    let escape_pressed = ui.input(|input| input.key_pressed(egui::Key::Escape));
    if stopped
        && !escape_pressed
        && let Some(drag) = transient.preview_drag
    {
        match commit_preview_drag(coordinator, transient, drag) {
            Ok(()) => transient.preview_error = None,
            Err(error) => transient.preview_error = Some(error),
        }
    }
}

fn commit_preview_drag(
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
    drag: PreviewDrag,
) -> Result<(), String> {
    let target = edit_target(drag.overlay_id, drag.widget_id, "canvas_position");
    coordinator
        .commit_edit(&target)
        .map_err(|error| error.to_string())?;
    transient.preview_drag = None;
    transient.active_edit = None;
    Ok(())
}

fn render_canvas_preview(
    ui: &mut egui::Ui,
    canvas: crate::model::CanvasSize,
    overlay_id: OverlayId,
    widgets: &[TextWidget],
    selected_widget_id: Option<TextWidgetId>,
    drag: &mut Option<PreviewDrag>,
    #[cfg(test)] control_rects: &mut HashMap<String, egui::Rect>,
    #[cfg(test)] preview_rect: &mut Option<egui::Rect>,
) -> (
    Option<PreviewSelection>,
    Option<(TextWidgetId, Position)>,
    bool,
) {
    let scale = preview_scale(canvas, ui.available_width(), ui.available_height());
    let size = egui::vec2(
        canvas.width() as f32 * scale,
        canvas.height() as f32 * scale,
    );
    let (canvas_rect, canvas_response) = ui.allocate_exact_size(size, egui::Sense::click());
    #[cfg(test)]
    {
        *preview_rect = Some(canvas_rect);
    }
    let painter = ui.painter_at(canvas_rect);
    let columns = (canvas_rect.width() / CHECKER_TILE_SIZE).ceil() as usize;
    let rows = (canvas_rect.height() / CHECKER_TILE_SIZE).ceil() as usize;
    for row in 0..rows {
        for column in 0..columns {
            let tile = egui::Rect::from_min_size(
                canvas_rect.min
                    + egui::vec2(
                        column as f32 * CHECKER_TILE_SIZE,
                        row as f32 * CHECKER_TILE_SIZE,
                    ),
                egui::vec2(CHECKER_TILE_SIZE, CHECKER_TILE_SIZE),
            )
            .intersect(canvas_rect);
            let gray = if (row + column) % 2 == 0 { 112 } else { 122 };
            painter.rect_filled(tile, 0.0, egui::Color32::from_gray(gray));
        }
    }
    painter.rect_stroke(
        canvas_rect,
        0.0,
        egui::Stroke::new(1.0_f32, egui::Color32::from_gray(96)),
    );

    let mut moved = None;
    let mut selection = None;
    let mut pressed_widget = None;
    let mut stopped = false;
    let mut hovered_hitboxes = Vec::new();
    let mut selected_hitbox = None;
    let pointer_pressed =
        ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary));
    let pointer_position = ui.input(|input| input.pointer.interact_pos());
    let pointer_button_down =
        ui.input(|input| input.pointer.button_down(egui::PointerButton::Primary));
    for widget in widgets.iter().rev() {
        let color = egui::Color32::from_rgba_unmultiplied(
            widget.color().red(),
            widget.color().green(),
            widget.color().blue(),
            widget.color().alpha(),
        );
        let region_width = ((canvas.width() as f32 - widget.position().x()) * scale).max(0.0);
        // Capping paint size protects the editor from huge but model-valid values;
        // it is deliberately local and never written back to the authoritative model.
        let paint_font_size = (widget.font_size() * scale).clamp(1.0, PREVIEW_MAX_PAINT_FONT);
        let font_id = match widget.font_family() {
            FontFamily::NotoSans => egui::FontId::proportional(paint_font_size),
            FontFamily::JetBrainsMono => egui::FontId::monospace(paint_font_size),
        };
        let mut layout = egui::text::LayoutJob::simple(
            widget.content().to_owned(),
            font_id,
            color,
            region_width,
        );
        layout.halign = alignment_to_egui(widget.alignment());
        layout.wrap.max_width = region_width;
        let galley = painter.layout_job(layout);
        let region_origin = canvas_to_preview(canvas_rect.min, widget.position(), scale);
        let paint_origin = aligned_paint_origin(region_origin, region_width, widget.alignment());
        painter.galley(paint_origin, galley.clone(), color);

        let region = text_region_rect(canvas_rect, widget.position(), scale, galley.size().y);
        let visual_rect = galley
            .mesh_bounds
            .translate(paint_origin.to_vec2())
            .intersect(region);
        let hitbox = widget_hitbox(canvas_rect, region_origin, visual_rect);
        #[cfg(test)]
        control_rects.insert(format!("Canvas widget {}", widget.id()), hitbox);
        let response = ui.interact(
            hitbox,
            ui.make_persistent_id(("preview-text", overlay_id, widget.id())),
            egui::Sense::click_and_drag(),
        );
        if selected_widget_id == Some(widget.id()) {
            #[cfg(test)]
            control_rects.insert("Canvas preview".to_owned(), hitbox);
            selected_hitbox = Some(hitbox);
        }
        if response.hovered() {
            hovered_hitboxes.push(hitbox);
        }
        if pointer_pressed
            && drag.is_none()
            && response.is_pointer_button_down_on()
            && let Some(pointer) = pointer_position
            && hitbox.contains(pointer)
        {
            pressed_widget = Some((widget.id(), region_origin, pointer));
        }
        // The press frame already selects/reveals this widget; avoid queuing
        // the same selection a second time on the release frame.
        if response.clicked() && !drag_matches(*drag, overlay_id, widget.id()) {
            selection = Some(PreviewSelection::Widget(widget.id()));
        }
        if response.dragged()
            && let (Some(active), Some(pointer)) = (*drag, response.interact_pointer_pos())
            && drag_matches(Some(active), overlay_id, widget.id())
        {
            moved = Some((
                widget.id(),
                preview_to_canvas(
                    canvas_rect.min,
                    pointer,
                    active.pointer_offset,
                    scale,
                    canvas,
                ),
            ));
        }
        if drag_matches(*drag, overlay_id, widget.id())
            && (response.drag_stopped() || !pointer_button_down)
        {
            stopped = true;
        }
    }
    // Paint order is back-to-front, so the last active hit is frontmost.
    if let Some((widget_id, widget_origin, pointer)) = pressed_widget {
        selection = Some(PreviewSelection::Widget(widget_id));
        let start_position = widgets
            .iter()
            .find(|widget| widget.id() == widget_id)
            .unwrap()
            .position();
        *drag = Some(PreviewDrag {
            overlay_id,
            widget_id,
            pointer_offset: pointer - widget_origin,
            start_position,
        });
    }
    if selection.is_none() && canvas_response.clicked() {
        selection = Some(PreviewSelection::Clear);
    }
    for hitbox in hovered_hitboxes {
        painter.rect_stroke(hitbox, 0.0, egui::Stroke::new(1.0, HOVER_OUTLINE_COLOR));
    }
    if let Some(hitbox) = selected_hitbox {
        painter.rect_stroke(hitbox, 0.0, egui::Stroke::new(2.0, SELECTION_OUTLINE_COLOR));
    }
    (selection, moved, stopped)
}

fn begin_create(transient: &mut TransientState) {
    transient.create_open = true;
    transient.dialog_error = None;
    transient.create_name.clear();
    transient.create_width.clear();
    transient.create_height.clear();
}

fn create_overlay_from_form(
    coordinator: Option<&mut HeadlessCoordinator>,
    transient: &mut TransientState,
) -> Result<(), String> {
    let width = transient.create_width.trim().parse::<u32>();
    let height = transient.create_height.trim().parse::<u32>();
    match (width, height) {
        (Ok(width), Ok(height)) if width > 0 && height > 0 => {
            coordinator
                .ok_or_else(|| "workspace is not available".to_owned())?
                .create_overlay(transient.create_name.trim().to_owned(), width, height)
                .map_err(|error| error.to_string())?;
            transient.create_open = false;
            transient.dialog_error = None;
            transient.inspector_target = None;
            transient.preview_drag = None;
            Ok(())
        }
        _ => {
            let error = "Canvas width and height must be positive whole numbers.".to_owned();
            transient.dialog_error = Some(error.clone());
            Err(error)
        }
    }
}

#[cfg(test)]
fn cancel_dialogs(transient: &mut TransientState) {
    transient.create_open = false;
    transient.rename_open = false;
    transient.delete_target = None;
    transient.dialog_error = None;
}

fn begin_rename(
    coordinator: &HeadlessCoordinator,
    transient: &mut TransientState,
) -> Result<(), String> {
    let (id, name) = coordinator
        .selected_overlay()
        .map(|overlay| (overlay.id(), overlay.name().to_owned()))
        .ok_or_else(|| "no overlay is selected".to_owned())?;
    transient.rename_open = true;
    transient.rename_id = Some(id);
    transient.rename_name = name;
    transient.dialog_error = None;
    Ok(())
}

fn apply_rename(
    coordinator: Option<&mut HeadlessCoordinator>,
    transient: &mut TransientState,
) -> Result<(), String> {
    let id = transient
        .rename_id
        .ok_or_else(|| "no rename target is active".to_owned())?;
    coordinator
        .ok_or_else(|| "workspace is not available".to_owned())?
        .rename_overlay(id, transient.rename_name.trim().to_owned())
        .map_err(|error| error.to_string())?;
    transient.rename_open = false;
    transient.dialog_error = None;
    Ok(())
}

fn begin_delete(
    coordinator: &HeadlessCoordinator,
    transient: &mut TransientState,
) -> Result<(), String> {
    let id = coordinator
        .selected_overlay_id()
        .ok_or_else(|| "no overlay is selected".to_owned())?;
    transient.delete_target = Some(id);
    transient.dialog_error = None;
    Ok(())
}

fn confirm_delete(
    coordinator: Option<&mut HeadlessCoordinator>,
    transient: &mut TransientState,
) -> Result<(), String> {
    let id = transient
        .delete_target
        .ok_or_else(|| "no delete target is active".to_owned())?;
    coordinator
        .ok_or_else(|| "workspace is not available".to_owned())?
        .delete_overlay(id, true)
        .map_err(|error| error.to_string())?;
    transient.delete_target = None;
    transient.dialog_error = None;
    transient.inspector_target = None;
    transient.preview_drag = None;
    Ok(())
}

fn parse_port_input(input: &str) -> Result<u16, String> {
    let value = input
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("Port must be a whole number from {MIN_PORT} to {MAX_PORT}."))?;
    if !(u32::from(MIN_PORT)..=u32::from(MAX_PORT)).contains(&value) {
        return Err(format!(
            "Port must be a whole number from {MIN_PORT} to {MAX_PORT}."
        ));
    }
    u16::try_from(value)
        .map_err(|_| format!("Port must be a whole number from {MIN_PORT} to {MAX_PORT}."))
}

fn save_port_for_next_launch(
    settings: &mut SettingsState,
    transient: &mut TransientState,
) -> Result<(), String> {
    transient.settings_save_error = None;
    transient.settings_save_succeeded = false;
    let port = match parse_port_input(&transient.settings_port_input) {
        Ok(port) => port,
        Err(error) => {
            transient.settings_save_error = Some(error.clone());
            return Err(error);
        }
    };

    match settings.save_port_for_next_launch(port) {
        Ok(()) => {
            transient.settings_save_succeeded = true;
            Ok(())
        }
        Err(error) => {
            let error = format!("Could not save port for next launch: {error}");
            transient.settings_save_error = Some(error.clone());
            Err(error)
        }
    }
}

fn menu_label(action: &str, shortcut: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("{action}\t⌘{shortcut}")
    } else {
        format!("{action}\tCtrl+{shortcut}")
    }
}

fn request_close(
    context: &egui::Context,
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
) -> Result<(), String> {
    // Veto the native request before any fallible transaction work.
    context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    if let Err(error) = resolve_active_edit(coordinator, transient) {
        if !transient.close_prompt_open {
            transient.close_prompt_open = true;
            #[cfg(test)]
            {
                transient.close_prompt_count += 1;
            }
        }
        transient.close_error = Some(error.clone());
        return Err(error);
    }
    if coordinator.is_dirty() {
        if !transient.close_prompt_open {
            transient.close_prompt_open = true;
            #[cfg(test)]
            {
                transient.close_prompt_count += 1;
            }
        }
    } else {
        allow_close(context, transient);
    }
    Ok(())
}

fn allow_close(context: &egui::Context, transient: &mut TransientState) {
    transient.close_prompt_open = false;
    transient.allow_close = true;
    context.send_viewport_cmd(egui::ViewportCommand::Close);
}

fn render_close_prompt(
    context: &egui::Context,
    transient: &mut TransientState,
    actions: &mut Vec<DeferredAction>,
) {
    if !transient.close_prompt_open {
        return;
    }
    egui::Window::new("Unsaved changes")
        .id(egui::Id::new("close-confirmation"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(context, |ui| {
            ui.label("Save your changes before closing?");
            ui.horizontal(|ui| {
                let save = ui.button("Save and close");
                #[cfg(test)]
                transient
                    .control_rects
                    .insert("Close/Save".to_owned(), save.rect);
                if save.clicked() {
                    actions.push(DeferredAction::CloseSave);
                }
                let discard = ui.button("Discard");
                #[cfg(test)]
                transient
                    .control_rects
                    .insert("Close/Discard".to_owned(), discard.rect);
                if discard.clicked() {
                    actions.push(DeferredAction::CloseDiscard);
                }
                let cancel = ui.button("Cancel");
                #[cfg(test)]
                transient
                    .control_rects
                    .insert("Close/Cancel".to_owned(), cancel.rect);
                if cancel.clicked() {
                    actions.push(DeferredAction::CloseCancel);
                }
            });
            if let Some(error) = transient.close_error.as_deref() {
                ui.colored_label(egui::Color32::from_rgb(239, 83, 80), error);
            }
        });
}

fn save_workspace(coordinator: Option<&mut HeadlessCoordinator>) -> Result<(), String> {
    coordinator
        .ok_or_else(|| "workspace is not available".to_owned())?
        .save()
        .map_err(|error| error.to_string())
}

fn copy_url(context: &egui::Context, url: &str) {
    context.copy_text(url.to_owned());
}

fn open_url(context: &egui::Context, url: &str) {
    context.open_url(egui::OpenUrl::same_tab(url));
}

#[cfg(test)]
fn copy_selected_url(
    context: &egui::Context,
    coordinator: Option<&HeadlessCoordinator>,
) -> Result<(), String> {
    let url = coordinator
        .ok_or_else(|| "workspace is not available".to_owned())?
        .selected_url()
        .ok_or_else(|| "browser-source URL is unavailable".to_owned())?;
    copy_url(context, &url);
    Ok(())
}

#[cfg(test)]
fn open_selected_url(
    context: &egui::Context,
    coordinator: Option<&HeadlessCoordinator>,
) -> Result<(), String> {
    let url = coordinator
        .ok_or_else(|| "workspace is not available".to_owned())?
        .selected_url()
        .ok_or_else(|| "browser-source URL is unavailable".to_owned())?;
    open_url(context, &url);
    Ok(())
}

fn render_settings(
    ui: &mut egui::Ui,
    settings: &mut SettingsState,
    transient: &mut TransientState,
    active_port: Option<u16>,
) {
    let configured_port = settings.configured_port();
    let settings_path = settings
        .settings_path()
        .map(|path| path.display().to_string());
    ui.horizontal_wrapped(|ui| {
        if let Some(port) = configured_port {
            ui.label(format!("Current configured port: {port}"));
            ui.label(format!("Next-launch port: {port}"));
        } else {
            ui.label("Current configured port: unavailable");
            ui.label("Next-launch port: unavailable");
            ui.label(format!(
                "Display-only default: {} (not used while settings are invalid).",
                settings.display_port()
            ));
        }
        if let Some(path) = settings_path.as_deref() {
            ui.label(format!("Settings path: {path}"));
        } else {
            ui.label("Settings path: unavailable");
        }
        if let Some(active_port) = active_port {
            ui.label(format!(
                "Running server remains on port {active_port} until restart."
            ));
        }
    });
    if settings.settings_error().is_some() {
        ui.label(
            "First copy the exact settings source to a separate backup location. Then repair it or move it aside yourself, and restart Chikachika.",
        );
        ui.label("No fallback port is used while settings are invalid.");
    }
    ui.horizontal(|ui| {
        ui.label("Port for next launch (1–65535)");
        let input = ui.text_edit_singleline(&mut transient.settings_port_input);
        #[cfg(test)]
        transient
            .control_rects
            .insert("Settings port".to_owned(), input.rect);
        transient.text_editor_ids.push(input.id);
        if input.has_focus() {
            transient.focused_text_editor = Some(input.id);
        }
        if input.changed() {
            transient.settings_save_error = None;
            transient.settings_save_succeeded = false;
        }
        let save = ui.button("Save port for next launch");
        #[cfg(test)]
        transient
            .control_rects
            .insert("Save port for next launch".to_owned(), save.rect);
        if save.clicked()
            && let Err(error) = save_port_for_next_launch(settings, transient)
        {
            transient.settings_save_error = Some(error);
        }
    });
    if let Some(error) = transient.settings_save_error.as_deref() {
        ui.label(
            "Check the settings path and permissions, then try again. The previous configured port remains unchanged.",
        );
        ui.colored_label(egui::Color32::from_rgb(239, 83, 80), error);
    }
    if transient.settings_save_succeeded {
        ui.colored_label(
            egui::Color32::from_rgb(129, 199, 132),
            "Port saved for next launch. Changes take effect after restarting Chikachika.",
        );
    } else {
        ui.label("Port changes take effect only after restarting Chikachika.");
    }
}

#[cfg(test)]
fn append_settings_labels(
    visible: &mut Vec<String>,
    settings: &SettingsState,
    transient: &TransientState,
    active_port: Option<u16>,
) {
    visible.push("Local server settings".to_owned());
    if !transient.settings_expanded {
        if let Some(error) = settings.settings_error() {
            visible.push(format!("Settings error: {error}"));
        }
        if let Some(error) = transient.settings_save_error.as_deref() {
            visible.push(format!("Settings save error: {error}"));
            visible.push("The previous configured port remains unchanged.".to_owned());
        }
        return;
    }
    if let Some(port) = settings.configured_port() {
        visible.push(format!("Current configured port: {port}"));
        visible.push(format!("Next-launch port: {port}"));
    } else {
        visible.extend([
            "Current configured port: unavailable".to_owned(),
            "Next-launch port: unavailable".to_owned(),
            format!(
                "Display-only default: {} (not used while settings are invalid).",
                settings.display_port()
            ),
        ]);
    }
    if let Some(path) = settings.settings_path() {
        visible.push(format!("Settings path: {}", path.display()));
    } else {
        visible.push("Settings path: unavailable".to_owned());
    }
    if let Some(active_port) = active_port {
        visible.push(format!(
            "Running server remains on port {active_port} until restart."
        ));
    }
    if let Some(error) = settings.settings_error() {
        visible.push(format!("Settings error: {error}"));
        visible.extend([
            "First copy the exact settings source to a separate backup location. Then repair it or move it aside yourself, and restart Chikachika.".to_owned(),
            "No fallback port is used while settings are invalid.".to_owned(),
        ]);
    }
    visible.extend([
        "Port for next launch (1–65535)".to_owned(),
        "Save port for next launch".to_owned(),
    ]);
    if let Some(error) = transient.settings_save_error.as_deref() {
        visible.push(error.to_owned());
        visible.push(
            "Check the settings path and permissions, then try again. The previous configured port remains unchanged.".to_owned(),
        );
    }
    if transient.settings_save_succeeded {
        visible.push(
            "Port saved for next launch. Changes take effect after restarting Chikachika."
                .to_owned(),
        );
    } else {
        visible.push("Port changes take effect only after restarting Chikachika.".to_owned());
    }
}

#[cfg(test)]
fn select_named(coordinator: Option<&mut HeadlessCoordinator>, name: &str) -> Result<(), String> {
    let coordinator = coordinator.ok_or_else(|| "workspace is not available".to_owned())?;
    let id = coordinator
        .overlays()
        .iter()
        .find(|overlay| overlay.name() == name)
        .map(|overlay| overlay.id())
        .ok_or_else(|| format!("unknown control: {name}"))?;
    select_overlay(coordinator, id)
}

fn select_overlay(coordinator: &mut HeadlessCoordinator, id: OverlayId) -> Result<(), String> {
    coordinator
        .select_overlay(id)
        .map_err(|error| error.to_string())
}

fn render_create_dialog(
    context: &egui::Context,
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
) {
    if !transient.create_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Create overlay")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(context, |ui| {
            ui.label("Name");
            let name = ui.text_edit_singleline(&mut transient.create_name);
            #[cfg(test)]
            transient
                .control_rects
                .insert("Create name".to_owned(), name.rect);
            transient.text_editor_ids.push(name.id);
            if name.has_focus() {
                transient.focused_text_editor = Some(name.id);
            }
            ui.label("Fixed canvas width");
            let width = ui.text_edit_singleline(&mut transient.create_width);
            #[cfg(test)]
            transient
                .control_rects
                .insert("Create width".to_owned(), width.rect);
            transient.text_editor_ids.push(width.id);
            ui.label("Fixed canvas height");
            let height = ui.text_edit_singleline(&mut transient.create_height);
            #[cfg(test)]
            transient
                .control_rects
                .insert("Create height".to_owned(), height.rect);
            transient.text_editor_ids.push(height.id);
            if let Some(error) = transient.dialog_error.as_deref() {
                ui.colored_label(egui::Color32::from_rgb(183, 28, 28), error);
            }
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    transient.create_open = false;
                    transient.dialog_error = None;
                }
                if ui.button("Create").clicked()
                    && let Err(error) = create_overlay_from_form(Some(coordinator), transient)
                {
                    transient.dialog_error = Some(error);
                }
            });
        });
    if !open {
        transient.create_open = false;
    }
}

fn render_rename_dialog(
    context: &egui::Context,
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
) {
    if !transient.rename_open {
        return;
    }
    let Some(id) = transient.rename_id else {
        transient.rename_open = false;
        return;
    };
    let mut open = true;
    egui::Window::new("Rename overlay")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(context, |ui| {
            ui.label(format!("Rename overlay {id}"));
            let name = ui.text_edit_singleline(&mut transient.rename_name);
            #[cfg(test)]
            transient
                .control_rects
                .insert("Rename name".to_owned(), name.rect);
            transient.text_editor_ids.push(name.id);
            if let Some(error) = transient.dialog_error.as_deref() {
                ui.colored_label(egui::Color32::from_rgb(183, 28, 28), error);
            }
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    transient.rename_open = false;
                    transient.dialog_error = None;
                }
                if ui.button("Apply rename").clicked()
                    && let Err(error) = apply_rename(Some(coordinator), transient)
                {
                    transient.dialog_error = Some(error);
                }
            });
        });
    if !open {
        transient.rename_open = false;
    }
}

fn render_delete_dialog(
    context: &egui::Context,
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
) {
    let Some(id) = transient.delete_target else {
        return;
    };
    let Some(name) = coordinator
        .overlay(id)
        .map(|overlay| overlay.name().to_owned())
    else {
        transient.delete_target = None;
        transient.preview_drag = None;
        transient.inspector_target = None;
        return;
    };
    let mut open = true;
    egui::Window::new("Confirm deletion")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(context, |ui| {
            ui.label(format!("Delete '{name}'?"));
            ui.label(format!("Target identity: {id}"));
            ui.label("This removes the live browser route and will be persisted only after Save.");
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    transient.delete_target = None;
                }
                if ui.button("Confirm delete").clicked()
                    && let Err(error) = confirm_delete(Some(coordinator), transient)
                {
                    transient.dialog_error = Some(error);
                }
            });
            if let Some(error) = transient.dialog_error.as_deref() {
                ui.colored_label(egui::Color32::from_rgb(183, 28, 28), error);
            }
        });
    if !open {
        transient.delete_target = None;
    }
}

/// A native-window-free egui scenario harness.
///
/// Unlike the old semantic-only helper, this harness drives actual egui input,
/// retains the emitted shape list, and exposes the rectangles produced for
/// stable widget selector IDs. It remains deterministic and does not claim to
/// replace a pixel-level native GUI smoke test.
#[cfg(test)]
pub struct ScenarioHarness {
    context: egui::Context,
    app: ChikachikaApp,
    last_copied_text: String,
    last_open_url: Option<egui::OpenUrl>,
    last_shapes: Vec<egui::epaint::ClippedShape>,
    last_viewport_commands: Vec<egui::ViewportCommand>,
    screen_size: egui::Vec2,
}

#[cfg(test)]
impl ScenarioHarness {
    /// Creates a harness from deterministic startup state.
    pub fn new(outcome: BootstrapOutcome) -> Self {
        Self::new_with_size(
            outcome,
            egui::vec2(WINDOW_INITIAL_SIZE[0], WINDOW_INITIAL_SIZE[1]),
        )
    }

    /// Creates a harness from deterministic startup state and settings.
    pub fn new_with_settings(outcome: BootstrapOutcome, settings: SettingsState) -> Self {
        Self::new_with_size_and_settings(
            outcome,
            settings,
            egui::vec2(WINDOW_INITIAL_SIZE[0], WINDOW_INITIAL_SIZE[1]),
        )
    }

    fn new_with_size(outcome: BootstrapOutcome, screen_size: egui::Vec2) -> Self {
        Self::new_with_size_and_settings(
            outcome,
            SettingsState::from_settings(SettingsStore::at("settings.json"), Settings::default()),
            screen_size,
        )
    }

    fn new_with_size_and_settings(
        outcome: BootstrapOutcome,
        settings: SettingsState,
        screen_size: egui::Vec2,
    ) -> Self {
        Self {
            context: egui::Context::default(),
            app: ChikachikaApp::from_application_bootstrap(ApplicationBootstrap::new(
                outcome, settings,
            )),
            last_copied_text: String::new(),
            last_open_url: None,
            last_shapes: Vec::new(),
            last_viewport_commands: Vec::new(),
            screen_size,
        }
    }

    fn raw_input(&self, events: Vec<egui::Event>) -> egui::RawInput {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                self.screen_size,
            )),
            events,
            ..Default::default()
        };
        input.viewports.entry(egui::ViewportId::ROOT).or_default();
        input
    }

    fn frame_close_requested(&mut self) {
        self.frame_with_close(Vec::new());
    }

    fn frame_with_close(&mut self, events: Vec<egui::Event>) {
        let mut input = self.raw_input(events);
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events = vec![egui::ViewportEvent::Close];
        self.run_input(input);
    }

    fn emitted_close_commands(&self) -> &[egui::ViewportCommand] {
        &self.last_viewport_commands
    }

    fn close_prompt_count(&self) -> usize {
        self.app.transient.close_prompt_count
    }

    /// Advances one egui frame with no input events.
    pub fn frame(&mut self) {
        self.frame_events(Vec::new());
    }

    fn frame_events(&mut self, events: Vec<egui::Event>) {
        let input = self.raw_input(events);
        self.run_input(input);
    }

    fn run_input(&mut self, input: egui::RawInput) {
        #[cfg(test)]
        {
            self.app.transient.widget_selector_rects.clear();
            self.app.transient.overlay_selector_rects.clear();
            self.app.transient.control_rects.clear();
            self.app.transient.layer_action_enabled.clear();
            self.app.transient.menu_action_enabled.clear();
            self.app.transient.preview_rect = None;
        }
        let app = &mut self.app;
        let output = self.context.run(input, |context| app.render(context));
        self.last_copied_text = output.platform_output.copied_text;
        self.last_open_url = output.platform_output.open_url;
        self.last_shapes = output.shapes;
        self.last_viewport_commands = output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .clone();
    }

    /// Sends one real egui event and renders the resulting frame.
    pub fn event(&mut self, event: egui::Event) {
        self.frame_events(vec![event]);
    }

    /// Sends a real pointer-move event.
    pub fn pointer_move(&mut self, position: egui::Pos2) {
        self.event(egui::Event::PointerMoved(position));
    }

    /// Sends a real pointer button event.
    pub fn pointer_button(&mut self, position: egui::Pos2, pressed: bool) {
        self.event(egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        });
    }

    /// Sends a complete real pointer click sequence at a screen coordinate.
    pub fn pointer_click(&mut self, position: egui::Pos2) {
        self.pointer_move(position);
        self.pointer_button(position, true);
        self.pointer_button(position, false);
    }

    fn pointer_click_in_single_frame(&mut self, position: egui::Pos2) {
        self.frame_events(vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ]);
    }

    /// Sends a real egui key event.
    pub fn key(&mut self, key: egui::Key, pressed: bool) {
        self.key_with_modifiers(key, pressed, egui::Modifiers::default());
    }

    /// Sends a real egui key event with explicit modifiers.
    pub fn key_with_modifiers(
        &mut self,
        key: egui::Key,
        pressed: bool,
        modifiers: egui::Modifiers,
    ) {
        self.event(egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        });
    }

    /// Clicks a rendered control using its actual rectangle.
    pub fn pointer_click_control(&mut self, label: &str) -> Result<(), String> {
        let initial_rect = self
            .control_rect(label)
            .ok_or_else(|| format!("control rectangle is unavailable: {label}"))?;
        self.pointer_move(initial_rect.center());
        let rect = self.control_rect(label).unwrap_or(initial_rect);
        self.pointer_button(rect.center(), true);
        self.pointer_button(rect.center(), false);
        Ok(())
    }

    /// Replaces a rendered text control using focus, select-all, and text input events.
    pub fn replace_text_control(&mut self, label: &str, text: &str) -> Result<(), String> {
        self.pointer_click_control(label)?;
        self.key_with_modifiers(egui::Key::A, true, egui::Modifiers::COMMAND);
        self.event(egui::Event::Text(text.to_owned()));
        Ok(())
    }

    /// Replaces a rendered numeric control using its real focused text editor.
    pub fn replace_number_control(&mut self, label: &str, text: &str) -> Result<(), String> {
        self.pointer_click_control(label)?;
        self.key_with_modifiers(egui::Key::A, true, egui::Modifiers::COMMAND);
        self.event(egui::Event::Text(text.to_owned()));
        self.key(egui::Key::Enter, true);
        Ok(())
    }

    /// Returns the actual shapes emitted by the last frame.
    pub fn shapes(&self) -> &[egui::epaint::ClippedShape] {
        &self.last_shapes
    }

    /// Returns the current adapter state for semantic assertions.
    pub fn app(&self) -> &ChikachikaApp {
        &self.app
    }

    /// Returns the application settings for deterministic assertions.
    pub fn settings(&self) -> &SettingsState {
        self.app.settings()
    }

    /// Returns mutable adapter state for deterministic input setup.
    pub fn app_mut(&mut self) -> &mut ChikachikaApp {
        &mut self.app
    }

    /// Returns the stable-ID row rectangle emitted by the last frame.
    pub fn widget_selector_rect(&self, widget_id: TextWidgetId) -> Option<egui::Rect> {
        self.app
            .transient
            .widget_selector_rects
            .get(&widget_id)
            .copied()
    }

    /// Returns the current editor hitbox for a stable widget ID.
    pub fn widget_canvas_rect(&self, widget_id: TextWidgetId) -> Option<egui::Rect> {
        self.app
            .transient
            .control_rects
            .get(&format!("Canvas widget {widget_id}"))
            .copied()
    }

    /// Returns the overlay option rectangle emitted by the open selector.
    pub fn overlay_selector_rect(&self, overlay_id: OverlayId) -> Option<egui::Rect> {
        self.app
            .transient
            .overlay_selector_rects
            .get(&overlay_id)
            .copied()
    }

    /// Returns a named control rectangle emitted by the last frame.
    pub fn control_rect(&self, label: &str) -> Option<egui::Rect> {
        self.app.transient.control_rects.get(label).copied()
    }

    fn layer_action_is_enabled(&self, label: &'static str) -> Option<bool> {
        self.app.transient.layer_action_enabled.get(label).copied()
    }

    fn menu_action_is_enabled(&self, label: &'static str) -> Option<bool> {
        self.app.transient.menu_action_enabled.get(label).copied()
    }

    /// Activates a semantic adapter action by exact label and renders the next
    /// frame. Pointer/key-oriented scenarios should use the event methods above.
    pub fn click(&mut self, label: &str) -> Result<(), String> {
        match label {
            "Copy URL" => copy_selected_url(&self.context, self.app.coordinator.as_ref())?,
            "Open in browser" | "Open output" => {
                open_selected_url(&self.context, self.app.coordinator.as_ref())?
            }
            _ => self.app.activate(label)?,
        }
        self.frame();
        Ok(())
    }

    /// Returns the clipboard text emitted by the most recent semantic frame.
    pub fn copied_text(&self) -> &str {
        &self.last_copied_text
    }

    /// Returns the browser URL emitted by the most recent semantic frame.
    pub fn opened_url(&self) -> Option<&str> {
        self.last_open_url
            .as_ref()
            .map(|open_url| open_url.url.as_str())
    }

    /// Returns whether the most recent browser-opening output requested a new
    /// tab rather than the current tab.
    pub fn opens_in_new_tab(&self) -> Option<bool> {
        self.last_open_url.as_ref().map(|open_url| open_url.new_tab)
    }

    /// Applies deterministic form values used by adapter scenarios.
    pub fn set_create_fields(&mut self, name: &str, width: &str, height: &str) {
        self.app.transient.create_name = name.to_owned();
        self.app.transient.create_width = width.to_owned();
        self.app.transient.create_height = height.to_owned();
    }

    /// Applies deterministic rename input used by adapter scenarios.
    pub fn set_rename_field(&mut self, name: &str) {
        self.app.transient.rename_name = name.to_owned();
    }

    /// Applies a deterministic next-launch port input used by adapter scenarios.
    pub fn set_port_field(&mut self, port: &str) {
        self.app.transient.settings_port_input = port.to_owned();
        self.app.transient.settings_save_error = None;
        self.app.transient.settings_save_succeeded = false;
    }

    /// Returns whether the current rendered state exposes the requested label.
    pub fn has_label(&self, label: &str) -> bool {
        if let Some(failure) = self.app.blocked.as_ref() {
            let mut visible = vec![
                "Chikachika cannot open this workspace".to_owned(),
                "Startup is blocked".to_owned(),
                "The saved overlay source was not changed.".to_owned(),
                failure.error().to_string(),
            ];
            if failure.store().is_some() {
                visible.extend([
                    "First copy the exact source file to a separate backup location. Then repair it or move it aside yourself, and restart Chikachika.".to_owned(),
                    "No replacement Save action is available here.".to_owned(),
                ]);
            } else {
                visible.extend([
                    "No persistence path could be resolved. Fix the platform app-data configuration yourself, then restart Chikachika.".to_owned(),
                    "No replacement Save action is available because no source path exists.".to_owned(),
                ]);
            }
            return visible
                .into_iter()
                .any(|text| text == label || text.contains(label));
        }
        let Some(coordinator) = self.app.coordinator.as_ref() else {
            return false;
        };
        let mut visible = vec![
            "File".to_owned(),
            "Edit".to_owned(),
            "View".to_owned(),
            "Help".to_owned(),
            "Overlay".to_owned(),
            "Widgets".to_owned(),
            "Canvas preview".to_owned(),
            "Create overlay".to_owned(),
            "Create Overlay".to_owned(),
            "Fit Canvas".to_owned(),
            "User Documentation".to_owned(),
            "Server unavailable".to_owned(),
            "Local server settings".to_owned(),
        ];
        if coordinator.is_dirty() {
            visible.extend(["Unsaved changes".to_owned(), "Save".to_owned()]);
        } else {
            visible.push("Saved".to_owned());
        }
        if coordinator.overlays().is_empty() {
            visible.extend([
                "No overlays yet.".to_owned(),
                "Create one to begin a local browser source workspace.".to_owned(),
            ]);
        }
        if coordinator.selected_overlay().is_none() {
            visible.push(
                "Select an overlay or use Create overlay to make your first workspace.".to_owned(),
            );
        }
        if let Some(overlay) = coordinator.selected_overlay() {
            visible.extend([
                overlay.name().to_owned(),
                "Rename".to_owned(),
                "Delete".to_owned(),
                "Frontmost first".to_owned(),
                "Add text".to_owned(),
                "Add text widget".to_owned(),
            ]);
            if let Some(url) = coordinator.selected_url() {
                visible.extend([url, "Copy URL".to_owned(), "Open output".to_owned()]);
            } else {
                visible.push(
                    "Unavailable until the local server successfully binds and reports readiness."
                        .to_owned(),
                );
            }
            if let Some(widget) = coordinator.selected_widget() {
                visible.extend([
                    widget.name().to_owned(),
                    "Widget inspector".to_owned(),
                    "Stable widget identity".to_owned(),
                    format!("Stable widget identity: {}", widget.id()),
                    "Duplicate".to_owned(),
                    "Delete widget".to_owned(),
                    "Forward".to_owned(),
                    "Backward".to_owned(),
                    "Name".to_owned(),
                    "Content".to_owned(),
                    "Font family".to_owned(),
                    "Font size".to_owned(),
                    "Color".to_owned(),
                    "RGBA".to_owned(),
                    "Alignment".to_owned(),
                    "Position".to_owned(),
                    "Canvas preview".to_owned(),
                    "Text".to_owned(),
                ]);
            } else {
                visible.extend([
                    "Overlay information".to_owned(),
                    "No widget selected.".to_owned(),
                    "Select a widget from the frontmost-first list to edit it.".to_owned(),
                ]);
            }
            visible.extend(
                overlay
                    .widgets()
                    .iter()
                    .map(|widget| widget.name().to_owned()),
            );
        }
        if let Some(error) = coordinator.last_error() {
            visible.push(error.to_owned());
        }
        if self.app.transient.create_open {
            visible.extend([
                "Name".to_owned(),
                "Fixed canvas width".to_owned(),
                "Fixed canvas height".to_owned(),
                "Create".to_owned(),
            ]);
        }
        if self.app.transient.rename_open {
            visible.extend(["Apply rename".to_owned(), "Rename overlay".to_owned()]);
        }
        if self.app.transient.delete_target.is_some() {
            visible.extend(["Confirm deletion".to_owned(), "Confirm delete".to_owned()]);
        }
        if let Some(error) = self.app.transient.dialog_error.as_deref() {
            visible.push(error.to_owned());
        }
        if let Some(error) = self.app.transient.close_error.as_deref() {
            visible.push(error.to_owned());
        }
        if let Some(error) = self.app.settings.settings_error() {
            visible.push(format!("Settings error: {error}"));
        }
        if let Some(error) = self.app.transient.settings_save_error.as_deref() {
            visible.push(format!("Settings save error: {error}"));
        }
        if self.app.transient.settings_save_succeeded {
            visible.push("Port saved for next launch".to_owned());
            visible.push(
                "Port saved for next launch. Changes take effect after restarting Chikachika."
                    .to_owned(),
            );
        }
        append_settings_labels(
            &mut visible,
            &self.app.settings,
            &self.app.transient,
            self.app
                .coordinator
                .as_ref()
                .and_then(|coordinator| coordinator.server_address().map(|address| address.port())),
        );
        visible
            .into_iter()
            .any(|text| text == label || text.contains(label))
    }
}

#[cfg(target_os = "macos")]
fn configure_macos_event_loop(
    builder: &mut winit::event_loop::EventLoopBuilder<eframe::UserEvent>,
) {
    use winit::platform::macos::EventLoopBuilderExtMacOS;
    builder.with_default_menu(false);
}

/// Run the Chikachika native window until it is closed.

/// The caller should pass `ApplicationBootstrap::new(outcome, settings)` (or
/// `outcome.with_settings(settings)`) so the GUI can display and update the
/// application settings state alongside the overlay workspace.
pub fn run(bootstrap: ApplicationBootstrap) -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(WINDOW_INITIAL_SIZE)
            .with_min_inner_size(WINDOW_MINIMUM_SIZE),
        #[cfg(target_os = "macos")]
        event_loop_builder: Some(Box::new(configure_macos_event_loop)),
        ..Default::default()
    };

    eframe::run_native(
        "Chikachika",
        native_options,
        Box::new(move |_creation_context| {
            Ok(Box::new(ChikachikaApp::from_application_bootstrap(
                bootstrap,
            )))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::Store;

    fn ready_app() -> HeadlessCoordinator {
        use std::cell::RefCell;

        thread_local! {
            static TEST_STORE_DIRECTORY: RefCell<(tempfile::TempDir, usize)> = RefCell::new((
                tempfile::tempdir().expect("test store directory"),
                0,
            ));
        }
        let path = TEST_STORE_DIRECTORY.with(|state| {
            let mut state = state.borrow_mut();
            let path = state.0.path().join(format!("overlays-{}.json", state.1));
            state.1 += 1;
            path
        });
        HeadlessCoordinator::empty(Store::at(path))
    }

    fn setup_widgets(harness: &mut ScenarioHarness) -> (OverlayId, Vec<TextWidgetId>) {
        let coordinator = harness.app_mut().coordinator_mut().unwrap();
        let overlay_id = coordinator
            .create_overlay("Live", 320, 240)
            .expect("create overlay");
        let back = coordinator
            .add_widget(overlay_id, TextWidget::new("Back"))
            .unwrap();
        let middle = coordinator
            .add_widget(overlay_id, TextWidget::new("Middle"))
            .unwrap();
        let front = coordinator
            .add_widget(overlay_id, TextWidget::new("Front"))
            .unwrap();
        (overlay_id, vec![front, middle, back])
    }

    #[test]
    fn failed_preview_transitions_keep_recoverable_state() {
        let mut coordinator = ready_app();
        let overlay_id = coordinator.create_overlay("Live", 320, 240).unwrap();
        let widget_ids = [
            coordinator
                .add_widget(overlay_id, TextWidget::new("Front"))
                .unwrap(),
            coordinator
                .add_widget(overlay_id, TextWidget::new("Back"))
                .unwrap(),
        ];
        let foreign_overlay = coordinator.create_overlay("Other", 320, 240).unwrap();
        let foreign_widget = coordinator
            .add_widget(foreign_overlay, TextWidget::new("Other"))
            .unwrap();
        coordinator.select_overlay(overlay_id).unwrap();
        coordinator.select_widget(widget_ids[0]).unwrap();

        let invalid_drag = PreviewDrag {
            overlay_id,
            widget_id: foreign_widget,
            pointer_offset: egui::Vec2::ZERO,
            start_position: Position::new(0.0, 0.0),
        };
        let mut transient = TransientState::default();
        transient.preview_drag = Some(invalid_drag);
        assert!(!select_preview_widget(
            &mut coordinator,
            &mut transient,
            foreign_widget
        ));
        assert!(transient.preview_drag.is_none());
        assert!(transient.preview_error.is_some());

        transient.preview_drag = Some(invalid_drag);
        assert!(begin_preview_drag(&mut coordinator, &mut transient, invalid_drag).is_err());
        assert!(transient.active_edit.is_none());
        assert_eq!(transient.preview_drag, Some(invalid_drag));

        let name_target = edit_target(overlay_id, widget_ids[0], "name");
        coordinator.begin_edit(name_target.clone()).unwrap();
        let position = Position::new(5.0, 6.0);
        assert!(update_preview_drag(&mut coordinator, invalid_drag, position).is_err());
        assert_eq!(coordinator.pending_edit_target(), Some(&name_target));

        let active_target = edit_target(overlay_id, widget_ids[0], "canvas_position");
        transient.active_edit = Some(active_target);
        transient.preview_drag = Some(PreviewDrag {
            overlay_id,
            widget_id: widget_ids[0],
            pointer_offset: egui::Vec2::ZERO,
            start_position: Position::new(0.0, 0.0),
        });
        let active_drag = transient.preview_drag.unwrap();
        assert!(commit_preview_drag(&mut coordinator, &mut transient, active_drag).is_err());
        assert_eq!(transient.preview_drag, Some(active_drag));
        assert!(transient.active_edit.is_some());
    }

    fn assert_workspace_geometry(harness: &ScenarioHarness, size: egui::Vec2) {
        let left = harness
            .app()
            .transient
            .widget_panel_rect
            .expect("widget panel is rendered");
        let right = harness
            .app()
            .transient
            .inspector_panel_rect
            .expect("inspector panel is rendered");
        let canvas = harness
            .app()
            .transient
            .preview_rect
            .expect("canvas is rendered");
        let name_field = harness
            .control_rect("Widget name")
            .expect("selected widget name field is rendered");

        assert!(left.width() >= WIDGET_LIST_MINIMUM_WIDTH);
        assert!(right.width() >= INSPECTOR_MINIMUM_WIDTH);
        assert!(left.right() <= canvas.left());
        assert!(canvas.right() <= right.left());
        assert!(left.left() >= 0.0 && left.right() <= size.x);
        assert!(right.left() >= 0.0 && right.right() <= size.x);
        assert!(canvas.top() >= 50.0 && canvas.bottom() <= size.y);
        assert!(right.contains(name_field.center()));
    }

    #[test]
    fn workspace_exposes_complete_lifecycle_controls() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.frame();
        assert!(!harness.app().is_blocked());
        assert!(harness.app().coordinator().unwrap().overlays().is_empty());
    }

    #[test]
    fn workspace_keeps_canvas_inside_the_minimum_window() {
        let mut harness = ScenarioHarness::new_with_size(
            BootstrapOutcome::Ready(ready_app()),
            egui::vec2(1024.0, 640.0),
        );
        setup_widgets(&mut harness);
        harness.frame();

        let canvas = harness
            .app()
            .transient
            .preview_rect
            .expect("canvas preview is rendered");
        assert!(
            canvas.left() >= 180.0,
            "canvas overlaps the left panel: {canvas:?}"
        );
        assert!(
            canvas.right() <= 1024.0,
            "canvas exceeds the window: {canvas:?}"
        );
        assert!(
            canvas.top() >= 50.0,
            "canvas overlaps the menu bars: {canvas:?}"
        );
        assert!(
            canvas.bottom() <= 640.0,
            "canvas exceeds the window: {canvas:?}"
        );
        assert_workspace_geometry(&harness, egui::vec2(1024.0, 640.0));
    }

    #[test]
    fn workspace_keeps_canvas_inside_a_larger_desktop_window() {
        let mut harness = ScenarioHarness::new_with_size(
            BootstrapOutcome::Ready(ready_app()),
            egui::vec2(1280.0, 800.0),
        );
        setup_widgets(&mut harness);
        harness.frame();

        let canvas = harness
            .app()
            .transient
            .preview_rect
            .expect("canvas preview is rendered");
        assert!(
            canvas.left() >= 180.0,
            "canvas overlaps the left panel: {canvas:?}"
        );
        assert!(
            canvas.right() <= 1280.0,
            "canvas exceeds the window: {canvas:?}"
        );
        assert!(
            canvas.top() >= 50.0,
            "canvas overlaps the menu bars: {canvas:?}"
        );
        assert!(
            canvas.bottom() <= 800.0,
            "canvas exceeds the window: {canvas:?}"
        );
        assert_workspace_geometry(&harness, egui::vec2(1280.0, 800.0));
    }

    #[test]
    fn native_window_and_sidebar_sizes_match_the_workspace_contract() {
        assert_eq!(WINDOW_INITIAL_SIZE, [1280.0, 800.0]);
        assert_eq!(WINDOW_MINIMUM_SIZE, [1024.0, 640.0]);
        assert_eq!(WIDGET_LIST_INITIAL_WIDTH, 220.0);
        assert_eq!(WIDGET_LIST_MINIMUM_WIDTH, 180.0);
        assert_eq!(INSPECTOR_INITIAL_WIDTH, 280.0);
        assert_eq!(INSPECTOR_MINIMUM_WIDTH, 260.0);
    }

    #[test]
    fn workspace_style_matches_fdr_spacing_and_typography_targets() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.frame();

        let style = harness.context.style();
        assert_eq!(style.spacing.item_spacing, egui::vec2(8.0, 8.0));
        assert_eq!(style.spacing.window_margin, egui::Margin::same(12.0));
        assert_eq!(
            style
                .text_styles
                .get(&egui::TextStyle::Body)
                .expect("body text style")
                .size,
            14.0
        );
        assert_eq!(
            workspace_side_panel_frame(&style).inner_margin,
            egui::Margin::same(12.0)
        );
        assert_eq!(
            workspace_central_panel_frame(&style).inner_margin,
            egui::Margin::same(12.0)
        );
    }

    #[test]
    fn inspector_layer_controls_reflect_available_moves() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (_overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();

        assert_eq!(harness.layer_action_is_enabled("Forward"), Some(false));
        assert_eq!(harness.layer_action_is_enabled("Backward"), Some(true));

        let middle_row = harness
            .widget_selector_rect(ids[1])
            .expect("middle widget row");
        harness.pointer_click(middle_row.center());
        assert_eq!(harness.layer_action_is_enabled("Forward"), Some(true));
        assert_eq!(harness.layer_action_is_enabled("Backward"), Some(true));

        let back_row = harness
            .widget_selector_rect(ids[2])
            .expect("back widget row");
        harness.pointer_click(back_row.center());
        assert_eq!(harness.layer_action_is_enabled("Forward"), Some(true));
        assert_eq!(harness.layer_action_is_enabled("Backward"), Some(false));
    }

    #[test]
    fn side_panel_divider_drag_resizes_and_refits_canvas_at_minimum_size() {
        let size = egui::vec2(1024.0, 640.0);
        let mut harness =
            ScenarioHarness::new_with_size(BootstrapOutcome::Ready(ready_app()), size);
        let coordinator = harness.app_mut().coordinator_mut().unwrap();
        let overlay_id = coordinator
            .create_overlay("Wide", 1280, 720)
            .expect("create wide overlay");
        coordinator
            .add_widget(overlay_id, TextWidget::new("Selected"))
            .expect("add selected widget");
        harness.frame();

        let original_panel = harness
            .app()
            .transient
            .widget_panel_rect
            .expect("widget panel is rendered");
        let original_canvas = harness
            .app()
            .transient
            .preview_rect
            .expect("canvas is rendered");
        let divider = egui::pos2(original_panel.right(), original_panel.center().y);
        let destination = divider + egui::vec2(100.0, 0.0);

        harness.pointer_move(divider);
        harness.pointer_button(divider, true);
        harness.pointer_move(destination);
        harness.frame();
        harness.pointer_button(destination, false);
        harness.frame();

        let resized_panel = harness
            .app()
            .transient
            .widget_panel_rect
            .expect("resized widget panel is rendered");
        let resized_canvas = harness
            .app()
            .transient
            .preview_rect
            .expect("resized canvas is rendered");
        assert!(
            resized_panel.width() > original_panel.width() + 40.0,
            "drag should resize the widget panel: {original_panel:?} -> {resized_panel:?}"
        );
        assert!(
            resized_canvas.width() < original_canvas.width(),
            "canvas should refit after the center area shrinks: {original_canvas:?} -> {resized_canvas:?}"
        );
        let canvas_aspect = resized_canvas.width() / resized_canvas.height();
        assert!((canvas_aspect - 1280.0 / 720.0).abs() < 0.01);
        assert_workspace_geometry(&harness, size);
    }

    #[test]
    fn issue_23_menus_run_their_supported_actions() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let coordinator =
            HeadlessCoordinator::empty(Store::at(directory.path().join("overlays.json")));
        let mut harness = ScenarioHarness::new_with_size(
            BootstrapOutcome::Ready(coordinator),
            egui::vec2(WINDOW_INITIAL_SIZE[0], WINDOW_INITIAL_SIZE[1]),
        );
        harness.frame();

        harness
            .pointer_click_control("File menu")
            .expect("open File menu");
        harness
            .pointer_click_control("File/Create Overlay")
            .expect("open create dialog from File menu");
        assert!(harness.app().transient.create_open);
        harness.click("Cancel").expect("close create dialog");
        harness.click("Create overlay").expect("open create dialog");
        harness.set_create_fields("Live", "320", "240");
        harness.click("Create").expect("create overlay");

        harness
            .pointer_click_control("Edit menu")
            .expect("open Edit menu");
        harness
            .pointer_click_control("Edit/Add Text")
            .expect("add text from Edit menu");
        harness
            .pointer_click_control("Edit menu")
            .expect("reopen Edit menu");
        harness
            .pointer_click_control("Edit/Duplicate")
            .expect("duplicate from Edit menu");
        let coordinator = harness.app().coordinator().unwrap();
        let duplicated = coordinator
            .selected_widget_id()
            .expect("duplicate selected");
        let added = coordinator
            .selected_overlay()
            .unwrap()
            .widgets()
            .iter()
            .find(|widget| widget.id() != duplicated)
            .unwrap()
            .id();
        assert_eq!(coordinator.selected_overlay().unwrap().widgets().len(), 2);
        harness
            .pointer_click_control("Edit menu")
            .expect("reopen Edit menu");
        harness
            .pointer_click_control("Edit/Backward")
            .expect("move widget backward from Edit menu");
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_overlay()
                .unwrap()
                .widgets()
                .iter()
                .map(TextWidget::id)
                .collect::<Vec<_>>(),
            vec![added, duplicated]
        );
        harness
            .pointer_click_control("Edit menu")
            .expect("reopen Edit menu");
        harness
            .pointer_click_control("Edit/Forward")
            .expect("move widget forward from Edit menu");
        harness
            .pointer_click_control("Edit menu")
            .expect("reopen Edit menu");
        harness
            .pointer_click_control("Edit/Delete")
            .expect("delete from Edit menu");
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_overlay()
                .unwrap()
                .widgets()
                .len(),
            1
        );

        harness
            .pointer_click_control("View menu")
            .expect("open View menu");
        harness
            .pointer_click_control("View/Fit Canvas")
            .expect("fit canvas from View menu");
        harness
            .pointer_click_control("Help menu")
            .expect("open Help menu");
        harness
            .pointer_click_control("Help/User Documentation")
            .expect("open user documentation from Help menu");
        assert_eq!(harness.opened_url(), Some(USER_DOCUMENTATION_URL));

        harness
            .pointer_click_control("File menu")
            .expect("reopen File menu");
        harness
            .pointer_click_control("File/Save")
            .expect("save from File menu");
        assert!(!harness.app().coordinator().unwrap().is_dirty());
    }

    #[test]
    fn output_actions_wait_for_server_readiness() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.click("Create overlay").expect("open create dialog");
        harness.set_create_fields("Live", "320", "240");
        harness.click("Create").expect("create overlay");
        harness.frame();
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_url()
                .is_none()
        );
        assert!(harness.has_label("Unavailable until the local server successfully binds"));
        harness
            .pointer_click_control("Copy URL")
            .expect("readiness-disabled URL action is still safely inactive");
        assert!(harness.copied_text().is_empty());

        harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .set_server_address("127.0.0.1:1234".parse().unwrap());
        harness.frame();
        let url = harness.app().coordinator().unwrap().selected_url().unwrap();
        harness
            .pointer_click_control("Copy URL")
            .expect("copy ready output URL");
        assert_eq!(harness.copied_text(), url);
        harness
            .pointer_click_control("Open output")
            .expect("open ready output");
        assert_eq!(harness.opened_url(), Some(url.as_str()));
    }

    #[test]
    fn widget_row_rename_focuses_the_existing_inspector_field() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (_overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();

        harness
            .pointer_click_control(&format!("Rename widget {}", ids[1]))
            .expect("focus the inspector name field from the widget row");
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[1])
        );
        assert!(harness.app().transient.name_field_focused);
    }

    #[test]
    fn blocked_startup_shows_backup_first_recovery_guidance() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source_path = directory.path().join("overlays.json");
        std::fs::write(&source_path, b"not json").expect("write malformed overlay source");
        let outcome = HeadlessCoordinator::bootstrap_outcome(Store::at(&source_path));
        let BootstrapOutcome::Blocked(_) = outcome else {
            panic!("malformed overlay source should block startup");
        };
        let mut harness = ScenarioHarness::new(outcome);
        harness.frame();
        assert!(harness.has_label("Chikachika cannot open this workspace"));
        assert!(harness.has_label("The saved overlay source was not changed."));
        assert!(
            harness.has_label("First copy the exact source file to a separate backup location.")
        );
        assert!(
            harness.has_label("Then repair it or move it aside yourself, and restart Chikachika.")
        );
        assert!(!harness.has_label("remove the source"));
    }

    #[test]
    fn settings_controls_show_port_path_and_restart_semantics() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let settings_path = directory.path().join("settings.json");
        let settings = SettingsState::from_settings(
            SettingsStore::at(&settings_path),
            Settings::new(4_000).expect("valid settings"),
        );
        let mut harness =
            ScenarioHarness::new_with_settings(BootstrapOutcome::Ready(ready_app()), settings);
        harness.frame();

        assert!(harness.has_label("Local server settings"));
        assert!(harness.control_rect("Save port for next launch").is_none());
        harness
            .pointer_click_control("Local server settings")
            .expect("expand server settings");
        harness.frame();
        assert!(harness.control_rect("Save port for next launch").is_some());
        assert!(harness.has_label("Current configured port: 4000"));
        assert!(harness.has_label("Next-launch port: 4000"));
        assert!(harness.has_label(&format!("Settings path: {}", settings_path.display())));
        assert!(harness.has_label("Port for next launch (1–65535)"));
        assert!(harness.has_label("Save port for next launch"));
        assert!(harness.has_label("Port changes take effect only after restarting Chikachika."));
    }

    #[test]
    fn port_input_accepts_only_the_inclusive_valid_range() {
        assert_eq!(parse_port_input("1"), Ok(1));
        assert_eq!(parse_port_input("65535"), Ok(65535));
        assert!(parse_port_input("0").is_err());
        assert!(parse_port_input("65536").is_err());
        assert!(parse_port_input("not a port").is_err());
        assert!(parse_port_input(" ").is_err());

        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.frame();
        harness.set_port_field("0");
        assert!(harness.click("Save port for next launch").is_err());
        assert!(harness.has_label("Port must be a whole number from 1 to 65535."));
        assert_eq!(harness.settings().configured_port(), Some(51_737));
    }

    #[test]
    fn settings_save_is_separate_from_overlay_save_and_updates_only_settings() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let settings_path = directory.path().join("settings.json");
        let settings = SettingsState::from_settings(
            SettingsStore::at(&settings_path),
            Settings::new(4_000).expect("valid settings"),
        );
        let mut harness =
            ScenarioHarness::new_with_settings(BootstrapOutcome::Ready(ready_app()), settings);
        harness.frame();
        harness.click("Create overlay").expect("open create dialog");
        harness.set_create_fields("Unsaved", "320", "240");
        harness.click("Create").expect("create overlay");
        assert!(harness.app().coordinator().unwrap().is_dirty());

        harness.set_port_field("4_001");
        assert!(harness.click("Save port for next launch").is_err());
        harness.set_port_field("4001");
        harness
            .click("Save port for next launch")
            .expect("save next-launch port");

        assert_eq!(harness.settings().configured_port(), Some(4_001));
        assert!(harness.settings().settings_error().is_none());
        assert!(harness.app().coordinator().unwrap().is_dirty());
        assert!(harness.has_label("Port saved for next launch."));
        assert!(harness.has_label("Changes take effect after restarting Chikachika."));
        assert_eq!(
            SettingsStore::at(&settings_path)
                .load()
                .unwrap()
                .server_port(),
            4_001
        );
    }

    #[test]
    fn failed_settings_save_preserves_previous_value_and_error() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let settings_path = directory.path().join("settings.json");
        std::fs::create_dir(&settings_path).expect("make destination a directory");
        let settings = SettingsState::from_settings(
            SettingsStore::at(&settings_path),
            Settings::new(4_000).expect("valid settings"),
        );
        let mut harness =
            ScenarioHarness::new_with_settings(BootstrapOutcome::Ready(ready_app()), settings);
        harness.frame();
        harness.set_port_field("4001");
        assert!(harness.click("Save port for next launch").is_err());

        assert_eq!(harness.settings().configured_port(), Some(4_000));
        assert!(harness.settings().settings_error().is_none());
        assert!(harness.has_label("Could not save port for next launch:"));
        assert!(harness.has_label("The previous configured port remains unchanged."));
        assert!(!harness.has_label("Port saved for next launch."));
    }

    #[test]
    fn invalid_settings_show_recovery_guidance_and_can_be_repaired() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let settings_path = directory.path().join("settings.json");
        std::fs::write(&settings_path, b"not json").expect("write malformed settings");
        let settings = SettingsState::load(SettingsStore::at(&settings_path));
        assert!(!settings.is_valid());
        let mut harness =
            ScenarioHarness::new_with_settings(BootstrapOutcome::Ready(ready_app()), settings);
        harness.frame();
        harness
            .pointer_click_control("Local server settings")
            .expect("expand server settings");

        assert!(harness.has_label("Current configured port: unavailable"));
        assert!(harness.has_label("Settings error:"));
        assert!(
            harness
                .has_label("First copy the exact settings source to a separate backup location.")
        );
        assert!(
            harness.has_label("Then repair it or move it aside yourself, and restart Chikachika.")
        );
        assert!(harness.has_label("No fallback port is used while settings are invalid."));
        assert!(!harness.has_label("remove the settings source"));

        harness.set_port_field("4001");
        harness
            .click("Save port for next launch")
            .expect("repair settings by saving a valid port");
        assert!(harness.settings().is_valid());
        assert_eq!(harness.settings().configured_port(), Some(4_001));
        assert!(harness.settings().settings_error().is_none());
        assert!(harness.has_label("Port saved for next launch."));
    }

    #[test]
    fn headless_harness_drives_create_validation_and_successful_creation() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.frame();
        harness.click("Create overlay").expect("open create dialog");
        harness.set_create_fields("", "0", "720");
        assert!(harness.click("Create").is_err());
        assert!(harness.has_label("Canvas width and height must be positive whole numbers."));
        assert!(harness.app().coordinator().unwrap().overlays().is_empty());

        harness.set_create_fields("Starting Soon", "1280", "720");
        harness.click("Create").expect("create overlay");
        let coordinator = harness.app().coordinator().unwrap();
        assert_eq!(coordinator.overlays().len(), 1);
        assert_eq!(
            coordinator.selected_overlay().unwrap().name(),
            "Starting Soon"
        );
        assert!(coordinator.is_dirty());
        assert!(harness.has_label("Starting Soon"));
        assert!(harness.has_label("No widget selected."));
    }

    #[test]
    fn headless_harness_drives_selection_and_rename() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.frame();
        harness.click("Create overlay").expect("open create dialog");
        harness.set_create_fields("Starting Soon", "1280", "720");
        harness.click("Create").expect("create first overlay");
        let first_id = harness
            .app()
            .coordinator()
            .unwrap()
            .selected_overlay_id()
            .unwrap();
        harness.click("Create overlay").expect("open second dialog");
        harness.set_create_fields("Be Right Back", "640", "360");
        harness.click("Create").expect("create second overlay");
        harness
            .click("Starting Soon")
            .expect("select first overlay");
        assert_eq!(
            harness.app().coordinator().unwrap().selected_overlay_id(),
            Some(first_id)
        );
        harness.click("Rename").expect("open rename dialog");
        harness.set_rename_field("Live Soon");
        harness.click("Apply rename").expect("rename overlay");
        let coordinator = harness.app().coordinator().unwrap();
        assert_eq!(coordinator.selected_overlay_id(), Some(first_id));
        assert_eq!(coordinator.selected_overlay().unwrap().name(), "Live Soon");
    }

    #[test]
    fn delete_cancel_and_confirm_are_target_specific() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.click("Create overlay").expect("open first dialog");
        harness.set_create_fields("First", "320", "240");
        harness.click("Create").expect("create first overlay");
        harness.click("Create overlay").expect("open second dialog");
        harness.set_create_fields("Second", "640", "480");
        harness.click("Create").expect("create second overlay");
        harness.click("First").expect("select first overlay");
        let first_id = harness
            .app()
            .coordinator()
            .unwrap()
            .selected_overlay_id()
            .unwrap();
        harness.click("Delete").expect("open delete confirmation");
        harness.click("Cancel").expect("cancel deletion");
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(first_id)
                .is_some()
        );
        harness.click("Delete").expect("reopen delete confirmation");
        harness.click("Confirm delete").expect("confirm deletion");
        let coordinator = harness.app().coordinator().unwrap();
        assert!(coordinator.overlay(first_id).is_none());
        assert_eq!(coordinator.selected_overlay().unwrap().name(), "Second");
    }

    #[test]
    fn preview_scale_and_coordinate_conversion_preserve_canvas_geometry() {
        let canvas = crate::model::CanvasSize::new(1920, 1080).unwrap();
        let scale = preview_scale(canvas, 960.0, 360.0);
        assert_eq!(scale, 1.0 / 3.0);
        assert_eq!(preview_scale(canvas, 600.0, 400.0), 600.0 / 1920.0);
        assert_eq!(preview_scale(canvas, 400.0, 500.0), 400.0 / 1920.0);

        let origin = egui::pos2(10.0, 20.0);
        let position = Position::new(300.0, 150.0);
        assert_eq!(
            canvas_to_preview(origin, position, scale),
            egui::pos2(110.0, 70.0)
        );
        assert_eq!(
            preview_to_canvas(
                origin,
                egui::pos2(125.0, 80.0),
                egui::vec2(15.0, 10.0),
                scale,
                canvas,
            ),
            position
        );
        assert_eq!(
            preview_to_canvas(
                origin,
                egui::pos2(-500.0, 900.0),
                egui::Vec2::ZERO,
                scale,
                canvas,
            ),
            Position::new(0.0, 1080.0)
        );
    }

    #[test]
    fn preview_hitbox_is_widget_scoped_and_empty_text_has_a_small_handle() {
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 240.0));
        let visual = egui::Rect::from_min_max(egui::pos2(40.0, 50.0), egui::pos2(90.0, 70.0));
        let hitbox = widget_hitbox(canvas, egui::pos2(40.0, 50.0), visual);
        assert!(hitbox.contains(egui::pos2(60.0, 60.0)));
        assert!(!hitbox.contains(egui::pos2(200.0, 200.0)));

        let empty = widget_hitbox(canvas, egui::pos2(300.0, 230.0), egui::Rect::NOTHING);
        assert_eq!(empty.min, egui::pos2(300.0, 230.0));
        assert_eq!(empty.max, egui::pos2(312.0, 240.0));
    }

    #[test]
    fn preview_region_and_alignment_keep_model_position_as_top_left() {
        let canvas = egui::Rect::from_min_size(egui::pos2(5.0, 10.0), egui::vec2(320.0, 240.0));
        let position = Position::new(40.0, 30.0);
        let region = text_region_rect(canvas, position, 0.5, 24.0);
        assert_eq!(region.min, egui::pos2(25.0, 25.0));
        assert_eq!(region.right(), canvas.right());
        let width = region.width();
        assert_eq!(
            aligned_paint_origin(region.min, width, Alignment::Left),
            region.min
        );
        assert_eq!(
            aligned_paint_origin(region.min, width, Alignment::Center),
            egui::pos2(region.center().x, region.min.y)
        );
        assert_eq!(
            aligned_paint_origin(region.min, width, Alignment::Right),
            egui::pos2(region.right(), region.min.y)
        );
    }

    #[test]
    fn multi_widget_editor_scenario() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        assert!(
            harness
                .shapes()
                .iter()
                .any(|shape| matches!(shape.shape, egui::Shape::Text(_)))
        );

        let middle_rect = harness
            .widget_selector_rect(ids[1])
            .expect("middle selector row shape");
        harness.pointer_click(middle_rect.center());
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[1])
        );
        assert_eq!(
            harness.app().transient.inspector_target,
            Some((overlay_id, ids[1]))
        );

        // Replace inspector values through the focused controls, not through
        // coordinator or transient-state shortcuts.
        harness
            .replace_text_control("Widget name", "Edited middle")
            .expect("edit widget name through egui input");
        harness
            .replace_text_control("Widget content", "Edited body")
            .expect("edit widget content through egui input");
        harness
            .replace_number_control("Font size", "42")
            .expect("edit font size through egui input");
        harness
            .replace_number_control("Position X", "123")
            .expect("edit position through egui input");
        harness
            .pointer_click_control("Alignment Center")
            .expect("edit alignment through egui pointer input");

        let coordinator = harness.app().coordinator().unwrap();
        let overlay = coordinator.overlay(overlay_id).unwrap();
        assert_eq!(
            overlay
                .widgets()
                .iter()
                .map(TextWidget::id)
                .collect::<Vec<_>>(),
            ids
        );
        assert_eq!(overlay.widget(ids[0]).unwrap().name(), "Front");
        assert_eq!(overlay.widget(ids[2]).unwrap().name(), "Back");
        assert_eq!(overlay.widget(ids[1]).unwrap().name(), "Edited middle");
        assert_eq!(overlay.widget(ids[1]).unwrap().content(), "Edited body");
        assert_eq!(overlay.widget(ids[1]).unwrap().font_size(), 42.0);
        assert_eq!(
            overlay.widget(ids[1]).unwrap().position(),
            Position::new(123.0, 0.0)
        );
        assert_eq!(
            overlay.widget(ids[1]).unwrap().alignment(),
            Alignment::Center
        );
        assert_eq!(coordinator.selected_widget_id(), Some(ids[1]));
        assert_eq!(
            harness.app().transient.inspector_target,
            Some((overlay_id, ids[1]))
        );

        harness
            .pointer_click_control("Forward")
            .expect("move selected widget forward through egui pointer input");
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widgets()
                .iter()
                .map(TextWidget::id)
                .collect::<Vec<_>>(),
            vec![ids[1], ids[0], ids[2]]
        );
        harness.frame();
        harness
            .pointer_click_control("Backward")
            .expect("move selected widget backward through egui pointer input");
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widgets()
                .iter()
                .map(TextWidget::id)
                .collect::<Vec<_>>(),
            ids
        );

        harness
            .pointer_click_control("Add text widget")
            .expect("create widget through egui pointer input");
        let inserted = harness
            .app()
            .coordinator()
            .unwrap()
            .selected_widget_id()
            .expect("new widget selection");
        assert!(!ids.contains(&inserted));
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widgets()
                .iter()
                .map(TextWidget::id)
                .collect::<Vec<_>>(),
            vec![inserted, ids[0], ids[1], ids[2]]
        );

        harness
            .pointer_click_control("Duplicate")
            .expect("duplicate widget through egui pointer input");
        let duplicated = harness
            .app()
            .coordinator()
            .unwrap()
            .selected_widget_id()
            .expect("duplicated widget selection");
        assert_ne!(duplicated, inserted);
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widgets()
                .iter()
                .map(TextWidget::id)
                .collect::<Vec<_>>(),
            vec![duplicated, inserted, ids[0], ids[1], ids[2]]
        );
        assert!(harness.has_label(&format!("Stable widget identity: {duplicated}")));
        harness.frame();

        harness
            .pointer_click_control("Delete widget")
            .expect("delete duplicated widget through egui pointer input");
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widgets()
                .iter()
                .map(TextWidget::id)
                .collect::<Vec<_>>(),
            vec![inserted, ids[0], ids[1], ids[2]]
        );
        harness.frame();
        let inserted_rect = harness
            .widget_selector_rect(inserted)
            .expect("inserted selector row after duplicate deletion");
        harness.pointer_click(inserted_rect.center());
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(inserted)
        );
        harness
            .pointer_click_control("Delete widget")
            .expect("delete inserted widget through egui pointer input");
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widgets()
                .iter()
                .map(TextWidget::id)
                .collect::<Vec<_>>(),
            ids
        );
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[0])
        );

        let preview_rect = harness
            .app()
            .transient
            .preview_rect
            .expect("actual preview canvas shape");
        let preview_texts: Vec<String> = harness
            .shapes()
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if preview_rect.contains(text.pos)
                        || preview_rect.contains(text.pos + egui::vec2(1.0, 1.0)) =>
                {
                    Some(text.galley.job.text.clone())
                }
                _ => None,
            })
            .filter(|text| matches!(text.as_str(), "Back" | "Edited body" | "Front"))
            .collect();
        assert_eq!(
            preview_texts,
            vec![
                "Back".to_owned(),
                "Edited body".to_owned(),
                "Front".to_owned()
            ]
        );
    }

    #[test]
    fn multi_widget_preview_paint_order() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        {
            let coordinator = harness.app_mut().coordinator_mut().unwrap();
            coordinator
                .update_overlay(overlay_id, |overlay| {
                    overlay.set_widget_position(ids[0], Position::new(10.0, 10.0))?;
                    overlay.set_widget_position(ids[1], Position::new(100.0, 60.0))?;
                    overlay.set_widget_position(ids[2], Position::new(200.0, 110.0))
                })
                .unwrap();
        }
        harness.frame();
        let preview_rect = harness
            .app()
            .transient
            .preview_rect
            .expect("actual preview canvas shape");
        let preview_texts: Vec<String> = harness
            .shapes()
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if preview_rect.contains(text.pos)
                        || preview_rect.contains(text.pos + egui::vec2(1.0, 1.0)) =>
                {
                    Some(text.galley.job.text.clone())
                }
                _ => None,
            })
            .filter(|text| matches!(text.as_str(), "Back" | "Middle" | "Front"))
            .collect();
        // Actual preview text primitives must be emitted back-to-front even
        // though index zero is frontmost in the authoritative model.
        assert_eq!(
            preview_texts,
            vec!["Back".to_owned(), "Middle".to_owned(), "Front".to_owned()]
        );
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn canvas_click_selects_frontmost_overlap_and_updates_inspector() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();

        let front = harness.widget_canvas_rect(ids[0]).expect("front hitbox");
        let middle = harness.widget_canvas_rect(ids[1]).expect("middle hitbox");
        let back = harness.widget_canvas_rect(ids[2]).expect("back hitbox");
        let overlap = front.intersect(middle).intersect(back);
        assert!(
            overlap.is_positive(),
            "widgets overlap: {front:?}, {middle:?}, {back:?}"
        );
        let back_row = harness
            .widget_selector_rect(ids[2])
            .expect("back widget row");
        harness.pointer_click(back_row.center());
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[2])
        );

        harness.pointer_click(overlap.center());

        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[0])
        );
        assert_eq!(
            harness.app().transient.inspector_target,
            Some((overlay_id, ids[0]))
        );
        assert!(harness.has_label(&format!("Stable widget identity: {}", ids[0])));
    }

    #[test]
    fn obscured_widget_remains_selectable_from_its_list_row() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        let row = harness
            .widget_selector_rect(ids[2])
            .expect("obscured widget row");

        harness.pointer_click(row.center());

        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[2])
        );
        assert_eq!(
            harness.app().transient.inspector_target,
            Some((overlay_id, ids[2]))
        );
        assert!(harness.has_label(&format!("Stable widget identity: {}", ids[2])));
        harness.frame();
        let row_texts: Vec<String> = harness
            .shapes()
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            row_texts
                .iter()
                .any(|text| text.contains("▶") && text.contains("Back")),
            "selected row should contain its non-color marker: {row_texts:?}"
        );
    }

    #[test]
    fn empty_canvas_click_repaints_inspector_and_selection_outline() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (_overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        let empty_point = harness
            .app()
            .transient
            .preview_rect
            .expect("canvas is rendered")
            .center();
        assert!(ids.iter().all(|id| {
            !harness
                .widget_canvas_rect(*id)
                .expect("widget hitbox")
                .contains(empty_point)
        }));

        harness.pointer_click(empty_point);

        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            None
        );
        assert_eq!(harness.app().transient.inspector_target, None);
        // Process the repaint scheduled for the click and inspect the shapes
        // actually emitted for the resulting frame.
        harness.frame();
        let rendered_text: Vec<&str> = harness
            .shapes()
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(rendered_text.contains(&"Overlay information"));
        assert!(!rendered_text.contains(&"Widget inspector"));
        assert!(
            !harness.shapes().iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Rect(rect) if rect.stroke.color == SELECTION_OUTLINE_COLOR
            )),
            "selection outline should be removed in the repaint after the click"
        );
    }

    #[test]
    fn single_frame_canvas_click_selects_a_separate_caption_and_renders_its_inspector() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let coordinator = harness.app_mut().coordinator_mut().unwrap();
        let overlay_id = coordinator.create_overlay("Live", 320, 240).unwrap();
        let dark = coordinator
            .add_widget(
                overlay_id,
                TextWidget::with_properties(
                    "Dark foreground",
                    Position::new(20.0, 20.0),
                    24.0,
                    Color::rgb(6, 5, 5),
                    Alignment::Left,
                )
                .unwrap(),
            )
            .unwrap();
        let caption = coordinator
            .add_widget(
                overlay_id,
                TextWidget::with_properties(
                    "Light caption",
                    Position::new(100.0, 130.0),
                    12.0,
                    Color::white(),
                    Alignment::Left,
                )
                .unwrap(),
            )
            .unwrap();
        coordinator.select_widget(dark).unwrap();
        harness.frame();
        let caption_hitbox = harness
            .widget_canvas_rect(caption)
            .expect("caption canvas hitbox");
        assert!(
            !harness
                .widget_canvas_rect(dark)
                .expect("dark foreground hitbox")
                .contains(caption_hitbox.center()),
            "caption hit should not overlap the selected foreground"
        );

        harness.pointer_click_in_single_frame(caption_hitbox.center());

        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(caption)
        );
        assert_eq!(
            harness.app().transient.inspector_target,
            Some((overlay_id, caption))
        );
        harness.frame();
        let rendered_text: Vec<String> = harness
            .shapes()
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            rendered_text
                .iter()
                .any(|text| text.contains(&caption.to_string()))
        );
        assert!(
            rendered_text
                .iter()
                .any(|text| text.contains("▶ Light caption"))
        );
        assert!(
            harness.shapes().iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Rect(rect) if rect.stroke.color == SELECTION_OUTLINE_COLOR
            )),
            "selected caption should have a cyan canvas outline"
        );
    }

    #[test]
    fn canvas_selection_reveals_its_widget_row() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let overlay_id = harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .create_overlay("Live", 320, 240)
            .unwrap();
        let target = harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .add_widget(overlay_id, TextWidget::new("Target"))
            .unwrap();
        harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .update_overlay(overlay_id, |overlay| {
                overlay.set_widget_position(target, Position::new(200.0, 170.0))
            })
            .unwrap();
        for index in 0..36 {
            harness
                .app_mut()
                .coordinator_mut()
                .unwrap()
                .add_widget(overlay_id, TextWidget::new(format!("Other {index}")))
                .unwrap();
        }
        harness.frame();

        let panel = harness
            .app()
            .transient
            .widget_panel_rect
            .expect("widget panel is rendered");
        let row = harness.widget_selector_rect(target).expect("target row");
        assert!(
            !panel.contains(row.center()),
            "target row starts out of view"
        );
        let hitbox = harness.widget_canvas_rect(target).expect("target hitbox");

        harness.pointer_move(hitbox.center());
        harness.pointer_button(hitbox.center(), true);

        assert_eq!(
            harness.app().transient.pending_widget_reveal,
            Some((overlay_id, target))
        );
        harness.pointer_button(hitbox.center(), false);
        assert_eq!(harness.app().transient.pending_widget_reveal, None);
        harness.frame();

        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(target)
        );
        let panel = harness
            .app()
            .transient
            .widget_panel_rect
            .expect("widget panel is rendered");
        let row = harness
            .widget_selector_rect(target)
            .expect("revealed target row");
        assert!(
            panel.contains(row.center()),
            "selected row should be visible: {row:?} in {panel:?}"
        );
    }

    #[test]
    fn switching_overlays_by_pointer_clears_widget_selection() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let coordinator = harness.app_mut().coordinator_mut().unwrap();
        let first = coordinator.create_overlay("First", 320, 240).unwrap();
        coordinator
            .add_widget(first, TextWidget::new("First widget"))
            .unwrap();
        let second = coordinator.create_overlay("Second", 320, 240).unwrap();
        coordinator
            .add_widget(second, TextWidget::new("Second widget"))
            .unwrap();
        harness.frame();
        assert_eq!(
            harness.app().coordinator().unwrap().selected_overlay_id(),
            Some(second)
        );
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_widget_id()
                .is_some()
        );

        harness
            .pointer_click_control("Overlay selector")
            .expect("open overlay selector");
        let first_option = harness
            .overlay_selector_rect(first)
            .expect("first overlay option is visible");
        harness.pointer_click(first_option.center());

        let coordinator = harness.app().coordinator().unwrap();
        assert_eq!(coordinator.selected_overlay_id(), Some(first));
        assert_eq!(coordinator.selected_widget_id(), None);
        assert_eq!(harness.app().transient.inspector_target, None);
        assert!(harness.has_label("Overlay information"));
    }

    #[test]
    fn deleting_selected_widget_by_pointer_selects_the_same_index_fallback() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        let middle_row = harness
            .widget_selector_rect(ids[1])
            .expect("middle widget row");
        harness.pointer_click(middle_row.center());
        harness
            .pointer_click_control("Delete widget")
            .expect("delete selected widget");

        let coordinator = harness.app().coordinator().unwrap();
        assert!(
            coordinator
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[1])
                .is_none()
        );
        assert_eq!(coordinator.selected_widget_id(), Some(ids[2]));
        assert_eq!(
            harness.app().transient.inspector_target,
            Some((overlay_id, ids[2]))
        );
    }

    #[test]
    fn hover_outline_is_distinct_and_editor_guides_do_not_change_browser_output() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .update_overlay(overlay_id, |overlay| {
                overlay.set_widget_position(ids[0], Position::new(20.0, 20.0))?;
                overlay.set_widget_position(ids[1], Position::new(110.0, 90.0))?;
                overlay.set_widget_position(ids[2], Position::new(220.0, 170.0))
            })
            .unwrap();
        harness.frame();
        harness.frame();
        let before = crate::browser::render(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_overlay()
                .unwrap(),
            0,
        );
        let hover = harness
            .widget_canvas_rect(ids[2])
            .expect("back widget hitbox")
            .center();

        harness.pointer_move(hover);

        let outline_colors: Vec<egui::Color32> = harness
            .shapes()
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect) => Some(rect.stroke.color),
                _ => None,
            })
            .collect();
        assert!(outline_colors.contains(&egui::Color32::from_rgb(38, 198, 218)));
        assert!(outline_colors.contains(&egui::Color32::from_rgb(224, 224, 224)));
        assert_eq!(
            crate::browser::render(
                harness
                    .app()
                    .coordinator()
                    .unwrap()
                    .selected_overlay()
                    .unwrap(),
                0,
            ),
            before
        );

        harness.pointer_click(hover);
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[2])
        );
        harness.frame();
        let final_editor_outline = harness
            .shapes()
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.stroke.color == HOVER_OUTLINE_COLOR
                        || rect.stroke.color == SELECTION_OUTLINE_COLOR =>
                {
                    Some(rect.stroke.color)
                }
                _ => None,
            })
            .last();
        assert_eq!(
            final_editor_outline,
            Some(SELECTION_OUTLINE_COLOR),
            "selection outline should remain visible over hover"
        );
        assert_eq!(
            crate::browser::render(
                harness
                    .app()
                    .coordinator()
                    .unwrap()
                    .selected_overlay()
                    .unwrap(),
                0,
            ),
            before
        );
    }

    #[test]
    fn selection_change_clears_stale_drag() {
        let mut harness = ScenarioHarness::new_with_size(
            BootstrapOutcome::Ready(ready_app()),
            egui::vec2(960.0, 1_200.0),
        );
        let (overlay_id, ids) = setup_widgets(&mut harness);
        {
            let coordinator = harness.app_mut().coordinator_mut().unwrap();
            coordinator
                .update_overlay(overlay_id, |overlay| {
                    overlay.set_widget_position(ids[0], Position::new(40.0, 30.0))?;
                    overlay.set_widget_position(ids[1], Position::new(180.0, 90.0))
                })
                .unwrap();
        }
        harness.frame();
        let canvas_size = crate::model::CanvasSize::new(320, 240).unwrap();
        let hitbox = harness
            .control_rect("Canvas preview")
            .expect("selected widget preview hitbox");
        let grab = hitbox.min + egui::vec2(3.0, 4.0);
        let moved_pointer = grab + egui::vec2(50.0, 35.0);
        harness.pointer_move(grab);
        harness.pointer_button(grab, true);
        assert!(harness.app().transient.preview_drag.is_some());
        let pressed_preview_rect = harness
            .app()
            .transient
            .preview_rect
            .expect("canvas preview while pointer is pressed");
        let pressed_scale = preview_scale(
            canvas_size,
            pressed_preview_rect.width(),
            pressed_preview_rect.height(),
        );
        let pressed_origin = canvas_to_preview(
            pressed_preview_rect.min,
            Position::new(40.0, 30.0),
            pressed_scale,
        );
        harness.pointer_move(moved_pointer);
        assert!(harness.app().transient.preview_drag.is_some());
        assert_eq!(
            harness.app().transient.preview_drag.unwrap().pointer_offset,
            grab - pressed_origin
        );
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_some()
        );
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .position(),
            preview_to_canvas(
                pressed_preview_rect.min,
                moved_pointer,
                grab - pressed_origin,
                pressed_scale,
                canvas_size
            )
        );

        // The selector click changes the authoritative target while the pointer
        // remains down; the next frame must not apply the old drag to ids[1].
        let second_rect = harness
            .widget_selector_rect(ids[1])
            .expect("second selector row shape");
        harness.pointer_move(second_rect.center());
        harness.pointer_button(second_rect.center(), true);
        harness.pointer_button(second_rect.center(), false);
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[1])
        );
        assert!(harness.app().transient.preview_drag.is_none());
        let position_after_selection = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[1])
            .unwrap()
            .position();
        harness.pointer_move(egui::pos2(-500.0, -500.0));
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[1])
                .unwrap()
                .position(),
            position_after_selection
        );
        harness.pointer_button(moved_pointer, false);
        assert!(harness.app().transient.preview_drag.is_none());
    }

    #[test]
    fn drag_commit_cancel_and_live_updates() {
        let mut harness = ScenarioHarness::new_with_size(
            BootstrapOutcome::Ready(ready_app()),
            egui::vec2(960.0, 1200.0),
        );
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .update_overlay(overlay_id, |overlay| {
                overlay.set_widget_position(ids[0], Position::new(40.0, 30.0))
            })
            .unwrap();
        harness.frame();
        let start = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .position();
        let hit = harness.control_rect("Canvas preview").unwrap();
        let from = hit.min + egui::vec2(3.0, 4.0);
        let to = from + egui::vec2(50.0, 35.0);
        harness.pointer_move(from);
        harness.pointer_button(from, true);
        harness.pointer_move(to);
        let live = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .position();
        assert_ne!(live, start);
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .unwrap()
                .field,
            "canvas_position"
        );
        harness.key(egui::Key::Escape, true);
        harness.frame();
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .position(),
            start
        );
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_none()
        );
        harness.pointer_button(to, false);
    }

    #[test]
    fn widget_mutations_use_ids_and_keep_inspector_target_stable() {
        let mut coordinator = ready_app();
        let overlay_id = coordinator.create_overlay("Live", 320, 240).unwrap();
        let first = coordinator
            .add_widget(overlay_id, TextWidget::new("First"))
            .unwrap();
        let second = coordinator
            .add_widget(overlay_id, TextWidget::new("Second"))
            .unwrap();
        assert_eq!(coordinator.selected_widget_id(), Some(second));
        let name = edit_target(overlay_id, second, "name");
        apply_edit_value(
            &mut coordinator,
            &name,
            EditValue::Name("Renamed".to_owned()),
        )
        .unwrap();
        coordinator.commit_edit(&name).unwrap();
        coordinator
            .update_overlay(overlay_id, |overlay| {
                overlay.set_widget_content(second, "Changed")?;
                overlay.set_widget_font_family(second, FontFamily::JetBrainsMono)?;
                overlay.set_widget_position(second, Position::new(123.0, 45.0))?;
                overlay.set_widget_font_size(second, 42.0)?;
                overlay.set_widget_color(second, Color::rgba(10, 20, 30, 128))?;
                overlay.set_widget_alignment(second, Alignment::Center)
            })
            .unwrap();
        assert_eq!(
            coordinator
                .overlay(overlay_id)
                .unwrap()
                .widget(second)
                .unwrap()
                .name(),
            "Renamed"
        );
        assert_eq!(
            coordinator
                .overlay(overlay_id)
                .unwrap()
                .widget(second)
                .unwrap()
                .font_family(),
            FontFamily::JetBrainsMono
        );
        coordinator
            .move_widget_backward(overlay_id, second)
            .unwrap();
        assert_eq!(coordinator.selected_widget_id(), Some(second));
        coordinator.delete_selected_widget().unwrap();
        assert_eq!(coordinator.selected_widget_id(), Some(first));
    }

    #[test]
    fn no_widget_selection_shows_overlay_information_without_implicit_selection() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (_overlay_id, _ids) = setup_widgets(&mut harness);
        harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .clear_widget_selection();
        harness.frame();
        assert!(harness.has_label("Overlay information"));
        assert!(harness.has_label("No widget selected."));
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            None
        );
    }

    #[test]
    fn grouped_inspector_text_edits() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        harness
            .replace_text_control("Widget name", "Newest")
            .unwrap();
        let coordinator = harness.app().coordinator().unwrap();
        assert_eq!(
            coordinator
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .name(),
            "Newest"
        );
        assert!(coordinator.pending_edit_target().is_some());
        harness.key(egui::Key::Enter, true);
        harness.frame();
        assert!(harness.app().coordinator().unwrap().can_undo());
        harness.pointer_click_control("Widget content").unwrap();
        harness.event(egui::Event::Text("line one".to_owned()));
        harness.event(egui::Event::Text("\nline two".to_owned()));
        harness.frame();
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_some()
        );
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .content()
                .contains("line two")
        );
    }

    #[test]
    fn color_slider_drag_updates_live_and_undoes_as_one_edit() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        let before = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .color();
        let hub = harness.app().coordinator().unwrap().hub();
        let initial_revision = hub.revision(overlay_id).unwrap().unwrap();

        harness.pointer_click_control("Color").unwrap();
        harness.frame();
        let slider = harness.control_rect("Color R slider").unwrap();
        let start = egui::pos2(slider.left() + slider.width() * 0.85, slider.center().y);
        let first_move = egui::pos2(slider.left() + slider.width() * 0.65, slider.center().y);
        let second_move = egui::pos2(slider.left() + slider.width() * 0.35, slider.center().y);
        harness.pointer_move(start);
        harness.pointer_button(start, true);
        harness.pointer_move(first_move);
        let after_first_move = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .color();
        let first_revision = hub.revision(overlay_id).unwrap().unwrap();
        assert_ne!(after_first_move, before);
        assert!(first_revision > initial_revision);

        harness.pointer_move(second_move);
        let after_second_move = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .color();
        assert_ne!(after_second_move, after_first_move);
        assert!(hub.revision(overlay_id).unwrap().unwrap() > first_revision);
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .unwrap()
                .field,
            "color"
        );

        harness.pointer_button(second_move, false);
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_none()
        );
        assert!(harness.app().coordinator().unwrap().can_undo());
        assert!(harness.app_mut().coordinator_mut().unwrap().undo().unwrap());
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .color(),
            before
        );
    }

    #[test]
    fn color_channel_numeric_edit_is_one_undoable_change() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        let before = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .color();
        harness.pointer_click_control("Color").unwrap();
        harness.frame();
        harness
            .replace_number_control("Color R", "123")
            .expect("edit red channel through egui input");
        harness.frame();
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .color()
                .red(),
            123
        );
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_none()
        );
        assert!(harness.app_mut().coordinator_mut().unwrap().undo().unwrap());
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .color(),
            before
        );
    }

    #[test]
    fn numeric_drag_commits_on_release_and_cancels_with_escape() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        let before_size = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .font_size();
        let size_rect = harness.control_rect("Font size").unwrap();
        let size_start = size_rect.center();
        let size_end = size_start - egui::vec2(20.0, 0.0);
        harness.pointer_move(size_start);
        harness.pointer_button(size_start, true);
        harness.pointer_move(size_end);
        let dragged_size = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .font_size();
        assert_ne!(dragged_size, before_size);
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_some()
        );
        harness.pointer_button(size_end, false);
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_none()
        );
        assert!(harness.app_mut().coordinator_mut().unwrap().undo().unwrap());
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .font_size(),
            before_size
        );

        harness.frame();
        let before_position = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .position();
        let x_rect = harness.control_rect("Position X").unwrap();
        let x_start = x_rect.center();
        let x_end = x_start + egui::vec2(30.0, 0.0);
        harness.pointer_move(x_start);
        harness.pointer_button(x_start, true);
        harness.pointer_move(x_end);
        let dragged_position = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .position();
        assert_ne!(dragged_position, before_position);
        harness.key(egui::Key::Escape, true);
        harness.frame();
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .position(),
            before_position
        );
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_none()
        );
        harness.pointer_button(x_end, false);
    }

    #[test]
    fn quit_while_text_focused_resolves_pending_edit() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        harness.pointer_click_control("Widget content").unwrap();
        harness.frame_with_close(vec![egui::Event::Text(" final".to_owned())]);

        assert!(harness.app().transient.close_prompt_open);
        assert_eq!(harness.close_prompt_count(), 1);
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .content()
                .ends_with(" final")
        );
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_none()
        );
    }

    #[test]
    fn close_prompt_save_discard_cancel_matrix() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (_overlay_id, _ids) = setup_widgets(&mut harness);
        harness.frame();
        harness.click("Add").unwrap();
        harness.frame();
        let name_rect = harness.control_rect("Widget name").unwrap();
        let add_rect = harness.control_rect("Add text widget").unwrap();
        harness.frame_close_requested();
        assert!(harness.app().transient.close_prompt_open);
        assert!(
            harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::CancelClose)
        );
        assert_eq!(harness.close_prompt_count(), 1);
        harness.frame_close_requested();
        assert_eq!(harness.close_prompt_count(), 1);
        let widgets_before = harness
            .app()
            .coordinator()
            .unwrap()
            .selected_overlay()
            .unwrap()
            .widgets()
            .to_vec();
        harness.pointer_click(add_rect.center());
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_overlay()
                .unwrap()
                .widgets(),
            widgets_before
        );
        harness.pointer_click(name_rect.center());
        harness.event(egui::Event::Text("Blocked edit".to_owned()));
        harness.key_with_modifiers(
            egui::Key::D,
            true,
            if cfg!(target_os = "macos") {
                egui::Modifiers::COMMAND
            } else {
                egui::Modifiers::CTRL
            },
        );
        harness.frame_close_requested();
        assert!(
            harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::CancelClose)
        );
        harness.pointer_click_control("Close/Cancel").unwrap();
        harness.frame();
        assert!(!harness.app().transient.close_prompt_open);
        harness.frame_close_requested();
        harness.pointer_click_control("Close/Discard").unwrap();
        assert!(
            harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::Close)
        );
    }

    #[test]
    fn focused_fields_suppress_document_shortcuts_and_global_save_remains_available() {
        #[derive(Clone, Copy)]
        enum Field {
            FontSize,
            PositionX,
            ColorRed,
            CreateName,
            CreateWidth,
            CreateHeight,
            RenameName,
            SettingsPort,
        }

        let primary = if cfg!(target_os = "macos") {
            egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::CTRL
        };
        let cases = [
            (Field::FontSize, "Font size"),
            (Field::PositionX, "Position X"),
            (Field::ColorRed, "Color R"),
            (Field::CreateName, "Create name"),
            (Field::CreateWidth, "Create width"),
            (Field::CreateHeight, "Create height"),
            (Field::RenameName, "Rename name"),
            (Field::SettingsPort, "Settings port"),
        ];

        for (field, label) in cases {
            let directory = tempfile::tempdir().expect("temporary workspace directory");
            let overlay_path = directory.path().join("overlays.json");
            let settings_path = directory.path().join("settings.json");
            let mut coordinator = HeadlessCoordinator::empty(Store::at(&overlay_path));
            let overlay_id = coordinator.create_overlay("Live", 320, 240).unwrap();
            let widget_ids = [
                coordinator
                    .add_widget(overlay_id, TextWidget::new("Front"))
                    .unwrap(),
                coordinator
                    .add_widget(overlay_id, TextWidget::new("Back"))
                    .unwrap(),
            ];
            coordinator.select_widget(widget_ids[0]).unwrap();
            coordinator.save().unwrap();
            coordinator
                .update_overlay(overlay_id, |overlay| {
                    overlay.set_widget_content(widget_ids[0], "Unsaved")
                })
                .unwrap();
            coordinator
                .update_overlay(overlay_id, |overlay| {
                    overlay.set_widget_content(widget_ids[0], "Current")
                })
                .unwrap();
            assert!(coordinator.undo().unwrap());
            assert!(coordinator.can_undo());
            assert!(coordinator.can_redo());
            assert!(coordinator.is_dirty());
            let baseline = coordinator.overlays().to_vec();
            let settings = SettingsState::from_settings(
                SettingsStore::at(&settings_path),
                Settings::new(4_000).unwrap(),
            );
            let mut harness =
                ScenarioHarness::new_with_settings(BootstrapOutcome::Ready(coordinator), settings);
            harness.frame();

            match field {
                Field::FontSize | Field::PositionX => {}
                Field::ColorRed => {
                    harness.pointer_click_control("Color").unwrap();
                    harness.frame();
                }
                Field::CreateName | Field::CreateWidth | Field::CreateHeight => {
                    harness.pointer_click_control("Create overlay").unwrap();
                    harness.frame();
                }
                Field::RenameName => {
                    harness.pointer_click_control("Rename").unwrap();
                    harness.frame();
                }
                Field::SettingsPort => {
                    harness
                        .pointer_click_control("Local server settings")
                        .unwrap();
                    harness.frame();
                }
            }
            harness
                .pointer_click_control(label)
                .unwrap_or_else(|error| panic!("focus {label}: {error}"));
            let focused = harness
                .context
                .memory(|memory| memory.focused())
                .unwrap_or_else(|| panic!("{label} must receive pointer focus"));
            assert!(
                egui::widgets::text_edit::TextEditState::load(&harness.context, focused).is_some(),
                "{label} focus must belong to a real egui text editor"
            );

            let assert_document_unchanged = |harness: &ScenarioHarness, dirty: bool| {
                let coordinator = harness.app().coordinator().unwrap();
                assert_eq!(
                    coordinator.overlays(),
                    baseline,
                    "{label} changed document data"
                );
                assert_eq!(coordinator.selected_overlay_id(), Some(overlay_id));
                assert_eq!(coordinator.selected_widget_id(), Some(widget_ids[0]));
                assert!(coordinator.can_undo(), "{label} lost undo history");
                assert!(coordinator.can_redo(), "{label} lost redo history");
                assert_eq!(
                    coordinator.is_dirty(),
                    dirty,
                    "{label} changed saved status"
                );
            };

            // Keep destructive text-edit keys at their no-op caret boundaries;
            // the assertions below are about document shortcuts, not text entry.
            harness.key(egui::Key::Home, true);
            harness.key(egui::Key::Backspace, true);
            assert_document_unchanged(&harness, true);
            harness.key(egui::Key::End, true);
            harness.key(egui::Key::Delete, true);
            assert_document_unchanged(&harness, true);

            harness.key_with_modifiers(egui::Key::Z, true, primary);
            assert_document_unchanged(&harness, true);
            harness.key_with_modifiers(egui::Key::Z, true, primary | egui::Modifiers::SHIFT);
            assert_document_unchanged(&harness, true);
            harness.key_with_modifiers(egui::Key::D, true, primary);
            assert_document_unchanged(&harness, true);

            harness.key_with_modifiers(egui::Key::S, true, primary);
            assert_document_unchanged(&harness, false);
            assert_eq!(
                Store::at(&overlay_path).load().unwrap(),
                baseline,
                "global Save while {label} is focused must write the workspace"
            );
        }
    }

    #[test]
    fn global_save_commits_and_persists_pending_inspector_edit_while_text_focused() {
        let directory = tempfile::tempdir().expect("temporary workspace directory");
        let overlay_path = directory.path().join("overlays.json");
        let mut coordinator = HeadlessCoordinator::empty(Store::at(&overlay_path));
        let overlay_id = coordinator.create_overlay("Live", 320, 240).unwrap();
        let widget_id = coordinator
            .add_widget(overlay_id, TextWidget::new("Front"))
            .unwrap();
        coordinator.save().unwrap();
        let settings = SettingsState::from_settings(
            SettingsStore::at(directory.path().join("settings.json")),
            Settings::new(4_000).unwrap(),
        );
        let mut harness =
            ScenarioHarness::new_with_settings(BootstrapOutcome::Ready(coordinator), settings);
        harness.frame();
        harness
            .replace_text_control("Widget content", "Saved while focused")
            .unwrap();

        let coordinator = harness.app().coordinator().unwrap();
        assert!(coordinator.pending_edit_target().is_some());
        assert!(coordinator.is_dirty());
        harness.key_with_modifiers(
            egui::Key::S,
            true,
            if cfg!(target_os = "macos") {
                egui::Modifiers::COMMAND
            } else {
                egui::Modifiers::CTRL
            },
        );

        let coordinator = harness.app().coordinator().unwrap();
        assert!(coordinator.pending_edit_target().is_none());
        assert!(!coordinator.is_dirty());
        assert_eq!(
            coordinator
                .overlay(overlay_id)
                .unwrap()
                .widget(widget_id)
                .unwrap()
                .content(),
            "Saved while focused"
        );
        assert_eq!(
            Store::at(&overlay_path).load().unwrap()[0]
                .widget(widget_id)
                .unwrap()
                .content(),
            "Saved while focused"
        );
    }

    #[test]
    fn color_popup_open_and_close_preserve_document_history() {
        let directory = tempfile::tempdir().expect("temporary workspace directory");
        let mut coordinator =
            HeadlessCoordinator::empty(Store::at(directory.path().join("overlays.json")));
        let overlay_id = coordinator.create_overlay("Live", 320, 240).unwrap();
        let widget_id = coordinator
            .add_widget(overlay_id, TextWidget::new("Front"))
            .unwrap();
        coordinator.rename_overlay(overlay_id, "Temporary").unwrap();
        assert!(coordinator.undo().unwrap());
        let baseline = coordinator.overlays().to_vec();
        assert!(coordinator.can_undo());
        assert!(coordinator.can_redo());
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(coordinator));
        harness.frame();

        harness.pointer_click_control("Color").unwrap();
        harness.frame();
        assert!(harness.control_rect("Color R slider").is_some());
        harness.pointer_click_control("Color").unwrap();
        harness.frame();

        let coordinator = harness.app().coordinator().unwrap();
        assert!(harness.control_rect("Color R slider").is_none());
        assert_eq!(coordinator.overlays(), baseline);
        assert_eq!(coordinator.selected_widget_id(), Some(widget_id));
        assert!(coordinator.can_undo());
        assert!(coordinator.can_redo());
    }

    #[test]
    fn focus_safe_shortcut_matrix_and_text_native_undo_and_newline() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        let primary = if cfg!(target_os = "macos") {
            egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::CTRL
        };
        harness.key_with_modifiers(egui::Key::D, true, primary);
        harness.frame();
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_overlay()
                .unwrap()
                .widgets()
                .len(),
            4
        );
        harness.key(egui::Key::Delete, true);
        harness.frame();
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_overlay()
                .unwrap()
                .widgets()
                .len(),
            3
        );
        harness
            .replace_text_control("Widget content", "first\nsecond")
            .unwrap();
        let original = harness
            .app()
            .coordinator()
            .unwrap()
            .overlay(overlay_id)
            .unwrap()
            .widget(ids[0])
            .unwrap()
            .content()
            .to_owned();
        assert!(original.contains("second"));
        let primary = if cfg!(target_os = "macos") {
            egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::CTRL
        };
        harness.key_with_modifiers(egui::Key::Z, true, primary);
        harness.frame();
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[0])
        );
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .content(),
            "Front"
        );
        harness.key(egui::Key::Enter, true);
        harness.event(egui::Event::Text("\ncontinued".to_owned()));
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .content()
                .contains("continued")
        );
        harness.pointer_click_control("Widget name").unwrap();
        harness.key_with_modifiers(egui::Key::D, true, primary);
        harness.frame();
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[0])
        );
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widgets()
                .len(),
            3
        );
        harness.key(egui::Key::Delete, true);
        harness.frame();
        assert_eq!(
            harness.app().coordinator().unwrap().selected_widget_id(),
            Some(ids[0])
        );
        harness.key_with_modifiers(egui::Key::S, true, primary);
        harness.frame();
        assert!(!harness.app().coordinator().unwrap().is_dirty());

        harness.pointer_click_control("Widget name").unwrap();
        harness.key_with_modifiers(egui::Key::A, true, primary);
        harness.event(egui::Event::Text("Obsolete".to_owned()));
        harness.key(egui::Key::Enter, true);
        harness.frame();
        harness.pointer_click_control("Edit menu").unwrap();
        harness.frame();
        harness.pointer_click_control("Edit/Undo").unwrap();
        harness.frame();
        harness.pointer_click_control("Widget name").unwrap();
        harness.key_with_modifiers(egui::Key::Z, true, primary);
        harness.frame();
        assert_ne!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .name(),
            "Obsolete"
        );
    }

    #[test]
    fn document_undo_cannot_restore_native_text_undo_from_previous_selection() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        let primary = if cfg!(target_os = "macos") {
            egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::CTRL
        };

        harness.pointer_click_control("Widget name").unwrap();
        harness.key_with_modifiers(egui::Key::A, true, primary);
        harness.event(egui::Event::Text("Native edit".to_owned()));
        harness.key(egui::Key::Enter, true);
        harness.frame();
        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .name(),
            "Native edit"
        );

        let second_row = harness.widget_selector_rect(ids[1]).unwrap();
        harness.pointer_click(second_row.center());
        harness.pointer_click_control("Widget name").unwrap();
        harness.key_with_modifiers(egui::Key::A, true, primary);
        harness.event(egui::Event::Text("Document edit".to_owned()));
        harness.key(egui::Key::Enter, true);
        harness.frame();
        harness.pointer_click_control("Edit menu").unwrap();
        harness.frame();
        harness.pointer_click_control("Edit/Undo").unwrap();
        harness.frame();

        let first_row = harness.widget_selector_rect(ids[0]).unwrap();
        harness.pointer_click(first_row.center());
        harness.pointer_click_control("Widget name").unwrap();
        harness.key_with_modifiers(egui::Key::Z, true, primary);
        harness.frame();

        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .name(),
            "Native edit"
        );
    }

    #[test]
    fn issue24_menu_actions_and_enabled_states() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        harness.pointer_click_control("Edit menu").unwrap();
        harness.frame();
        assert_eq!(harness.menu_action_is_enabled("Undo"), Some(true));
        assert_eq!(harness.menu_action_is_enabled("Redo"), Some(false));
        harness
            .replace_text_control("Widget name", "Menu edit")
            .unwrap();
        harness.key(egui::Key::Enter, true);
        harness.frame();
        harness.pointer_click_control("Edit menu").unwrap();
        harness.frame();
        assert_eq!(harness.menu_action_is_enabled("Undo"), Some(true));
        harness.pointer_click_control("Edit/Undo").unwrap();
        harness.frame();
        assert_ne!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .name(),
            "Menu edit"
        );
    }

    fn ping_server(address: std::net::SocketAddr) -> String {
        use std::io::{Read, Write};
        use std::net::TcpStream;
        use std::time::Duration;

        let mut stream = TcpStream::connect(address).expect("connect to local server");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set bounded ping timeout");
        stream
            .write_all(b"GET /ping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .expect("send ping request");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("read ping response");
        response
    }

    #[test]
    fn close_save_failure_keeps_work_and_server() {
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("is-a-directory");
        std::fs::create_dir(&store_path).unwrap();
        let mut coordinator = HeadlessCoordinator::empty(Store::at(&store_path));
        let overlay_id = coordinator.create_overlay("Live", 320, 240).unwrap();
        coordinator
            .add_widget(overlay_id, TextWidget::new("Front"))
            .unwrap();
        coordinator
            .add_widget(overlay_id, TextWidget::new("Middle"))
            .unwrap();
        coordinator
            .add_widget(overlay_id, TextWidget::new("Back"))
            .unwrap();
        let server = crate::server::start_on_port_with_hub(0, coordinator.hub()).unwrap();
        let address = server.local_addr();
        coordinator.set_server_address(address);
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(coordinator));
        harness.frame();
        harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .add_widget(overlay_id, TextWidget::new("Unsaved"))
            .unwrap();
        harness.frame();
        assert!(ping_server(address).contains("200 OK\r\n"));
        assert!(ping_server(address).ends_with("pong"));

        harness.frame_close_requested();
        assert!(harness.app().transient.close_prompt_open);
        harness.pointer_click_control("Close/Cancel").unwrap();
        assert!(!harness.app().transient.close_prompt_open);
        assert!(ping_server(address).ends_with("pong"));

        harness.frame_close_requested();
        harness.pointer_click_control("Close/Save").unwrap();
        assert!(harness.app().transient.close_prompt_open);
        assert!(harness.app().coordinator().unwrap().is_dirty());
        assert!(harness.app().coordinator().unwrap().can_undo());
        let error = harness.app().transient.close_error.clone().unwrap();
        assert!(harness.has_label(&error));
        assert!(
            !harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::Close)
        );
        assert!(ping_server(address).ends_with("pong"));
        server.shutdown().unwrap();
    }

    #[test]
    fn pending_close_failure_cancels_native_close_and_preserves_live_work() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, widgets) = setup_widgets(&mut harness);
        let target = edit_target(overlay_id, widgets[0], "content");
        let mismatched = edit_target(overlay_id, widgets[1], "content");
        {
            let coordinator = harness.app_mut().coordinator_mut().unwrap();
            coordinator.begin_edit(target.clone()).unwrap();
            coordinator
                .update_edit(&target, |overlay| {
                    overlay.set_widget_content(widgets[0], "live work")
                })
                .unwrap();
        }
        harness.app_mut().transient.active_edit = Some(mismatched);
        harness.frame_close_requested();

        assert!(harness.app().transient.close_prompt_open);
        assert!(
            harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::CancelClose)
        );
        assert!(
            !harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::Close)
        );
        let error = harness.app().transient.close_error.clone().unwrap();
        assert!(harness.has_label(&error));
        let coordinator = harness.app().coordinator().unwrap();
        assert_eq!(coordinator.pending_edit_target(), Some(&target));
        assert!(coordinator.is_dirty());
        assert_eq!(
            coordinator
                .overlay(overlay_id)
                .unwrap()
                .widget(widgets[0])
                .unwrap()
                .content(),
            "live work"
        );

        harness.pointer_click_control("Close/Cancel").unwrap();
        assert!(!harness.app().transient.close_prompt_open);
        let coordinator = harness.app().coordinator().unwrap();
        assert_eq!(coordinator.pending_edit_target(), Some(&target));
        assert_eq!(
            coordinator
                .overlay(overlay_id)
                .unwrap()
                .widget(widgets[0])
                .unwrap()
                .content(),
            "live work"
        );
    }

    #[test]
    fn clean_close_does_not_open_unsaved_prompt() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.frame();
        harness.frame_close_requested();

        assert!(!harness.app().transient.close_prompt_open);
        assert_eq!(harness.close_prompt_count(), 0);
        assert!(
            harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::Close)
        );
    }

    #[test]
    fn blocked_startup_close_does_not_open_save_prompt() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("overlays.json");
        std::fs::write(&source_path, b"not json").unwrap();
        let outcome = HeadlessCoordinator::bootstrap_outcome(Store::at(&source_path));
        let mut harness = ScenarioHarness::new(outcome);
        harness.frame();
        harness.frame_close_requested();

        assert!(!harness.app().transient.close_prompt_open);
        assert_eq!(harness.close_prompt_count(), 0);
        assert!(
            !harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::CancelClose)
        );
    }

    #[test]
    fn close_save_writes_the_new_baseline_before_closing() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("overlays.json");
        let mut coordinator = HeadlessCoordinator::empty(Store::at(&path));
        let overlay_id = coordinator.create_overlay("Before", 320, 240).unwrap();
        let widget_id = coordinator
            .add_widget(overlay_id, TextWidget::new("before"))
            .unwrap();
        coordinator.save().unwrap();
        coordinator
            .update_overlay(overlay_id, |overlay| {
                overlay.set_widget_content(widget_id, "saved")
            })
            .unwrap();
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(coordinator));
        harness.frame();
        harness.frame_close_requested();
        assert!(harness.app().transient.close_prompt_open);
        harness.pointer_click_control("Close/Save").unwrap();

        let coordinator = harness.app().coordinator().unwrap();
        assert!(!coordinator.is_dirty());
        assert_eq!(
            crate::persistence::load(&path).unwrap(),
            coordinator.overlays()
        );
        assert!(
            harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::Close)
        );
    }

    #[test]
    fn close_discard_does_not_write_overlay_or_settings_files() {
        let directory = tempfile::tempdir().unwrap();
        let overlay_path = directory.path().join("overlays.json");
        let settings_path = directory.path().join("settings.json");
        let settings_store = SettingsStore::at(&settings_path);
        settings_store.save(Settings::new(4_321).unwrap()).unwrap();
        let settings_before = std::fs::read(&settings_path).unwrap();
        let mut coordinator = HeadlessCoordinator::empty(Store::at(&overlay_path));
        let overlay_id = coordinator.create_overlay("Saved", 320, 240).unwrap();
        let widget_id = coordinator
            .add_widget(overlay_id, TextWidget::new("baseline"))
            .unwrap();
        coordinator.save().unwrap();
        let overlay_before = std::fs::read(&overlay_path).unwrap();
        coordinator
            .update_overlay(overlay_id, |overlay| {
                overlay.set_widget_content(widget_id, "discarded")
            })
            .unwrap();
        let settings = SettingsState::from_settings(settings_store, Settings::new(4_321).unwrap());
        let mut harness =
            ScenarioHarness::new_with_settings(BootstrapOutcome::Ready(coordinator), settings);
        harness.frame();
        harness.frame_close_requested();
        harness.pointer_click_control("Close/Discard").unwrap();

        assert_eq!(std::fs::read(&overlay_path).unwrap(), overlay_before);
        assert_eq!(std::fs::read(&settings_path).unwrap(), settings_before);
        assert!(
            harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::Close)
        );
    }

    #[test]
    fn file_quit_while_text_focused_resolves_pending_edit() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        harness.pointer_click_control("Widget content").unwrap();
        harness.event(egui::Event::Text(" final".to_owned()));
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_some()
        );
        harness.pointer_click_control("File menu").unwrap();
        harness.pointer_click_control("File/Quit").unwrap();

        assert!(harness.app().transient.close_prompt_open);
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .content()
                .ends_with(" final")
        );
        assert!(
            harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::CancelClose)
        );
        assert!(
            !harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::Close)
        );
        harness.frame();
        harness.pointer_click_control("Close/Cancel").unwrap();
        assert!(!harness.app().transient.close_prompt_open);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn command_q_while_text_focused_resolves_pending_edit() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        let (overlay_id, ids) = setup_widgets(&mut harness);
        harness.frame();
        harness.pointer_click_control("Widget content").unwrap();
        harness.frame_events(vec![
            egui::Event::Text(" final".to_owned()),
            egui::Event::Key {
                key: egui::Key::Q,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
        ]);

        assert!(harness.app().transient.close_prompt_open);
        assert!(
            harness
                .emitted_close_commands()
                .contains(&egui::ViewportCommand::CancelClose)
        );
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .overlay(overlay_id)
                .unwrap()
                .widget(ids[0])
                .unwrap()
                .content()
                .ends_with(" final")
        );
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .pending_edit_target()
                .is_none()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_event_loop_disables_default_menu() {
        let mut builder = winit::event_loop::EventLoop::<eframe::UserEvent>::with_user_event();
        configure_macos_event_loop(&mut builder);
    }

    #[test]
    fn gui_production_uses_ordered_id_based_inspector() {
        let source = include_str!("gui.rs");
        let production = source
            .split("/// A native-window-free egui scenario harness")
            .next()
            .expect("production adapter precedes tests");
        assert!(production.contains("widgets().to_vec()"));
        assert!(production.contains("iter().rev()"));
        assert!(production.contains("selected_widget()"));
        assert!(production.contains("set_widget_font_family"));
        assert!(production.contains("Duplicate"));
        assert!(production.contains("Delete widget"));
        assert!(production.contains("Forward"));
        assert!(production.contains("Backward"));
        assert!(!production.contains("text_widget()"));
        assert!(!production.contains("revision()"));
        assert!(production.contains("Open output"));
        assert!(production.contains("Save port for next launch"));
    }
}
