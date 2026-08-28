//! Native egui adapter for the issue #4 overlay workspace.
//!
//! The adapter owns only transient form and confirmation state. The overlay
//! collection, selection, dirty state, persistence status, and URL readiness
//! all remain in [`crate::app::HeadlessCoordinator`].

use eframe::egui;

use crate::app::{BootstrapOutcome, HeadlessCoordinator};
use crate::model::{Alignment, Color, OverlayId, Position, TextWidget, TextWidgetId};

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
    transient: TransientState,
}

impl ChikachikaApp {
    /// Creates the adapter from the startup result.
    pub fn from_bootstrap(outcome: BootstrapOutcome) -> Self {
        match outcome {
            BootstrapOutcome::Ready(coordinator) => Self {
                coordinator: Some(coordinator),
                blocked: None,
                transient: TransientState::default(),
            },
            BootstrapOutcome::Blocked(failure) => Self {
                coordinator: None,
                blocked: Some(failure),
                transient: TransientState::default(),
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
                    ui.monospace(url);
                } else {
                    ui.label(
                    "Unavailable until the local server successfully binds and reports readiness.",
                );
                }
                ui.label("URL actions and configurable-port controls are deferred to issue #8.");
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

fn save_workspace(coordinator: Option<&mut HeadlessCoordinator>) -> Result<(), String> {
    coordinator
        .ok_or_else(|| "workspace is not available".to_owned())?
        .save()
        .map_err(|error| error.to_string())
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
}

#[cfg(test)]
impl ScenarioHarness {
    /// Creates a harness from deterministic startup state.
    pub fn new(outcome: BootstrapOutcome) -> Self {
        Self {
            context: egui::Context::default(),
            app: ChikachikaApp::from_bootstrap(outcome),
        }
    }

    /// Advances one egui frame.
    pub fn frame(&mut self) {
        let app = &mut self.app;
        let _ = self
            .context
            .run(egui::RawInput::default(), |context| app.render(context));
    }

    /// Returns the current adapter state for semantic assertions.
    pub fn app(&self) -> &ChikachikaApp {
        &self.app
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
        self.app.activate(label)?;
        self.frame();
        Ok(())
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
            "URL actions and configurable-port controls are deferred to issue #8.".to_owned(),
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
        visible
            .into_iter()
            .any(|text| text == label || text.contains(label))
    }
}

/// Run the Chikachika native window until it is closed.
pub fn run(outcome: BootstrapOutcome) -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Chikachika",
        native_options,
        Box::new(move |_creation_context| Ok(Box::new(ChikachikaApp::from_bootstrap(outcome)))),
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
    fn text_editor_controls_are_present_while_issue_8_controls_remain_absent() {
        let source = include_str!("gui.rs");
        let production = source
            .split("/// A small native-window-free semantic scenario harness")
            .next()
            .expect("production adapter precedes its tests");
        assert!(!production.contains("copy_url_button"));
        assert!(!production.contains("open_url_button"));
        assert!(!production.contains("port_control"));
        assert!(production.contains("TextEdit::multiline"));
        assert!(production.contains("Add text widget"));
        assert!(production.contains("Remove text widget"));
        assert!(production.contains("Canvas preview"));
        assert!(production.contains("Browser-source URL"));
        assert!(production.contains("Save"));
        assert!(production.contains("Confirm delete"));
    }
}
