//! Browser-source representation and compile-time embedded assets.
//!
//! This module is a projection of the authoritative [`crate::model::Overlay`]
//! document. It deliberately contains no transport or mutation protocol, so a
//! future server adapter can host the same representation without introducing
//! browser-owned state.

use crate::model::{Alignment, Color, Overlay, Position, TextWidget};

const INDEX_HTML: &str = include_str!("../assets/browser/index.html");
const STYLES: &str = include_str!("../assets/browser/style.css");
const SCRIPT: &str = include_str!("../assets/browser/overlay.js");

/// A complete browser projection of an [`Overlay`].
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserRepresentation {
    canvas: BrowserCanvas,
    overlay_id: String,
    revision: u64,
    text_widget: Option<BrowserTextWidget>,
}

impl BrowserRepresentation {
    /// Returns the fixed canvas dimensions in CSS pixels.
    pub const fn canvas(&self) -> BrowserCanvas {
        self.canvas
    }

    /// Returns the stable overlay identity used by hosting adapters.
    pub fn overlay_id(&self) -> &str {
        &self.overlay_id
    }

    /// Returns the model revision represented by this snapshot.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the optional projected text widget.
    pub fn text_widget(&self) -> Option<&BrowserTextWidget> {
        self.text_widget.as_ref()
    }
}

/// Fixed dimensions for the browser canvas.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrowserCanvas {
    width: u32,
    height: u32,
}

impl BrowserCanvas {
    /// Returns the canvas width in CSS pixels.
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Returns the canvas height in CSS pixels.
    pub const fn height(self) -> u32 {
        self.height
    }
}

/// A complete browser projection of the model's supported text widget.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTextWidget {
    widget_id: String,
    content: String,
    position: BrowserPosition,
    font_size: f32,
    color: BrowserColor,
    alignment: BrowserAlignment,
}

impl BrowserTextWidget {
    /// Returns the stable text-widget identity.
    pub fn widget_id(&self) -> &str {
        &self.widget_id
    }

    /// Returns the text content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the position in canvas coordinates.
    pub const fn position(&self) -> BrowserPosition {
        self.position
    }

    /// Returns the font size in CSS pixels.
    pub const fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Returns the RGBA color.
    pub const fn color(&self) -> BrowserColor {
        self.color
    }

    /// Returns the horizontal alignment.
    pub const fn alignment(&self) -> BrowserAlignment {
        self.alignment
    }
}

/// A text-widget position in canvas coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrowserPosition {
    x: f32,
    y: f32,
}

impl BrowserPosition {
    /// Returns the horizontal coordinate.
    pub const fn x(self) -> f32 {
        self.x
    }

    /// Returns the vertical coordinate.
    pub const fn y(self) -> f32 {
        self.y
    }
}

/// An RGBA browser color.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrowserColor {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl BrowserColor {
    /// Returns the red channel.
    pub const fn red(self) -> u8 {
        self.red
    }

    /// Returns the green channel.
    pub const fn green(self) -> u8 {
        self.green
    }

    /// Returns the blue channel.
    pub const fn blue(self) -> u8 {
        self.blue
    }

    /// Returns the alpha channel.
    pub const fn alpha(self) -> u8 {
        self.alpha
    }
}

/// Horizontal alignment supported by the browser projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserAlignment {
    Left,
    Center,
    Right,
}

/// Projects the complete authoritative model state into a browser representation.
pub fn project(overlay: &Overlay) -> BrowserRepresentation {
    BrowserRepresentation {
        canvas: BrowserCanvas {
            width: overlay.canvas().width(),
            height: overlay.canvas().height(),
        },
        overlay_id: overlay.id().to_string(),
        revision: overlay.revision(),
        text_widget: overlay.text_widget().map(project_text_widget),
    }
}

/// Renders the complete authoritative model state as a transparent HTML document.
///
/// All HTML, CSS, and JavaScript used by this function are compiled into the
/// executable. The returned document is independent of the process working
/// directory and is ready for a hosting adapter to serve as the browser source.
pub fn render(overlay: &Overlay) -> String {
    let representation = project(overlay);
    let text_widget = representation
        .text_widget()
        .map(render_text_widget)
        .unwrap_or_default();

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
        .replace("{{CHIKACHIKA_TEXT_WIDGET}}", &text_widget)
}

/// Returns the embedded browser document template for hosting adapters.
pub const fn embedded_index_html() -> &'static str {
    INDEX_HTML
}

/// Returns the embedded browser stylesheet for hosting adapters.
pub const fn embedded_styles() -> &'static str {
    STYLES
}

/// Returns the embedded browser script for hosting adapters.
pub const fn embedded_script() -> &'static str {
    SCRIPT
}

fn project_text_widget(widget: &TextWidget) -> BrowserTextWidget {
    BrowserTextWidget {
        widget_id: widget.id().to_string(),
        content: widget.content().to_owned(),
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

fn render_text_widget(widget: &BrowserTextWidget) -> String {
    format!(
        "<span class=\"chikachika-text\" data-widget-id=\"{}\" style=\"left: {}px; top: {}px; font-size: {}px; color: rgba({}, {}, {}, {}); text-align: {};\">{}</span>",
        escape_html(&widget.widget_id),
        format_css_number(widget.position.x()),
        format_css_number(widget.position.y()),
        format_css_number(widget.font_size()),
        widget.color.red(),
        widget.color.green(),
        widget.color.blue(),
        widget.color.alpha() as f32 / 255.0,
        alignment_css(widget.alignment()),
        escape_html(&widget.content),
    )
}

fn format_css_number(value: f32) -> String {
    value.to_string()
}

fn alignment_css(alignment: BrowserAlignment) -> &'static str {
    match alignment {
        BrowserAlignment::Left => "left",
        BrowserAlignment::Center => "center",
        BrowserAlignment::Right => "right",
    }
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

impl BrowserCanvas {
    #[cfg(test)]
    const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl BrowserPosition {
    #[cfg(test)]
    const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl BrowserColor {
    #[cfg(test)]
    const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CanvasSize, Color, Overlay, Position, TextWidget};

    fn empty_overlay() -> Overlay {
        let canvas = CanvasSize::new(1280, 720).expect("valid canvas");
        Overlay::new("Starting Soon", canvas).expect("valid overlay")
    }

    #[test]
    fn empty_overlay_projects_transparent_fixed_canvas() {
        let overlay = empty_overlay();
        let representation = project(&overlay);

        assert_eq!(representation.canvas(), BrowserCanvas::new(1280, 720));
        assert_eq!(representation.overlay_id(), overlay.id().to_string());
        assert_eq!(representation.revision(), 0);
        assert!(representation.text_widget().is_none());

        let html = render(&overlay);
        assert!(html.contains("background: transparent !important;"));
        assert!(html.contains("data-width=\"1280\""));
        assert!(html.contains("data-height=\"720\""));
        assert!(html.contains("style=\"width: 1280px; height: 720px;\""));
        assert!(!html.contains("<span class=\"chikachika-text\""));
    }

    #[test]
    fn populated_overlay_projects_all_supported_text_values() {
        let mut overlay = empty_overlay();
        let widget = TextWidget::with_properties(
            "Hello <stream> & \"friends\"",
            Position::new(12.5, 34.25),
            42.5,
            Color::rgba(10, 20, 30, 128),
            Alignment::Center,
        )
        .expect("valid widget");
        let widget_id = widget.id();
        overlay.add_text_widget(widget).expect("add widget");

        let representation = project(&overlay);
        let projected = representation.text_widget().expect("text widget");
        assert_eq!(projected.widget_id(), widget_id.to_string());
        assert_eq!(projected.content(), "Hello <stream> & \"friends\"");
        assert_eq!(projected.position(), BrowserPosition::new(12.5, 34.25));
        assert_eq!(projected.font_size(), 42.5);
        assert_eq!(projected.color(), BrowserColor::new(10, 20, 30, 128));
        assert_eq!(projected.alignment(), BrowserAlignment::Center);

        let html = render(&overlay);
        assert!(html.contains("data-widget-id=\""));
        assert!(html.contains("left: 12.5px; top: 34.25px;"));
        assert!(html.contains("right: 0;"));
        assert!(html.contains("font-size: 42.5px;"));
        assert!(html.contains("color: rgba(10, 20, 30, 0.5019608);"));
        assert!(html.contains("text-align: center;"));
        assert!(html.contains("Hello &lt;stream&gt; &amp; &quot;friends&quot;"));
    }

    #[test]
    fn all_supported_alignments_project_to_matching_css_values() {
        for (model_alignment, browser_alignment, css) in [
            (Alignment::Left, BrowserAlignment::Left, "left"),
            (Alignment::Center, BrowserAlignment::Center, "center"),
            (Alignment::Right, BrowserAlignment::Right, "right"),
        ] {
            let mut overlay = empty_overlay();
            let widget = TextWidget::with_properties(
                "aligned",
                Position::origin(),
                16.0,
                Color::white(),
                model_alignment,
            )
            .expect("valid widget");
            overlay.add_text_widget(widget).expect("add widget");

            let representation = project(&overlay);
            assert_eq!(
                representation
                    .text_widget()
                    .expect("text widget")
                    .alignment(),
                browser_alignment
            );
            assert!(render(&overlay).contains(&format!("text-align: {css};")));
        }
    }

    #[test]
    fn embedded_assets_are_available_without_filesystem_lookup() {
        assert!(embedded_index_html().contains("{{CHIKACHIKA_STYLES}}"));
        assert!(embedded_styles().contains("background: transparent"));
        assert!(embedded_script().contains("ChikachikaOverlay"));
    }
}
