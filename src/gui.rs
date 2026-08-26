//! Minimal native GUI for Chikachika.

use eframe::egui;

/// The initial native application view.
pub struct ChikachikaApp;

impl Default for ChikachikaApp {
    fn default() -> Self {
        Self
    }
}

impl eframe::App for ChikachikaApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(context, |ui| {
            ui.heading("Welcome to Chikachika");
            ui.label("Your local overlay workspace is ready.");
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.colored_label(egui::Color32::from_rgb(46, 125, 50), "Ready");
            });
        });
    }
}

/// Run the Chikachika native window until it is closed.
pub fn run() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 240.0])
            .with_min_inner_size([320.0, 180.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Chikachika",
        native_options,
        Box::new(|_creation_context| Ok(Box::new(ChikachikaApp))),
    )
}
