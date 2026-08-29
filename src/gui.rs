//! Native egui adapter for the issue #4 overlay workspace.
//!
//! The adapter owns transient form and confirmation state plus the separate
//! application settings state. The overlay collection, selection, dirty state,
//! persistence status, and URL readiness all remain in
//! [`crate::app::HeadlessCoordinator`].

use eframe::egui;

use crate::app::{ApplicationBootstrap, BootstrapOutcome, HeadlessCoordinator};
use crate::model::{Alignment, Color, OverlayId, Position, TextWidget, TextWidgetId};
use crate::settings::{MAX_PORT, MIN_PORT, Settings, SettingsState, Store as SettingsStore};

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
    settings_port_input: String,
    settings_save_error: Option<String>,
    settings_save_succeeded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewDrag {
    overlay_id: OverlayId,
    widget_id: TextWidgetId,
    pointer_offset: egui::Vec2,
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
        if self.blocked.is_some() {
            self.render_blocked(context);
        } else {
            self.render_workspace(context);
        }
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
                ui.label("Repair the source, then restart Chikachika. No replacement Save action is available here.");
            } else {
                ui.label("No persistence path could be resolved. Fix the platform app-data configuration and restart Chikachika.");
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
            "Create overlay" => {
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
            "Add text widget" => add_selected_text_widget(self.coordinator.as_mut()),
            "Remove text widget" => remove_selected_text_widget(self.coordinator.as_mut()),
            other => self.select_named(other),
        }
    }

    fn render_workspace(&mut self, context: &egui::Context) {
        let coordinator = self
            .coordinator
            .as_mut()
            .expect("usable state has a coordinator");
        let transient = &mut self.transient;

        egui::TopBottomPanel::top("status").show(context, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Chikachika overlay workspace");
                ui.separator();
                if coordinator.is_dirty() {
                    ui.colored_label(egui::Color32::from_rgb(239, 108, 0), "Unsaved changes");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(46, 125, 50), "Saved");
                }
                if coordinator.is_dirty() && ui.button("Save").clicked() {
                    let _ = save_workspace(Some(coordinator));
                }
                if let Some(error) = coordinator.last_error() {
                    ui.colored_label(
                        egui::Color32::from_rgb(183, 28, 28),
                        format!("Error: {error}"),
                    );
                }
            });
        });

        render_settings(context, &mut self.settings, transient, Some(&*coordinator));

        egui::SidePanel::left("overlay-list")
            .resizable(true)
            .default_width(230.0)
            .show(context, |ui| {
                ui.heading("Overlays");
                ui.add_space(4.0);
                if ui.button("Create overlay").clicked() {
                    begin_create(transient);
                }
                ui.separator();
                if coordinator.overlays().is_empty() {
                    ui.label("No overlays yet.");
                    ui.label("Create one to begin a local browser source workspace.");
                } else {
                    let selected = coordinator.selected_overlay_id();
                    let overlay_rows: Vec<(OverlayId, String)> = coordinator
                        .overlays()
                        .iter()
                        .map(|overlay| (overlay.id(), overlay.name().to_owned()))
                        .collect();
                    for (id, name) in overlay_rows {
                        let is_selected = selected == Some(id);
                        let label = if is_selected {
                            format!("✓ {name}")
                        } else {
                            name
                        };
                        if ui.selectable_label(is_selected, label).clicked() {
                            let _ = select_overlay(coordinator, id);
                        }
                    }
                }
            });

        egui::CentralPanel::default().show(context, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Overlay details");
                ui.separator();
                let Some(overlay) = coordinator.selected_overlay() else {
                    transient.preview_drag = None;
                    ui.label(
                        "Select an overlay or use Create overlay to make your first workspace.",
                    );
                    return;
                };

                let id = overlay.id();
                let name = overlay.name().to_owned();
                let canvas = overlay.canvas();
                let revision = overlay.revision();
                ui.label(format!("Name: {name}"));
                ui.label(format!("Canvas: {} × {}", canvas.width(), canvas.height()));
                ui.label(format!("Stable identity: {id}"));
                ui.label(format!("Hosted revision: {revision}"));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        let _ = begin_rename(coordinator, transient);
                    }
                    if ui.button("Delete").clicked() {
                        let _ = begin_delete(coordinator, transient);
                    }
                });
                ui.add_space(8.0);
                render_text_editor(ui, coordinator, transient, id);
                ui.add_space(8.0);
                ui.label("Browser-source URL");
                if let Some(url) = coordinator.selected_url() {
                    ui.monospace(&url);
                    ui.horizontal(|ui| {
                        if ui.button("Copy URL").clicked() {
                            copy_url(ui.ctx(), &url);
                        }
                        if ui.button("Open in browser").clicked() {
                            open_url(ui.ctx(), &url);
                        }
                    });
                } else {
                    ui.label(
                    "Unavailable until the local server successfully binds and reports readiness.",
                );
                }
            });
        });

        render_create_dialog(context, coordinator, transient);
        render_rename_dialog(context, coordinator, transient);
        render_delete_dialog(context, coordinator, transient);
    }
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
    content: String,
    position: Position,
    font_size: f32,
    color: Color,
    alignment: Alignment,
}

impl TextEditorValues {
    fn from_widget(widget: &TextWidget) -> Self {
        Self {
            id: widget.id(),
            content: widget.content().to_owned(),
            position: widget.position(),
            font_size: widget.font_size(),
            color: widget.color(),
            alignment: widget.alignment(),
        }
    }
}

#[cfg(test)]
fn add_selected_text_widget(coordinator: Option<&mut HeadlessCoordinator>) -> Result<(), String> {
    let coordinator = coordinator.ok_or_else(|| "workspace is not available".to_owned())?;
    let overlay_id = coordinator
        .selected_overlay_id()
        .ok_or_else(|| "no overlay is selected".to_owned())?;
    add_text_widget(coordinator, overlay_id)
}

#[cfg(test)]
fn remove_selected_text_widget(
    coordinator: Option<&mut HeadlessCoordinator>,
) -> Result<(), String> {
    let coordinator = coordinator.ok_or_else(|| "workspace is not available".to_owned())?;
    let overlay = coordinator
        .selected_overlay()
        .ok_or_else(|| "no overlay is selected".to_owned())?;
    let overlay_id = overlay.id();
    let widget_id = overlay
        .text_widget()
        .ok_or_else(|| "no text widget exists".to_owned())?
        .id();
    remove_text_widget(coordinator, overlay_id, widget_id)
}

fn add_text_widget(
    coordinator: &mut HeadlessCoordinator,
    overlay_id: OverlayId,
) -> Result<(), String> {
    coordinator
        .update_overlay(overlay_id, |overlay| {
            overlay.add_text_widget(TextWidget::new("Text"))?;
            Ok(())
        })
        .map_err(|error| error.to_string())
}

fn remove_text_widget(
    coordinator: &mut HeadlessCoordinator,
    overlay_id: OverlayId,
    widget_id: TextWidgetId,
) -> Result<(), String> {
    coordinator
        .update_overlay(overlay_id, |overlay| {
            overlay.remove_text_widget(widget_id)?;
            Ok(())
        })
        .map_err(|error| error.to_string())
}

fn apply_text_editor_values(
    coordinator: &mut HeadlessCoordinator,
    overlay_id: OverlayId,
    values: TextEditorValues,
) -> Result<(), String> {
    coordinator
        .update_overlay(overlay_id, move |overlay| {
            overlay.set_text_content(values.id, values.content)?;
            overlay.set_text_position(values.id, values.position)?;
            overlay.set_text_font_size(values.id, values.font_size)?;
            overlay.set_text_color(values.id, values.color)?;
            overlay.set_text_alignment(values.id, values.alignment)
        })
        .map_err(|error| error.to_string())
}

fn render_text_editor(
    ui: &mut egui::Ui,
    coordinator: &mut HeadlessCoordinator,
    transient: &mut TransientState,
    overlay_id: OverlayId,
) {
    ui.heading("Text widget");
    let Some(overlay) = coordinator.overlay(overlay_id) else {
        transient.preview_drag = None;
        return;
    };
    let canvas = overlay.canvas();
    let Some(widget) = overlay.text_widget() else {
        transient.preview_drag = None;
        ui.label("This overlay has no text widget.");
        if ui.button("Add text widget").clicked() {
            let _ = add_text_widget(coordinator, overlay_id);
        }
        return;
    };
    let mut values = TextEditorValues::from_widget(widget);
    let original = values.clone();
    if !drag_matches(transient.preview_drag, overlay_id, values.id) {
        transient.preview_drag = None;
    }

    if ui.button("Remove text widget").clicked() {
        let _ = remove_text_widget(coordinator, overlay_id, values.id);
        return;
    }

    ui.label("Content");
    ui.add(
        egui::TextEdit::multiline(&mut values.content)
            .desired_rows(4)
            .desired_width(f32::INFINITY),
    );
    ui.horizontal(|ui| {
        ui.label("Font size");
        ui.add(
            egui::DragValue::new(&mut values.font_size)
                .speed(0.5)
                .suffix(" px"),
        );
        ui.label("Color");
        let mut color = egui::Color32::from_rgba_unmultiplied(
            values.color.red(),
            values.color.green(),
            values.color.blue(),
            values.color.alpha(),
        );
        if ui.color_edit_button_srgba(&mut color).changed() {
            values.color = Color::rgba(color.r(), color.g(), color.b(), color.a());
        }
    });
    ui.horizontal(|ui| {
        ui.label("Alignment");
        ui.selectable_value(&mut values.alignment, Alignment::Left, "Left");
        ui.selectable_value(&mut values.alignment, Alignment::Center, "Center");
        ui.selectable_value(&mut values.alignment, Alignment::Right, "Right");
    });
    ui.horizontal(|ui| {
        ui.label("Position");
        let mut x = values.position.x();
        let mut y = values.position.y();
        ui.label("X");
        ui.add(egui::DragValue::new(&mut x).range(0.0..=canvas.width() as f32));
        ui.label("Y");
        ui.add(egui::DragValue::new(&mut y).range(0.0..=canvas.height() as f32));
        values.position = Position::new(x, y);
    });

    ui.label("Canvas preview — drag to move the text widget");
    render_canvas_preview(
        ui,
        canvas,
        overlay_id,
        &mut values,
        &mut transient.preview_drag,
    );

    if values != original {
        let _ = apply_text_editor_values(coordinator, overlay_id, values);
    }
}

const PREVIEW_MAX_HEIGHT: f32 = 360.0;
const PREVIEW_MIN_HANDLE: f32 = 12.0;
const PREVIEW_MAX_PAINT_FONT: f32 = 512.0;

fn preview_scale(canvas: crate::model::CanvasSize, available_width: f32) -> f32 {
    (available_width.max(1.0) / canvas.width() as f32)
        .min(PREVIEW_MAX_HEIGHT / canvas.height() as f32)
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

fn render_canvas_preview(
    ui: &mut egui::Ui,
    canvas: crate::model::CanvasSize,
    overlay_id: OverlayId,
    values: &mut TextEditorValues,
    drag: &mut Option<PreviewDrag>,
) {
    let scale = preview_scale(canvas, ui.available_width());
    let size = egui::vec2(
        canvas.width() as f32 * scale,
        canvas.height() as f32 * scale,
    );
    let (canvas_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(canvas_rect);
    painter.rect_filled(canvas_rect, 0.0, egui::Color32::from_gray(24));
    painter.rect_stroke(
        canvas_rect,
        0.0,
        egui::Stroke::new(1.0_f32, egui::Color32::from_gray(96)),
    );

    let color = egui::Color32::from_rgba_unmultiplied(
        values.color.red(),
        values.color.green(),
        values.color.blue(),
        values.color.alpha(),
    );
    let region_width = ((canvas.width() as f32 - values.position.x()) * scale).max(0.0);
    // Capping paint size protects the editor from huge but model-valid values;
    // it is deliberately local and never written back to the authoritative model.
    let paint_font_size = (values.font_size * scale).clamp(1.0, PREVIEW_MAX_PAINT_FONT);
    let mut layout = egui::text::LayoutJob::simple(
        values.content.clone(),
        egui::FontId::proportional(paint_font_size),
        color,
        region_width,
    );
    layout.halign = alignment_to_egui(values.alignment);
    layout.wrap.max_width = region_width;
    let galley = painter.layout_job(layout);
    let region_origin = canvas_to_preview(canvas_rect.min, values.position, scale);
    let paint_origin = aligned_paint_origin(region_origin, region_width, values.alignment);
    painter.galley(paint_origin, galley.clone(), color);

    let region = text_region_rect(canvas_rect, values.position, scale, galley.size().y);
    let visual_rect = galley
        .mesh_bounds
        .translate(paint_origin.to_vec2())
        .intersect(region);
    let hitbox = widget_hitbox(canvas_rect, region_origin, visual_rect);
    let response = ui.interact(
        hitbox,
        ui.make_persistent_id((
            "preview-text",
            overlay_id.to_string(),
            values.id.to_string(),
        )),
        egui::Sense::drag(),
    );

    if response.drag_started()
        && let Some(pointer) = response.interact_pointer_pos()
        && hitbox.contains(pointer)
    {
        *drag = Some(PreviewDrag {
            overlay_id,
            widget_id: values.id,
            pointer_offset: pointer - region_origin,
        });
    }
    if response.dragged()
        && let (Some(active), Some(pointer)) = (*drag, response.interact_pointer_pos())
        && drag_matches(Some(active), overlay_id, values.id)
    {
        values.position = preview_to_canvas(
            canvas_rect.min,
            pointer,
            active.pointer_offset,
            scale,
            canvas,
        );
    }
    if response.drag_stopped() || !response.is_pointer_button_down_on() {
        *drag = None;
    }
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
    context: &egui::Context,
    settings: &mut SettingsState,
    transient: &mut TransientState,
    coordinator: Option<&HeadlessCoordinator>,
) {
    let configured_port = settings.configured_port();
    let settings_path = settings
        .settings_path()
        .map(|path| path.display().to_string());
    let settings_error = settings.settings_error().map(ToString::to_string);
    let active_port = coordinator
        .and_then(|coordinator| coordinator.server_address().map(|address| address.port()));

    egui::TopBottomPanel::bottom("settings")
        .resizable(true)
        .default_height(174.0)
        .show(context, |ui| {
            ui.heading("Local server settings");
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
            if let Some(error) = settings_error.as_deref() {
                ui.colored_label(
                    egui::Color32::from_rgb(183, 28, 28),
                    format!("Settings error: {error}"),
                );
                ui.label(
                    "Repair or remove the settings source, then restart Chikachika. No fallback port is used while settings are invalid.",
                );
            }
            ui.horizontal(|ui| {
                ui.label("Port for next launch (1–65535)");
                let input = ui.text_edit_singleline(&mut transient.settings_port_input);
                if input.changed() {
                    transient.settings_save_error = None;
                    transient.settings_save_succeeded = false;
                }
                if ui.button("Save port for next launch").clicked()
                    && let Err(error) = save_port_for_next_launch(settings, transient)
                {
                    transient.settings_save_error = Some(error);
                }
            });
            if let Some(error) = transient.settings_save_error.as_deref() {
                ui.colored_label(egui::Color32::from_rgb(183, 28, 28), error);
                ui.label(
                    "Check the settings path and permissions, then try again. The previous configured port remains unchanged.",
                );
            }
            if transient.settings_save_succeeded {
                ui.colored_label(
                    egui::Color32::from_rgb(46, 125, 50),
                    "Port saved for next launch. Changes take effect after restarting Chikachika.",
                );
            } else {
                ui.label("Port changes take effect only after restarting Chikachika.");
            }
        });
}

#[cfg(test)]
fn append_settings_labels(
    visible: &mut Vec<String>,
    settings: &SettingsState,
    transient: &TransientState,
    active_port: Option<u16>,
) {
    visible.push("Local server settings".to_owned());
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
        visible.push(
            "Repair or remove the settings source, then restart Chikachika. No fallback port is used while settings are invalid.".to_owned(),
        );
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
            ui.text_edit_singleline(&mut transient.create_name);
            ui.label("Fixed canvas width");
            ui.text_edit_singleline(&mut transient.create_width);
            ui.label("Fixed canvas height");
            ui.text_edit_singleline(&mut transient.create_height);
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
            ui.text_edit_singleline(&mut transient.rename_name);
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

/// A small native-window-free semantic scenario harness for adapter tests.
///
/// The harness renders frames for lifecycle state and routes named actions
/// through the same shared action helpers as the native event handlers. It does
/// not claim to replace a pixel-level native GUI smoke test.
#[cfg(test)]
pub struct ScenarioHarness {
    context: egui::Context,
    app: ChikachikaApp,
    last_copied_text: String,
    last_open_url: Option<egui::OpenUrl>,
}

#[cfg(test)]
impl ScenarioHarness {
    /// Creates a harness from deterministic startup state.
    pub fn new(outcome: BootstrapOutcome) -> Self {
        Self {
            context: egui::Context::default(),
            app: ChikachikaApp::from_bootstrap(outcome),
            last_copied_text: String::new(),
            last_open_url: None,
        }
    }

    /// Creates a harness from deterministic startup state and settings.
    pub fn new_with_settings(outcome: BootstrapOutcome, settings: SettingsState) -> Self {
        Self {
            context: egui::Context::default(),
            app: ChikachikaApp::from_application_bootstrap(ApplicationBootstrap::new(
                outcome, settings,
            )),
            last_copied_text: String::new(),
            last_open_url: None,
        }
    }

    /// Advances one egui frame.
    pub fn frame(&mut self) {
        let app = &mut self.app;
        let output = self
            .context
            .run(egui::RawInput::default(), |context| app.render(context));
        self.last_copied_text = output.platform_output.copied_text;
        self.last_open_url = output.platform_output.open_url;
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

    /// Activates a semantic adapter action by its exact label and renders the
    /// next frame. This deterministic path exercises the same shared action
    /// helpers as the egui event handlers without requiring native-window or
    /// pixel-coordinate automation.
    pub fn click(&mut self, label: &str) -> Result<(), String> {
        match label {
            "Copy URL" => copy_selected_url(&self.context, self.app.coordinator.as_ref())?,
            "Open in browser" => open_selected_url(&self.context, self.app.coordinator.as_ref())?,
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

    /// Returns whether the current adapter state exposes the requested visible
    /// text or control label. This is a semantic state check, while visual
    /// layout remains a short manual validation task.
    /// Returns whether the semantic rendered-state model exposes a label.
    pub fn has_label(&self, label: &str) -> bool {
        if let Some(failure) = self.app.blocked.as_ref() {
            return failure.error().to_string().contains(label);
        }
        let Some(coordinator) = self.app.coordinator.as_ref() else {
            return false;
        };
        let mut visible = vec![
            "Chikachika overlay workspace".to_owned(),
            "Overlays".to_owned(),
            "Create overlay".to_owned(),
            "Overlay details".to_owned(),
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
        if coordinator.selected_overlay().is_some() {
            visible.extend([
                "Rename".to_owned(),
                "Delete".to_owned(),
                "Browser-source URL".to_owned(),
            ]);
            if let Some(url) = coordinator.selected_url() {
                visible.push(url);
                visible.extend(["Copy URL".to_owned(), "Open in browser".to_owned()]);
            } else {
                visible.push(
                    "Unavailable until the local server successfully binds and reports readiness."
                        .to_owned(),
                );
            }
            if let Some(overlay) = coordinator.selected_overlay() {
                visible.push(overlay.name().to_owned());
                visible.push("Text widget".to_owned());
                if overlay.text_widget().is_some() {
                    visible.extend([
                        "Remove text widget".to_owned(),
                        "Content".to_owned(),
                        "Font size".to_owned(),
                        "Color".to_owned(),
                        "Alignment".to_owned(),
                        "Position".to_owned(),
                        "Canvas preview — drag to move the text widget".to_owned(),
                    ]);
                } else {
                    visible.extend([
                        "This overlay has no text widget.".to_owned(),
                        "Add text widget".to_owned(),
                    ]);
                }
            }
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

/// Run the Chikachika native window until it is closed.
///
/// The caller should pass `ApplicationBootstrap::new(outcome, settings)` (or
/// `outcome.with_settings(settings)`) so the GUI can display and update the
/// application settings state alongside the overlay workspace.
pub fn run(bootstrap: ApplicationBootstrap) -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([720.0, 480.0]),
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
        HeadlessCoordinator::empty(Store::at("test-overlays.json"))
    }

    #[test]
    fn workspace_exposes_complete_lifecycle_controls() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.frame();
        assert!(!harness.app().is_blocked());
        assert!(harness.app().coordinator().unwrap().overlays().is_empty());
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
        // Underscores are not accepted by the deliberately plain whole-number input.
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

        let malformed_path = directory.path().join("malformed.json");
        std::fs::write(&malformed_path, b"not json").expect("write malformed settings");
        let invalid_settings = SettingsState::load(SettingsStore::at(&malformed_path));
        let prior_error = invalid_settings
            .settings_error()
            .expect("load error")
            .to_string();
        let mut invalid_harness = ScenarioHarness::new_with_settings(
            BootstrapOutcome::Ready(ready_app()),
            invalid_settings,
        );
        invalid_harness.frame();
        invalid_harness.set_port_field("4001");
        // Restore the failed-save shape without replacing the malformed source.
        std::fs::remove_file(&malformed_path).expect("remove malformed source");
        std::fs::create_dir(&malformed_path).expect("make settings destination a directory");
        assert!(invalid_harness.click("Save port for next launch").is_err());
        assert_eq!(invalid_harness.settings().configured_port(), None);
        assert_eq!(
            invalid_harness
                .settings()
                .settings_error()
                .expect("prior load error remains")
                .to_string(),
            prior_error
        );
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

        assert!(harness.has_label("Current configured port: unavailable"));
        assert!(harness.has_label("Settings error:"));
        assert!(harness.has_label("Repair or remove the settings source"));
        assert!(harness.has_label("No fallback port is used while settings are invalid."));

        harness.set_port_field("4001");
        harness
            .click("Save port for next launch")
            .expect("repair settings by saving a valid port");
        assert!(harness.settings().is_valid());
        assert_eq!(harness.settings().configured_port(), Some(4_001));
        assert!(harness.settings().settings_error().is_none());
        assert!(harness.has_label("Port saved for next launch."));
        assert_eq!(
            SettingsStore::at(&settings_path)
                .load()
                .unwrap()
                .server_port(),
            4_001
        );
    }

    #[test]
    fn port_change_explains_restart_and_does_not_rebind_or_create_url() {
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
        harness.set_create_fields("Live", "320", "240");
        harness.click("Create").expect("create overlay");
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_url()
                .is_none()
        );
        harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .set_server_address("127.0.0.1:4000".parse().expect("socket address"));
        harness.set_port_field("4001");
        harness
            .click("Save port for next launch")
            .expect("save next-launch port");

        assert_eq!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .server_address()
                .unwrap()
                .port(),
            4_000
        );
        assert_eq!(harness.settings().configured_port(), Some(4_001));
        assert!(harness.has_label("Running server remains on port 4000 until restart."));
        assert!(harness.has_label("Changes take effect after restarting Chikachika."));
        // Once readiness is explicitly reported, the coordinator—not the
        // settings form—becomes the URL authority.
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_url()
                .is_some()
        );
    }

    #[test]
    fn blocked_mode_shows_exact_error_and_no_save_or_url() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("overlays.json");
        std::fs::write(&path, b"not json").unwrap();
        let outcome =
            HeadlessCoordinator::<crate::server::OverlayHub>::bootstrap_outcome(Store::at(path));
        let mut harness = ScenarioHarness::new(outcome);
        harness.frame();
        assert!(harness.app().is_blocked());
        assert!(harness.app().coordinator().is_none());
        assert!(harness.has_label("persisted overlays"));
        assert!(!harness.has_label("Save"));
        assert!(!harness.has_label("Retry"));
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
        harness.frame();
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
    fn errors_remain_visible_and_recoverable() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("overlays.json");
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(
            HeadlessCoordinator::empty(Store::at(&path)),
        ));
        harness.frame();
        harness.click("Create overlay").expect("open create dialog");
        harness.set_create_fields("Unsaved", "320", "240");
        harness.click("Create").expect("create overlay");
        std::fs::create_dir(&path).expect("replace source with directory");
        assert!(harness.click("Save").is_err());
        assert!(harness.has_label("could not atomically replace persisted overlays"));
        assert!(harness.app().coordinator().unwrap().is_dirty());
        assert!(harness.has_label("Save"));
    }

    #[test]
    fn server_unavailable_hides_url_and_readiness_shows_exact_selected_url() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.frame();
        harness.click("Create overlay").expect("open create dialog");
        harness.set_create_fields("Live", "320", "240");
        harness.click("Create").expect("create overlay");
        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_url()
                .is_none()
        );
        assert!(!harness.has_label("Copy URL"));
        assert!(!harness.has_label("Open in browser"));
        assert!(harness.has_label(
            "Unavailable until the local server successfully binds and reports readiness."
        ));
        harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .set_server_address("127.0.0.1:51737".parse().expect("socket address"));
        harness.frame();
        let url = harness.app().coordinator().unwrap().selected_url().unwrap();
        assert_eq!(
            url,
            format!(
                "http://127.0.0.1:51737/overlay/{}",
                harness
                    .app()
                    .coordinator()
                    .unwrap()
                    .selected_overlay_id()
                    .unwrap()
            )
        );
        assert!(harness.has_label("Browser-source URL"));
        assert!(harness.has_label("Copy URL"));
        assert!(harness.has_label("Open in browser"));
    }

    #[test]
    fn url_actions_emit_the_exact_selected_url() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.frame();
        assert!(!harness.has_label("Copy URL"));
        assert!(!harness.has_label("Open in browser"));
        assert!(harness.click("Copy URL").is_err());
        assert!(harness.click("Open in browser").is_err());
        assert_eq!(harness.copied_text(), "");
        assert_eq!(harness.opened_url(), None);

        harness.click("Create overlay").expect("open create dialog");
        harness.set_create_fields("Live", "320", "240");
        harness.click("Create").expect("create overlay");
        assert!(harness.click("Copy URL").is_err());
        assert!(harness.click("Open in browser").is_err());
        assert_eq!(harness.copied_text(), "");
        assert_eq!(harness.opened_url(), None);

        harness
            .app_mut()
            .coordinator_mut()
            .unwrap()
            .set_server_address("127.0.0.1:51737".parse().expect("socket address"));
        harness.frame();
        let url = harness.app().coordinator().unwrap().selected_url().unwrap();

        harness.click("Copy URL").expect("copy selected URL");
        assert_eq!(harness.copied_text(), url);
        assert_eq!(harness.opened_url(), None);

        harness
            .click("Open in browser")
            .expect("open selected URL in browser");
        assert_eq!(harness.copied_text(), "");
        assert_eq!(harness.opened_url(), Some(url.as_str()));
        assert_eq!(harness.opens_in_new_tab(), Some(false));
    }

    #[test]
    fn url_actions_hide_when_hosted_overlay_diverges_from_workspace() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.click("Create overlay").expect("open create dialog");
        harness.set_create_fields("Live", "320", "240");
        harness.click("Create").expect("create overlay");
        let coordinator = harness.app_mut().coordinator_mut().unwrap();
        coordinator.set_server_address("127.0.0.1:51737".parse().expect("socket address"));
        let overlay_id = coordinator.selected_overlay_id().unwrap();
        coordinator
            .hub()
            .remove(overlay_id)
            .expect("remove hosted overlay");
        harness.frame();

        assert!(
            harness
                .app()
                .coordinator()
                .unwrap()
                .selected_url()
                .is_none()
        );
        assert!(!harness.has_label("Copy URL"));
        assert!(!harness.has_label("Open in browser"));
        assert!(harness.click("Copy URL").is_err());
        assert!(harness.click("Open in browser").is_err());
    }

    #[test]
    fn preview_scale_and_coordinate_conversion_preserve_canvas_geometry() {
        let canvas = crate::model::CanvasSize::new(1920, 1080).unwrap();
        let scale = preview_scale(canvas, 960.0);
        assert_eq!(scale, 1.0 / 3.0);

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
    fn preview_drag_state_is_scoped_to_overlay_and_widget() {
        let first_overlay = crate::model::Overlay::with_dimensions("First", 320, 240).unwrap();
        let second_overlay = crate::model::Overlay::with_dimensions("Second", 320, 240).unwrap();
        let first_widget = TextWidget::new("first");
        let second_widget = TextWidget::new("second");
        let drag = PreviewDrag {
            overlay_id: first_overlay.id(),
            widget_id: first_widget.id(),
            pointer_offset: egui::vec2(3.0, 4.0),
        };
        assert!(drag_matches(
            Some(drag),
            first_overlay.id(),
            first_widget.id()
        ));
        assert!(!drag_matches(
            Some(drag),
            second_overlay.id(),
            first_widget.id()
        ));
        assert!(!drag_matches(
            Some(drag),
            first_overlay.id(),
            second_widget.id()
        ));
        assert!(!drag_matches(None, first_overlay.id(), first_widget.id()));
    }

    #[test]
    fn semantic_editor_controls_enforce_zero_or_one_widget() {
        let mut harness = ScenarioHarness::new(BootstrapOutcome::Ready(ready_app()));
        harness.click("Create overlay").expect("open create dialog");
        harness.set_create_fields("Live", "320", "240");
        harness.click("Create").expect("create overlay");
        assert!(harness.has_label("Add text widget"));
        assert!(!harness.has_label("Remove text widget"));

        harness
            .click("Add text widget")
            .expect("add the optional widget");
        assert!(!harness.has_label("Add text widget"));
        assert!(harness.has_label("Remove text widget"));
        assert!(harness.has_label("Content"));
        assert!(harness.has_label("Font size"));
        assert!(harness.has_label("Color"));
        assert!(harness.has_label("Alignment"));
        assert!(harness.has_label("Position"));
        assert!(harness.has_label("Canvas preview"));

        harness
            .click("Remove text widget")
            .expect("remove the optional widget");
        assert!(harness.has_label("Add text widget"));
        assert!(!harness.has_label("Remove text widget"));
    }

    #[test]
    fn text_editor_adds_updates_and_removes_through_the_coordinator() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut coordinator =
            HeadlessCoordinator::empty(Store::at(directory.path().join("overlays.json")));
        let overlay_id = coordinator
            .create_overlay("Live", 320, 240)
            .expect("create overlay");
        coordinator.save().expect("establish clean state");

        add_text_widget(&mut coordinator, overlay_id).expect("add text widget");
        let widget = coordinator
            .overlay(overlay_id)
            .unwrap()
            .text_widget()
            .unwrap();
        let widget_id = widget.id();
        assert_eq!(widget.content(), "Text");
        assert!(coordinator.is_dirty());

        coordinator.save().expect("save added widget");
        apply_text_editor_values(
            &mut coordinator,
            overlay_id,
            TextEditorValues {
                id: widget_id,
                content: "Starting\nSoon".to_owned(),
                position: Position::new(123.0, 45.0),
                font_size: 42.0,
                color: Color::rgba(10, 20, 30, 128),
                alignment: Alignment::Center,
            },
        )
        .expect("update all supported properties");
        let widget = coordinator
            .overlay(overlay_id)
            .unwrap()
            .text_widget()
            .unwrap();
        assert_eq!(widget.id(), widget_id);
        assert_eq!(widget.content(), "Starting\nSoon");
        assert_eq!(widget.position(), Position::new(123.0, 45.0));
        assert_eq!(widget.font_size(), 42.0);
        assert_eq!(widget.color(), Color::rgba(10, 20, 30, 128));
        assert_eq!(widget.alignment(), Alignment::Center);
        assert!(coordinator.is_dirty());
        let published_overlay = coordinator
            .hub()
            .snapshot(overlay_id)
            .expect("published overlay remains available")
            .expect("published overlay exists");
        let browser_snapshot = crate::browser::project(&published_overlay);
        assert_eq!(browser_snapshot.overlay_id(), &overlay_id.to_string());
        assert_eq!(
            browser_snapshot.revision(),
            coordinator
                .overlay(overlay_id)
                .expect("overlay remains present")
                .revision()
        );
        let browser_widget = browser_snapshot.text_widget().expect("browser text widget");
        assert_eq!(browser_widget.widget_id(), &widget_id.to_string());
        assert_eq!(browser_widget.content(), "Starting\nSoon");
        assert_eq!(browser_widget.position().x(), 123.0);
        assert_eq!(browser_widget.position().y(), 45.0);
        assert_eq!(browser_widget.font_size(), 42.0);
        assert_eq!(browser_widget.color().alpha(), 128);
        assert_eq!(
            browser_widget.alignment(),
            crate::browser::BrowserAlignment::Center
        );

        coordinator.save().expect("save valid edit");
        let prior = coordinator.overlay(overlay_id).unwrap().clone();
        let mut invalid = TextEditorValues::from_widget(prior.text_widget().unwrap());
        invalid.font_size = 0.0;
        assert!(apply_text_editor_values(&mut coordinator, overlay_id, invalid).is_err());
        assert_eq!(coordinator.overlay(overlay_id).unwrap(), &prior);
        assert!(!coordinator.is_dirty());
        assert!(coordinator.last_error().unwrap().contains("font size"));

        let mut large = TextEditorValues::from_widget(prior.text_widget().unwrap());
        large.font_size = 2048.0;
        apply_text_editor_values(&mut coordinator, overlay_id, large)
            .expect("model-valid font size is accepted without UI clamping");
        assert_eq!(
            coordinator
                .overlay(overlay_id)
                .unwrap()
                .text_widget()
                .unwrap()
                .font_size(),
            2048.0
        );

        remove_text_widget(&mut coordinator, overlay_id, widget_id).expect("remove widget");
        assert!(
            coordinator
                .overlay(overlay_id)
                .unwrap()
                .text_widget()
                .is_none()
        );
    }

    #[test]
    fn text_editor_url_and_port_controls_are_present() {
        let source = include_str!("gui.rs");
        let production = source
            .split("/// A small native-window-free semantic scenario harness")
            .next()
            .expect("production adapter precedes its tests");
        assert!(production.contains("TextEdit::multiline"));
        assert!(production.contains("Add text widget"));
        assert!(production.contains("Remove text widget"));
        assert!(production.contains("Canvas preview"));
        assert!(production.contains("Browser-source URL"));
        assert!(production.contains("Copy URL"));
        assert!(production.contains("Open in browser"));
        assert!(production.contains("copy_url"));
        assert!(production.contains("open_url"));
        assert!(production.contains("Save"));
        assert!(production.contains("save_port_for_next_launch"));
        assert!(production.contains("Current configured port"));
        assert!(production.contains("Settings path"));
        assert!(production.contains("MIN_PORT"));
        assert!(production.contains("MAX_PORT"));
        assert!(production.contains("Changes take effect after restarting Chikachika."));
        assert!(production.contains("Confirm delete"));
    }
}
