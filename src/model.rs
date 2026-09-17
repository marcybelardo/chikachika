//! Framework-independent ordered overlay document model.
//!
//! The model owns durable overlay content only. Selection, delivery revisions,
//! persistence, and rendering are deliberately kept in adapters.

use std::error::Error;
use std::fmt;

use uuid::Uuid;

/// The identity of an overlay.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OverlayId(Uuid);

impl OverlayId {
    pub fn as_uuid(self) -> Uuid {
        self.0
    }

    pub(crate) const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub(crate) fn is_valid(self) -> bool {
        !self.0.is_nil() && self.0.get_version_num() == 4
    }
}

impl fmt::Display for OverlayId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The stable identity of a text widget.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextWidgetId(Uuid);

impl TextWidgetId {
    pub fn as_uuid(self) -> Uuid {
        self.0
    }

    pub(crate) const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub(crate) fn is_valid(self) -> bool {
        !self.0.is_nil() && self.0.get_version_num() == 4
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
    pub fn new(width: u32, height: u32) -> Result<Self, ModelError> {
        if width == 0 || height == 0 {
            return Err(ModelError::InvalidCanvasSize { width, height });
        }
        Ok(Self { width, height })
    }

    pub const fn width(self) -> u32 {
        self.width
    }
    pub const fn height(self) -> u32 {
        self.height
    }
}

/// A text widget anchor position in canvas coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    x: f32,
    y: f32,
}

impl Position {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub const fn origin() -> Self {
        Self::new(0.0, 0.0)
    }
    pub const fn x(self) -> f32 {
        self.x
    }
    pub const fn y(self) -> f32 {
        self.y
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
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
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, u8::MAX)
    }
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
    pub const fn white() -> Self {
        Self::rgb(u8::MAX, u8::MAX, u8::MAX)
    }
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

/// Horizontal alignment for text rendering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

pub type TextAlignment = Alignment;

/// Stable font-family identifiers accepted by the document format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontFamily {
    NotoSans,
    JetBrainsMono,
}

impl FontFamily {
    pub const DEFAULT: Self = Self::NotoSans;

    pub const fn id(self) -> &'static str {
        match self {
            Self::NotoSans => "noto-sans",
            Self::JetBrainsMono => "jetbrains-mono",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::NotoSans => "Noto Sans",
            Self::JetBrainsMono => "JetBrains Mono",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ModelError> {
        match value {
            "noto-sans" => Ok(Self::NotoSans),
            "jetbrains-mono" => Ok(Self::JetBrainsMono),
            _ => Err(ModelError::UnknownFontFamily {
                value: value.to_owned(),
            }),
        }
    }
}

/// A durable text widget in frontmost-first collection order.
#[derive(Clone, Debug, PartialEq)]
pub struct TextWidget {
    id: TextWidgetId,
    name: String,
    content: String,
    font_family: FontFamily,
    position: Position,
    font_size: f32,
    color: Color,
    alignment: Alignment,
}

impl TextWidget {
    /// Creates a widget with a generated v4 ID and default presentation values.
    pub fn new(content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            id: TextWidgetId(Uuid::new_v4()),
            name: default_widget_name(&content),
            content,
            font_family: FontFamily::DEFAULT,
            position: Position::origin(),
            font_size: 16.0,
            color: Color::white(),
            alignment: Alignment::Left,
        }
    }

    /// Creates a widget with the original baseline presentation properties.
    pub fn with_properties(
        content: impl Into<String>,
        position: Position,
        font_size: f32,
        color: Color,
        alignment: Alignment,
    ) -> Result<Self, ModelError> {
        let content = content.into();
        let name = default_widget_name(&content);
        Self::with_all_properties(
            name,
            content,
            FontFamily::DEFAULT,
            position,
            font_size,
            color,
            alignment,
        )
    }

    /// Creates a widget with every durable property selected by the caller.
    pub fn with_all_properties(
        name: impl Into<String>,
        content: impl Into<String>,
        font_family: FontFamily,
        position: Position,
        font_size: f32,
        color: Color,
        alignment: Alignment,
    ) -> Result<Self, ModelError> {
        let name = name.into();
        let content = content.into();
        validate_name(&name)?;
        validate_position(position)?;
        validate_font_size(font_size)?;
        Ok(Self {
            id: TextWidgetId(Uuid::new_v4()),
            name,
            content,
            font_family,
            position,
            font_size,
            color,
            alignment,
        })
    }

    pub(crate) fn from_parts(
        id: TextWidgetId,
        name: String,
        content: String,
        font_family: FontFamily,
        position: Position,
        font_size: f32,
        color: Color,
        alignment: Alignment,
    ) -> Result<Self, ModelError> {
        if !id.is_valid() {
            return Err(ModelError::InvalidWidgetId { id });
        }
        validate_name(&name)?;
        validate_position(position)?;
        validate_font_size(font_size)?;
        Ok(Self {
            id,
            name,
            content,
            font_family,
            position,
            font_size,
            color,
            alignment,
        })
    }

    pub const fn id(&self) -> TextWidgetId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn content(&self) -> &str {
        &self.content
    }
    pub const fn font_family(&self) -> FontFamily {
        self.font_family
    }
    pub const fn position(&self) -> Position {
        self.position
    }
    pub const fn font_size(&self) -> f32 {
        self.font_size
    }
    pub const fn color(&self) -> Color {
        self.color
    }
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

/// An error constructing or mutating durable model content.
#[derive(Clone, Debug, PartialEq)]
pub enum ModelError {
    EmptyName,
    InvalidCanvasSize {
        width: u32,
        height: u32,
    },
    InvalidPosition {
        x: f32,
        y: f32,
    },
    InvalidFontSize {
        value: f32,
    },
    InvalidOverlayId {
        id: OverlayId,
    },
    OverlayIdentityChanged {
        expected: OverlayId,
        found: OverlayId,
    },
    InvalidWidgetId {
        id: TextWidgetId,
    },
    DuplicateOverlayId {
        id: OverlayId,
    },
    DuplicateWidgetId {
        id: TextWidgetId,
    },
    UnknownFontFamily {
        value: String,
    },
    TextWidgetNotFound {
        id: TextWidgetId,
    },
    WidgetAlreadyExists {
        id: TextWidgetId,
    },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "overlay name cannot be empty"),
            Self::InvalidCanvasSize { width, height } => write!(
                f,
                "canvas dimensions must be non-zero (got {width}x{height})"
            ),
            Self::InvalidPosition { x, y } => write!(
                f,
                "text position is invalid or outside the canvas ({x}, {y})"
            ),
            Self::InvalidFontSize { value } => write!(
                f,
                "font size must be finite and greater than zero (got {value})"
            ),
            Self::InvalidOverlayId { id } => write!(f, "overlay ID {id} is not a non-nil UUID v4"),
            Self::OverlayIdentityChanged { expected, found } => {
                write!(
                    f,
                    "overlay mutation changed identity from {expected} to {found}"
                )
            }
            Self::InvalidWidgetId { id } => write!(f, "widget ID {id} is not a non-nil UUID v4"),
            Self::DuplicateOverlayId { id } => write!(f, "duplicate overlay ID {id}"),
            Self::DuplicateWidgetId { id } => write!(f, "duplicate widget ID {id}"),
            Self::UnknownFontFamily { value } => write!(f, "unknown font family {value}"),
            Self::TextWidgetNotFound { id } => write!(f, "text widget {id} was not found"),
            Self::WidgetAlreadyExists { id } => write!(f, "text widget {id} already exists"),
        }
    }
}
impl Error for ModelError {}

/// The authoritative ordered overlay document. Index zero is frontmost.
#[derive(Clone, Debug, PartialEq)]
pub struct Overlay {
    id: OverlayId,
    name: String,
    canvas: CanvasSize,
    widgets: Vec<TextWidget>,
}

impl Overlay {
    pub fn new(name: impl Into<String>, canvas: CanvasSize) -> Result<Self, ModelError> {
        let name = name.into();
        validate_name(&name)?;
        Ok(Self {
            id: OverlayId(Uuid::new_v4()),
            name,
            canvas,
            widgets: Vec::new(),
        })
    }

    pub(crate) fn from_parts(
        id: OverlayId,
        name: String,
        canvas: CanvasSize,
        widgets: Vec<TextWidget>,
    ) -> Result<Self, ModelError> {
        if !id.is_valid() {
            return Err(ModelError::InvalidOverlayId { id });
        }
        validate_name(&name)?;
        let mut seen = std::collections::HashSet::with_capacity(widgets.len());
        for widget in &widgets {
            if !seen.insert(widget.id()) {
                return Err(ModelError::DuplicateWidgetId { id: widget.id() });
            }
            validate_widget_position(canvas, widget.position())?;
        }
        Ok(Self {
            id,
            name,
            canvas,
            widgets,
        })
    }

    pub fn with_dimensions(
        name: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<Self, ModelError> {
        Self::new(name, CanvasSize::new(width, height)?)
    }
    pub const fn id(&self) -> OverlayId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub const fn canvas(&self) -> CanvasSize {
        self.canvas
    }
    pub fn widgets(&self) -> &[TextWidget] {
        &self.widgets
    }
    pub fn widget(&self, id: TextWidgetId) -> Option<&TextWidget> {
        self.widgets.iter().find(|widget| widget.id() == id)
    }
    pub(crate) fn widget_index(&self, id: TextWidgetId) -> Option<usize> {
        self.widgets.iter().position(|widget| widget.id() == id)
    }

    pub fn rename(&mut self, name: impl Into<String>) -> Result<(), ModelError> {
        let name = name.into();
        validate_name(&name)?;
        self.name = name;
        Ok(())
    }

    /// Inserts at index zero (frontmost), returning the inserted stable ID.
    pub fn add_widget(
        &mut self,
        widget: impl Into<TextWidget>,
    ) -> Result<TextWidgetId, ModelError> {
        let widget = widget.into();
        validate_widget_position(self.canvas, widget.position())?;
        if self
            .widgets
            .iter()
            .any(|existing| existing.id() == widget.id())
        {
            return Err(ModelError::WidgetAlreadyExists { id: widget.id() });
        }
        let id = widget.id();
        self.widgets.insert(0, widget);
        Ok(id)
    }

    pub fn duplicate_widget(&mut self, id: TextWidgetId) -> Result<TextWidgetId, ModelError> {
        let source = self
            .widget(id)
            .cloned()
            .ok_or(ModelError::TextWidgetNotFound { id })?;
        let duplicate = TextWidget {
            id: TextWidgetId(Uuid::new_v4()),
            name: format!("{} Copy", source.name()),
            content: source.content,
            font_family: source.font_family,
            position: source.position,
            font_size: source.font_size,
            color: source.color,
            alignment: source.alignment,
        };
        let duplicate_id = duplicate.id();
        self.widgets.insert(0, duplicate);
        Ok(duplicate_id)
    }

    pub fn delete_widget(&mut self, id: TextWidgetId) -> Result<TextWidget, ModelError> {
        let index = self
            .widget_index(id)
            .ok_or(ModelError::TextWidgetNotFound { id })?;
        Ok(self.widgets.remove(index))
    }

    pub fn rename_widget(
        &mut self,
        id: TextWidgetId,
        name: impl Into<String>,
    ) -> Result<(), ModelError> {
        let name = name.into();
        validate_name(&name)?;
        self.widget_mut(id)?.name = name;
        Ok(())
    }

    pub fn set_widget_content(
        &mut self,
        id: TextWidgetId,
        content: impl Into<String>,
    ) -> Result<(), ModelError> {
        self.widget_mut(id)?.content = content.into();
        Ok(())
    }
    pub fn set_widget_position(
        &mut self,
        id: TextWidgetId,
        position: Position,
    ) -> Result<(), ModelError> {
        validate_widget_position(self.canvas, position)?;
        self.widget_mut(id)?.position = position;
        Ok(())
    }
    pub fn set_widget_font_size(
        &mut self,
        id: TextWidgetId,
        font_size: f32,
    ) -> Result<(), ModelError> {
        validate_font_size(font_size)?;
        self.widget_mut(id)?.font_size = font_size;
        Ok(())
    }
    pub fn set_widget_color(&mut self, id: TextWidgetId, color: Color) -> Result<(), ModelError> {
        self.widget_mut(id)?.color = color;
        Ok(())
    }
    pub fn set_widget_alignment(
        &mut self,
        id: TextWidgetId,
        alignment: Alignment,
    ) -> Result<(), ModelError> {
        self.widget_mut(id)?.alignment = alignment;
        Ok(())
    }
    pub fn set_widget_font_family(
        &mut self,
        id: TextWidgetId,
        font_family: FontFamily,
    ) -> Result<(), ModelError> {
        self.widget_mut(id)?.font_family = font_family;
        Ok(())
    }

    /// Moves one layer toward the front. Boundary calls are successful no-ops.
    pub fn move_widget_forward(&mut self, id: TextWidgetId) -> Result<(), ModelError> {
        let index = self
            .widget_index(id)
            .ok_or(ModelError::TextWidgetNotFound { id })?;
        if index > 0 {
            self.widgets.swap(index, index - 1);
        }
        Ok(())
    }
    /// Moves one layer toward the back. Boundary calls are successful no-ops.
    pub fn move_widget_backward(&mut self, id: TextWidgetId) -> Result<(), ModelError> {
        let index = self
            .widget_index(id)
            .ok_or(ModelError::TextWidgetNotFound { id })?;
        if index + 1 < self.widgets.len() {
            self.widgets.swap(index, index + 1);
        }
        Ok(())
    }

    fn widget_mut(&mut self, id: TextWidgetId) -> Result<&mut TextWidget, ModelError> {
        self.widgets
            .iter_mut()
            .find(|widget| widget.id() == id)
            .ok_or(ModelError::TextWidgetNotFound { id })
    }
}

pub(crate) fn validate_collection(overlays: &[Overlay]) -> Result<(), ModelError> {
    let mut overlay_ids = std::collections::HashSet::with_capacity(overlays.len());
    let mut widget_ids = std::collections::HashSet::new();
    for overlay in overlays {
        if !overlay.id().is_valid() {
            return Err(ModelError::InvalidOverlayId { id: overlay.id() });
        }
        if !overlay_ids.insert(overlay.id()) {
            return Err(ModelError::DuplicateOverlayId { id: overlay.id() });
        }
        validate_name(overlay.name())?;
        for widget in overlay.widgets() {
            if !widget.id().is_valid() {
                return Err(ModelError::InvalidWidgetId { id: widget.id() });
            }
            if !widget_ids.insert(widget.id()) {
                return Err(ModelError::DuplicateWidgetId { id: widget.id() });
            }
            validate_widget_position(overlay.canvas(), widget.position())?;
            validate_font_size(widget.font_size())?;
            validate_name(widget.name())?;
        }
    }
    Ok(())
}

fn default_widget_name(content: &str) -> String {
    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Text")
        .to_owned()
}
fn validate_name(name: &str) -> Result<(), ModelError> {
    if name.trim().is_empty() {
        Err(ModelError::EmptyName)
    } else {
        Ok(())
    }
}
fn validate_widget_position(canvas: CanvasSize, position: Position) -> Result<(), ModelError> {
    if !position.is_finite()
        || position.x < 0.0
        || position.y < 0.0
        || position.x > canvas.width as f32
        || position.y > canvas.height as f32
    {
        Err(ModelError::InvalidPosition {
            x: position.x,
            y: position.y,
        })
    } else {
        Ok(())
    }
}
fn validate_position(position: Position) -> Result<(), ModelError> {
    if position.is_finite() && position.x >= 0.0 && position.y >= 0.0 {
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
        Overlay::new("Starting Soon", CANVAS).unwrap()
    }

    #[test]
    fn ordered_widget_mutations() {
        let mut overlay = overlay();
        let back = overlay.add_widget("back").unwrap();
        let front = overlay.add_widget("front").unwrap();
        assert_eq!(
            overlay
                .widgets()
                .iter()
                .map(TextWidget::id)
                .collect::<Vec<_>>(),
            vec![front, back]
        );
        overlay.move_widget_backward(front).unwrap();
        assert_eq!(overlay.widgets()[1].id(), front);
        overlay.move_widget_forward(front).unwrap();
        assert_eq!(overlay.widgets()[0].id(), front);
        let copy = overlay.duplicate_widget(back).unwrap();
        assert_eq!(overlay.widgets()[0].id(), copy);
        assert_eq!(overlay.widget(copy).unwrap().name(), "back Copy");
        overlay.delete_widget(copy).unwrap();
        assert!(overlay.widget(copy).is_none());
    }

    #[test]
    fn widget_names_and_duplicate_identity() {
        let mut overlay = overlay();
        let id = overlay.add_widget("  Hello  \nsecond").unwrap();
        assert_eq!(overlay.widget(id).unwrap().name(), "Hello");
        let copy = overlay.duplicate_widget(id).unwrap();
        assert_ne!(id, copy);
        assert_eq!(overlay.widget(copy).unwrap().content(), "  Hello  \nsecond");
        assert_eq!(copy.as_uuid().get_version_num(), 4);
        assert_eq!(
            overlay.rename_widget(id, "  ").unwrap_err(),
            ModelError::EmptyName
        );
    }

    #[test]
    fn widget_property_edits_and_bounds() {
        let mut overlay = overlay();
        let id = overlay
            .add_widget(
                TextWidget::with_all_properties(
                    "Name",
                    "text",
                    FontFamily::JetBrainsMono,
                    Position::origin(),
                    22.0,
                    Color::rgb(1, 2, 3),
                    Alignment::Center,
                )
                .unwrap(),
            )
            .unwrap();
        overlay
            .set_widget_font_family(id, FontFamily::NotoSans)
            .unwrap();
        overlay
            .set_widget_position(id, Position::new(1920.0, 1080.0))
            .unwrap();
        assert_eq!(
            overlay.set_widget_position(id, Position::new(1921.0, 0.0)),
            Err(ModelError::InvalidPosition { x: 1921.0, y: 0.0 })
        );
        assert_eq!(
            overlay.set_widget_font_size(id, f32::NAN),
            Err(ModelError::InvalidFontSize { value: f32::NAN })
        );
        assert_eq!(
            overlay.widget(id).unwrap().font_family(),
            FontFamily::NotoSans
        );
    }

    #[test]
    fn collection_identity_validation() {
        let overlay = overlay();
        assert_eq!(
            Overlay::from_parts(
                OverlayId::from_uuid(Uuid::nil()),
                "x".into(),
                CANVAS,
                vec![]
            ),
            Err(ModelError::InvalidOverlayId {
                id: OverlayId::from_uuid(Uuid::nil())
            })
        );
        assert!(validate_collection(&[overlay.clone(), overlay]).is_err());
        let invalid = TextWidget::from_parts(
            TextWidgetId::from_uuid(Uuid::nil()),
            "x".into(),
            "x".into(),
            FontFamily::DEFAULT,
            Position::origin(),
            16.0,
            Color::white(),
            Alignment::Left,
        );
        assert!(invalid.is_err());
    }

    #[test]
    fn equality_is_durable_content_only() {
        let mut first = overlay();
        let mut second = first.clone();
        assert_eq!(first, second);
        first.add_widget("hello").unwrap();
        assert_ne!(first, second);
        second.add_widget("hello").unwrap();
        // IDs are durable, so independently added widgets are intentionally different.
        assert_ne!(first, second);
        assert_eq!(first, first.clone());
    }
}
