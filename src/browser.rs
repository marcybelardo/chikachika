//! Browser-source projection of the authoritative ordered overlay model.
//!
//! Projection receives delivery revision metadata explicitly. Revisions are not
//! read from or stored in the durable model.

use serde::Serialize;

use crate::model::{Alignment, Color, Overlay, Position, TextWidget};

const INDEX_HTML: &str = include_str!("../assets/browser/index.html");
const STYLES: &str = include_str!("../assets/browser/style.css");
const SCRIPT: &str = include_str!("../assets/browser/overlay.js");

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BrowserRepresentation {
    overlay_id: String,
    revision: u64,
    canvas: BrowserCanvas,
    widgets: Vec<BrowserTextWidget>,
}
impl BrowserRepresentation {
    pub const fn canvas(&self) -> BrowserCanvas {
        self.canvas
    }
    pub fn overlay_id(&self) -> &str {
        &self.overlay_id
    }
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    pub fn widgets(&self) -> &[BrowserTextWidget] {
        &self.widgets
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BrowserCanvas {
    width: u32,
    height: u32,
}
impl BrowserCanvas {
    pub const fn width(self) -> u32 {
        self.width
    }
    pub const fn height(self) -> u32 {
        self.height
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BrowserTextWidget {
    widget_id: String,
    name: String,
    content: String,
    font_family: String,
    position: BrowserPosition,
    font_size: f32,
    color: BrowserColor,
    alignment: BrowserAlignment,
}
impl BrowserTextWidget {
    pub fn widget_id(&self) -> &str {
        &self.widget_id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn content(&self) -> &str {
        &self.content
    }
    pub fn font_family(&self) -> &str {
        &self.font_family
    }
    pub const fn position(&self) -> BrowserPosition {
        self.position
    }
    pub const fn font_size(&self) -> f32 {
        self.font_size
    }
    pub const fn color(&self) -> BrowserColor {
        self.color
    }
    pub const fn alignment(&self) -> BrowserAlignment {
        self.alignment
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BrowserPosition {
    x: f32,
    y: f32,
}
impl BrowserPosition {
    pub const fn x(self) -> f32 {
        self.x
    }
    pub const fn y(self) -> f32 {
        self.y
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BrowserColor {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}
impl BrowserColor {
    pub const fn red(self) -> u8 {
        self.red
    }
    pub const fn green(self) -> u8 {
        self.green
    }
    pub const fn blue(self) -> u8 {
        self.blue
    }
    pub const fn alpha(self) -> u8 {
        self.alpha
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BrowserAlignment {
    Left,
    Center,
    Right,
}

/// Projects the durable model with an explicit running-session delivery revision.
pub fn project(overlay: &Overlay, revision: u64) -> BrowserRepresentation {
    BrowserRepresentation {
        canvas: BrowserCanvas {
            width: overlay.canvas().width(),
            height: overlay.canvas().height(),
        },
        overlay_id: overlay.id().to_string(),
        revision,
        widgets: overlay.widgets().iter().map(project_text_widget).collect(),
    }
}

/// Renders initial HTML for every widget. Model index zero is frontmost, so
/// reverse traversal makes frontmost content paint last.
pub fn render(overlay: &Overlay, revision: u64) -> String {
    let representation = project(overlay, revision);
    let widgets = representation
        .widgets()
        .iter()
        .rev()
        .map(render_text_widget)
        .collect::<String>();
    INDEX_HTML
        .replace("{{CHIKACHIKA_STYLES}}", STYLES)
        .replace("{{CHIKACHIKA_SCRIPT}}", SCRIPT)
        .replace(
            "{{CHIKACHIKA_CANVAS_WIDTH}}",
            &representation.canvas.width().to_string(),
        )
        .replace(
            "{{CHIKACHIKA_CANVAS_HEIGHT}}",
            &representation.canvas.height().to_string(),
        )
        .replace("{{CHIKACHIKA_TEXT_WIDGET}}", &widgets)
}

pub const fn embedded_index_html() -> &'static str {
    INDEX_HTML
}
pub const fn embedded_styles() -> &'static str {
    STYLES
}
pub const fn embedded_script() -> &'static str {
    SCRIPT
}

fn project_text_widget(widget: &TextWidget) -> BrowserTextWidget {
    BrowserTextWidget {
        widget_id: widget.id().to_string(),
        name: widget.name().to_owned(),
        content: widget.content().to_owned(),
        font_family: widget.font_family().id().to_owned(),
        position: project_position(widget.position()),
        font_size: widget.font_size(),
        color: project_color(widget.color()),
        alignment: project_alignment(widget.alignment()),
    }
}
fn project_position(position: Position) -> BrowserPosition {
    BrowserPosition {
        x: position.x(),
        y: position.y(),
    }
}
fn project_color(color: Color) -> BrowserColor {
    BrowserColor {
        red: color.red(),
        green: color.green(),
        blue: color.blue(),
        alpha: color.alpha(),
    }
}
fn project_alignment(alignment: Alignment) -> BrowserAlignment {
    match alignment {
        Alignment::Left => BrowserAlignment::Left,
        Alignment::Center => BrowserAlignment::Center,
        Alignment::Right => BrowserAlignment::Right,
    }
}
fn font_css(font: &str) -> &'static str {
    match font {
        "jetbrains-mono" => "'JetBrains Mono', monospace",
        _ => "'Noto Sans', sans-serif",
    }
}
fn alignment_css(alignment: BrowserAlignment) -> &'static str {
    match alignment {
        BrowserAlignment::Left => "left",
        BrowserAlignment::Center => "center",
        BrowserAlignment::Right => "right",
    }
}
fn format_css_number(value: f32) -> String {
    value.to_string()
}
fn render_text_widget(widget: &BrowserTextWidget) -> String {
    format!(
        "<span class=\"chikachika-text\" data-widget-id=\"{}\" data-widget-name=\"{}\" style=\"left: {}px; top: {}px; font-size: {}px; font-family: {}; color: rgba({}, {}, {}, {}); text-align: {};\">{}</span>",
        escape_html(&widget.widget_id),
        escape_html(&widget.name),
        format_css_number(widget.position.x()),
        format_css_number(widget.position.y()),
        format_css_number(widget.font_size()),
        font_css(widget.font_family()),
        widget.color.red(),
        widget.color.green(),
        widget.color.blue(),
        widget.color.alpha() as f32 / 255.0,
        alignment_css(widget.alignment()),
        escape_html(&widget.content),
    )
}
fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CanvasSize, Color, FontFamily, Overlay, Position, TextWidget};
    fn empty_overlay() -> Overlay {
        Overlay::new("Starting Soon", CanvasSize::new(1280, 720).unwrap()).unwrap()
    }

    #[test]
    fn browser_multi_widget_projection_and_html() {
        let mut overlay = empty_overlay();
        let back = overlay
            .add_widget(
                TextWidget::with_all_properties(
                    "Back",
                    "safe <back>",
                    FontFamily::NotoSans,
                    Position::new(12.5, 34.25),
                    42.5,
                    Color::rgba(10, 20, 30, 128),
                    Alignment::Center,
                )
                .unwrap(),
            )
            .unwrap();
        let front = overlay
            .add_widget(
                TextWidget::with_all_properties(
                    "Front",
                    "front & safe",
                    FontFamily::JetBrainsMono,
                    Position::origin(),
                    16.0,
                    Color::white(),
                    Alignment::Right,
                )
                .unwrap(),
            )
            .unwrap();
        let representation = project(&overlay, 37);
        assert_eq!(representation.revision(), 37);
        assert_eq!(
            representation
                .widgets()
                .iter()
                .map(|w| w.widget_id())
                .collect::<Vec<_>>(),
            vec![front.to_string(), back.to_string()]
        );
        assert_eq!(representation.widgets()[0].font_family(), "jetbrains-mono");
        let json = serde_json::to_string(&representation).unwrap();
        assert!(json.contains("\"widgets\""));
        assert!(!json.contains("text_widget"));
        let html = render(&overlay, 37);
        let front_position = html.find(&format!("data-widget-id=\"{front}\"")).unwrap();
        let back_position = html.find(&format!("data-widget-id=\"{back}\"")).unwrap();
        assert!(back_position < front_position);
        assert!(html.contains("safe &lt;back&gt;"));
        assert!(html.contains("font-family: 'JetBrains Mono', monospace;"));
        assert!(html.contains("background: transparent !important;"));
    }
    #[test]
    fn empty_projection_has_ordered_array_and_explicit_revision() {
        let overlay = empty_overlay();
        let representation = project(&overlay, 9);
        assert_eq!(representation.widgets(), &[]);
        assert_eq!(
            serde_json::to_value(&representation).unwrap()["revision"],
            9
        );
    }
    #[test]
    fn browser_projection_preserves_all_widget_fields() {
        let mut overlay = empty_overlay();
        let id = overlay
            .add_widget(
                TextWidget::with_all_properties(
                    "Name",
                    "hello",
                    FontFamily::JetBrainsMono,
                    Position::new(1.0, 2.0),
                    3.0,
                    Color::rgba(4, 5, 6, 7),
                    Alignment::Center,
                )
                .unwrap(),
            )
            .unwrap();
        let representation = project(&overlay, 1);
        let widget = &representation.widgets()[0];
        assert_eq!(widget.widget_id(), id.to_string());
        assert_eq!(widget.name(), "Name");
        assert_eq!(widget.content(), "hello");
        assert_eq!(widget.position(), BrowserPosition { x: 1.0, y: 2.0 });
        assert_eq!(widget.font_size(), 3.0);
        assert_eq!(
            widget.color(),
            BrowserColor {
                red: 4,
                green: 5,
                blue: 6,
                alpha: 7
            }
        );
        assert_eq!(widget.alignment(), BrowserAlignment::Center);
    }
    #[test]
    fn embedded_assets_are_available_without_filesystem_lookup() {
        assert!(embedded_index_html().contains("{{CHIKACHIKA_TEXT_WIDGET}}"));
        assert!(embedded_styles().contains("background: transparent"));
        assert!(embedded_script().contains("ChikachikaOverlay"));
    }
}
