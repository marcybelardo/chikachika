//! Framework-independent domain model for a Chikachika overlay.
//!
//! This module intentionally contains no editor, server, persistence, or browser
//! concerns. Adapters should read this model and perform mutations through
//! [`Overlay`]'s operations.

use std::error::Error;
use std::fmt;

use uuid::Uuid;

/// The identity of an overlay.
///
/// The UUID is generated once when an [`Overlay`] is created. Names and other
/// presentation values are deliberately not used as identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OverlayId(Uuid);

impl OverlayId {
    /// Returns the generated UUID without exposing a way to replace it.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }

    pub(crate) const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl fmt::Display for OverlayId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The identity of an optional text widget.
///
/// The UUID is generated once when the widget is created and remains unchanged
/// while its supported properties are edited.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextWidgetId(Uuid);

impl TextWidgetId {
    /// Returns the generated UUID without exposing a way to replace it.
    pub fn as_uuid(self) -> Uuid {
        self.0
    }

    pub(crate) const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl fmt::Display for TextWidgetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The fixed dimensions of an overlay canvas.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanvasSize {
    width: u32,
    height: u32,
}

impl CanvasSize {
    /// Creates a canvas size. Both dimensions must be greater than zero.
    pub fn new(width: u32, height: u32) -> Result<Self, ModelError> {
        if width == 0 || height == 0 {
            return Err(ModelError::InvalidCanvasSize { width, height });
        }

        Ok(Self { width, height })
    }

    /// Returns the canvas width in logical units.
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Returns the canvas height in logical units.
    pub const fn height(self) -> u32 {
        self.height
    }
}

/// A text widget position in canvas coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    x: f32,
    y: f32,
}

impl Position {
    /// Creates a position. Canvas-bound and finite validation is performed by
    /// the overlay operation that applies the position.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// The top-left canvas position.
    pub const fn origin() -> Self {
        Self::new(0.0, 0.0)
    }

    /// Returns the horizontal coordinate.
    pub const fn x(self) -> f32 {
        self.x
    }

    /// Returns the vertical coordinate.
    pub const fn y(self) -> f32 {
        self.y
    }

    fn is_finite_and_nonnegative(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.x >= 0.0 && self.y >= 0.0
    }
}

/// An RGBA text color.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl Color {
    /// Creates an opaque RGB color.
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, u8::MAX)
    }

    /// Creates an RGBA color.
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// White, fully opaque text.
    pub const fn white() -> Self {
        Self::rgb(u8::MAX, u8::MAX, u8::MAX)
    }

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

/// Horizontal alignment for text rendering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

/// Alias using the more explicit property name.
pub type TextAlignment = Alignment;

/// The supported editable properties of the one 0.0.1 text widget.
#[derive(Clone, Debug, PartialEq)]
pub struct TextWidget {
    id: TextWidgetId,
    content: String,
    position: Position,
    font_size: f32,
    color: Color,
    alignment: Alignment,
}

impl TextWidget {
    /// Creates a text widget with useful default presentation properties.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: TextWidgetId(Uuid::new_v4()),
            content: content.into(),
            position: Position::origin(),
            font_size: 16.0,
            color: Color::white(),
            alignment: Alignment::Left,
        }
    }

    /// Creates a text widget with all supported presentation properties.
    pub fn with_properties(
        content: impl Into<String>,
        position: Position,
        font_size: f32,
        color: Color,
        alignment: Alignment,
    ) -> Result<Self, ModelError> {
        validate_position(position)?;
        validate_font_size(font_size)?;

        Ok(Self {
            id: TextWidgetId(Uuid::new_v4()),
            content: content.into(),
            position,
            font_size,
            color,
            alignment,
        })
    }

    pub(crate) fn from_parts(
        id: TextWidgetId,
        content: String,
        position: Position,
        font_size: f32,
        color: Color,
        alignment: Alignment,
    ) -> Result<Self, ModelError> {
        validate_position(position)?;
        validate_font_size(font_size)?;

        Ok(Self {
            id,
            content,
            position,
            font_size,
            color,
            alignment,
        })
    }

    /// Returns this widget's stable identity.
    pub const fn id(&self) -> TextWidgetId {
        self.id
    }

    /// Returns the text content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the canvas position.
    pub const fn position(&self) -> Position {
        self.position
    }

    /// Returns the font size in logical units.
    pub const fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Returns the RGBA text color.
    pub const fn color(&self) -> Color {
        self.color
    }

    /// Returns the horizontal text alignment.
    pub const fn alignment(&self) -> Alignment {
        self.alignment
    }
}

impl From<String> for TextWidget {
    fn from(content: String) -> Self {
        Self::new(content)
    }
}

impl From<&str> for TextWidget {
    fn from(content: &str) -> Self {
        Self::new(content)
    }
}

/// An error from constructing or mutating the overlay domain model.
#[derive(Clone, Debug, PartialEq)]
pub enum ModelError {
    /// An overlay name cannot be empty or only whitespace.
    EmptyName,
    /// Canvas dimensions must both be non-zero.
    InvalidCanvasSize { width: u32, height: u32 },
    /// A position must be finite, non-negative, and within the fixed canvas.
    InvalidPosition { x: f32, y: f32 },
    /// Font size must be finite and greater than zero.
    InvalidFontSize { value: f32 },
    /// The overlay already contains its one permitted text widget.
    TextWidgetAlreadyExists,
    /// The requested widget does not exist in this overlay.
    TextWidgetNotFound { id: TextWidgetId },
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("overlay name cannot be empty"),
            Self::InvalidCanvasSize { width, height } => {
                write!(
                    formatter,
                    "canvas dimensions must be non-zero (got {width}x{height})"
                )
            }
            Self::InvalidPosition { x, y } => {
                write!(
                    formatter,
                    "text position is invalid or outside the canvas ({x}, {y})"
                )
            }
            Self::InvalidFontSize { value } => {
                write!(
                    formatter,
                    "font size must be finite and greater than zero (got {value})"
                )
            }
            Self::TextWidgetAlreadyExists => {
                formatter.write_str("an overlay can contain at most one text widget")
            }
            Self::TextWidgetNotFound { id } => write!(formatter, "text widget {id} was not found"),
        }
    }
}

impl Error for ModelError {}

/// The authoritative, framework-independent overlay document.
#[derive(Clone, Debug, PartialEq)]
pub struct Overlay {
    id: OverlayId,
    name: String,
    canvas: CanvasSize,
    text_widget: Option<TextWidget>,
    revision: u64,
}

impl Overlay {
    /// Creates an empty overlay with a generated stable identity.
    pub fn new(name: impl Into<String>, canvas: CanvasSize) -> Result<Self, ModelError> {
        let name = name.into();
        validate_name(&name)?;

        Ok(Self {
            id: OverlayId(Uuid::new_v4()),
            name,
            canvas,
            text_widget: None,
            revision: 0,
        })
    }

    pub(crate) fn from_parts(
        id: OverlayId,
        name: String,
        canvas: CanvasSize,
        text_widget: Option<TextWidget>,
        revision: u64,
    ) -> Result<Self, ModelError> {
        validate_name(&name)?;
        if let Some(widget) = text_widget.as_ref() {
            validate_position(widget.position)?;
            if widget.position.x > canvas.width as f32 || widget.position.y > canvas.height as f32 {
                return Err(ModelError::InvalidPosition {
                    x: widget.position.x,
                    y: widget.position.y,
                });
            }
        }

        Ok(Self {
            id,
            name,
            canvas,
            text_widget,
            revision,
        })
    }

    /// Creates an empty overlay from explicit width and height values.
    pub fn with_dimensions(
        name: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<Self, ModelError> {
        Self::new(name, CanvasSize::new(width, height)?)
    }

    /// Returns this overlay's stable identity.
    pub const fn id(&self) -> OverlayId {
        self.id
    }

    /// Returns the user-facing overlay name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the fixed canvas dimensions.
    pub const fn canvas(&self) -> CanvasSize {
        self.canvas
    }

    /// Returns the optional text widget.
    pub fn text_widget(&self) -> Option<&TextWidget> {
        self.text_widget.as_ref()
    }

    /// Returns the monotonically increasing revision for browser snapshots.
    ///
    /// Revisions are ordering metadata, not identities. The initial empty
    /// overlay is revision zero; successful changes increment it.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Renames the overlay without changing its identity.
    pub fn rename(&mut self, name: impl Into<String>) -> Result<(), ModelError> {
        let name = name.into();
        validate_name(&name)?;

        if self.name != name {
            self.name = name;
            self.bump_revision();
        }
        Ok(())
    }

    /// Adds the one supported text widget and returns its stable identity.
    ///
    /// A `&str`, `String`, or already-created [`TextWidget`] can be supplied.
    /// A pre-created widget is useful when all presentation properties should
    /// be selected before insertion.
    pub fn add_text_widget(
        &mut self,
        widget: impl Into<TextWidget>,
    ) -> Result<TextWidgetId, ModelError> {
        if self.text_widget.is_some() {
            return Err(ModelError::TextWidgetAlreadyExists);
        }

        let widget = widget.into();
        self.validate_widget_position(widget.position)?;
        let id = widget.id;
        self.text_widget = Some(widget);
        self.bump_revision();
        Ok(id)
    }

    /// Removes the text widget only when the caller supplies its current ID.
    pub fn remove_text_widget(&mut self, id: TextWidgetId) -> Result<TextWidget, ModelError> {
        match self.text_widget.as_ref() {
            Some(widget) if widget.id == id => {
                let widget = self.text_widget.take().expect("widget checked above");
                self.bump_revision();
                Ok(widget)
            }
            _ => Err(ModelError::TextWidgetNotFound { id }),
        }
    }

    /// Updates text content while preserving the widget identity.
    pub fn set_text_content(
        &mut self,
        id: TextWidgetId,
        content: impl Into<String>,
    ) -> Result<(), ModelError> {
        let content = content.into();
        let widget = self.widget_mut(id)?;
        if widget.content != content {
            widget.content = content;
            self.bump_revision();
        }
        Ok(())
    }

    /// Updates the text position while preserving the widget identity.
    pub fn set_text_position(
        &mut self,
        id: TextWidgetId,
        position: Position,
    ) -> Result<(), ModelError> {
        self.validate_widget_position(position)?;
        let widget = self.widget_mut(id)?;
        if widget.position != position {
            widget.position = position;
            self.bump_revision();
        }
        Ok(())
    }

    /// Updates the font size while preserving the widget identity.
    pub fn set_text_font_size(
        &mut self,
        id: TextWidgetId,
        font_size: f32,
    ) -> Result<(), ModelError> {
        validate_font_size(font_size)?;
        let widget = self.widget_mut(id)?;
        if widget.font_size != font_size {
            widget.font_size = font_size;
            self.bump_revision();
        }
        Ok(())
    }

    /// Updates the text color while preserving the widget identity.
    pub fn set_text_color(&mut self, id: TextWidgetId, color: Color) -> Result<(), ModelError> {
        let widget = self.widget_mut(id)?;
        if widget.color != color {
            widget.color = color;
            self.bump_revision();
        }
        Ok(())
    }

    /// Updates text alignment while preserving the widget identity.
    pub fn set_text_alignment(
        &mut self,
        id: TextWidgetId,
        alignment: Alignment,
    ) -> Result<(), ModelError> {
        let widget = self.widget_mut(id)?;
        if widget.alignment != alignment {
            widget.alignment = alignment;
            self.bump_revision();
        }
        Ok(())
    }

    fn validate_widget_position(&self, position: Position) -> Result<(), ModelError> {
        if !position.is_finite_and_nonnegative()
            || position.x > self.canvas.width as f32
            || position.y > self.canvas.height as f32
        {
            return Err(ModelError::InvalidPosition {
                x: position.x,
                y: position.y,
            });
        }
        Ok(())
    }

    fn widget_mut(&mut self, id: TextWidgetId) -> Result<&mut TextWidget, ModelError> {
        match self.text_widget.as_mut() {
            Some(widget) if widget.id == id => Ok(widget),
            _ => Err(ModelError::TextWidgetNotFound { id }),
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }
}

fn validate_name(name: &str) -> Result<(), ModelError> {
    if name.trim().is_empty() {
        Err(ModelError::EmptyName)
    } else {
        Ok(())
    }
}

fn validate_position(position: Position) -> Result<(), ModelError> {
    if position.is_finite_and_nonnegative() {
        Ok(())
    } else {
        Err(ModelError::InvalidPosition {
            x: position.x,
            y: position.y,
        })
    }
}

fn validate_font_size(font_size: f32) -> Result<(), ModelError> {
    if font_size.is_finite() && font_size > 0.0 {
        Ok(())
    } else {
        Err(ModelError::InvalidFontSize { value: font_size })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANVAS: CanvasSize = CanvasSize {
        width: 1920,
        height: 1080,
    };

    fn overlay() -> Overlay {
        Overlay::new("Starting Soon", CANVAS).expect("valid overlay")
    }

    #[test]
    fn creation_has_expected_defaults_and_v4_overlay_id() {
        let overlay = overlay();

        assert_eq!(overlay.name(), "Starting Soon");
        assert_eq!(overlay.canvas(), CANVAS);
        assert!(overlay.text_widget().is_none());
        assert_eq!(overlay.revision(), 0);
        assert_eq!(overlay.id().as_uuid().get_version_num(), 4);
    }

    #[test]
    fn separately_created_overlays_have_distinct_ids() {
        let first = overlay();
        let second = overlay();

        assert_ne!(first.id(), second.id());
    }

    #[test]
    fn rename_preserves_id_and_increments_revision() {
        let mut overlay = overlay();
        let id = overlay.id();

        overlay.rename("Live Now").expect("valid rename");

        assert_eq!(overlay.id(), id);
        assert_eq!(overlay.name(), "Live Now");
        assert_eq!(overlay.revision(), 1);
    }

    #[test]
    fn widget_cardinality_is_zero_or_one() {
        let mut overlay = overlay();
        let first = overlay.add_text_widget("hello").expect("first widget");
        let second = overlay.add_text_widget("second");

        assert_eq!(second, Err(ModelError::TextWidgetAlreadyExists));
        assert_eq!(overlay.text_widget().expect("widget").id(), first);
        assert_eq!(overlay.revision(), 1);

        let removed = overlay
            .remove_text_widget(first)
            .expect("remove existing widget");
        assert_eq!(removed.id(), first);
        assert!(overlay.text_widget().is_none());
        assert_eq!(overlay.revision(), 2);
    }

    #[test]
    fn widget_creation_and_supported_updates_preserve_id() {
        let mut overlay = overlay();
        let widget = TextWidget::with_properties(
            "hello",
            Position::new(12.0, 34.0),
            24.0,
            Color::rgba(10, 20, 30, 40),
            Alignment::Center,
        )
        .expect("valid widget");
        let id = widget.id();

        assert_eq!(id.as_uuid().get_version_num(), 4);
        overlay.add_text_widget(widget).expect("add widget");
        overlay
            .set_text_content(id, "updated")
            .expect("content update");
        overlay
            .set_text_position(id, Position::new(100.0, 200.0))
            .expect("position update");
        overlay.set_text_font_size(id, 32.0).expect("font update");
        overlay
            .set_text_color(id, Color::rgb(1, 2, 3))
            .expect("color update");
        overlay
            .set_text_alignment(id, Alignment::Right)
            .expect("alignment update");

        let widget = overlay.text_widget().expect("widget");
        assert_eq!(widget.id(), id);
        assert_eq!(widget.content(), "updated");
        assert_eq!(widget.position(), Position::new(100.0, 200.0));
        assert_eq!(widget.font_size(), 32.0);
        assert_eq!(widget.color(), Color::rgb(1, 2, 3));
        assert_eq!(widget.alignment(), Alignment::Right);
        assert_eq!(overlay.revision(), 6);
    }

    #[test]
    fn invalid_operations_do_not_mutate_or_advance_revision() {
        assert_eq!(Overlay::new("", CANVAS), Err(ModelError::EmptyName));
        assert_eq!(Overlay::new("   ", CANVAS), Err(ModelError::EmptyName));
        assert_eq!(
            CanvasSize::new(0, 1080),
            Err(ModelError::InvalidCanvasSize {
                width: 0,
                height: 1080
            })
        );
        assert_eq!(
            CanvasSize::new(1920, 0),
            Err(ModelError::InvalidCanvasSize {
                width: 1920,
                height: 0
            })
        );

        let mut overlay = overlay();
        let id = overlay.add_text_widget("hello").expect("widget");
        let revision = overlay.revision();

        assert_eq!(
            overlay.set_text_font_size(id, 0.0),
            Err(ModelError::InvalidFontSize { value: 0.0 })
        );
        assert_eq!(
            overlay.set_text_position(id, Position::new(-1.0, 0.0)),
            Err(ModelError::InvalidPosition { x: -1.0, y: 0.0 })
        );
        assert_eq!(
            overlay.set_text_position(id, Position::new(1921.0, 0.0)),
            Err(ModelError::InvalidPosition { x: 1921.0, y: 0.0 })
        );
        let missing = TextWidgetId(Uuid::new_v4());
        assert_eq!(
            overlay.set_text_content(missing, "ignored"),
            Err(ModelError::TextWidgetNotFound { id: missing })
        );
        assert_eq!(overlay.revision(), revision);
        assert_eq!(overlay.text_widget().expect("widget").content(), "hello");
    }

    #[test]
    fn no_op_updates_do_not_create_spurious_revisions() {
        let mut overlay = overlay();
        let id = overlay.add_text_widget("hello").expect("widget");
        let revision = overlay.revision();

        overlay.rename("Starting Soon").expect("same name is valid");
        overlay
            .set_text_content(id, "hello")
            .expect("same content is valid");
        overlay
            .set_text_position(id, Position::origin())
            .expect("same position is valid");
        overlay
            .set_text_font_size(id, 16.0)
            .expect("same font size is valid");
        overlay
            .set_text_color(id, Color::white())
            .expect("same color is valid");
        overlay
            .set_text_alignment(id, Alignment::Left)
            .expect("same alignment is valid");

        assert_eq!(overlay.revision(), revision);
    }
}
