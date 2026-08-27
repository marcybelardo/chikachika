//! Versioned, app-local persistence for overlay documents.
//!
//! This adapter deliberately uses separate Serde DTOs instead of adding
//! serialization concerns to the framework-independent domain model.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::model::{Alignment, CanvasSize, Color, ModelError, Overlay, Position, TextWidget};

/// The only persistence format currently understood by this version.
pub const FORMAT_VERSION: u32 = 1;

/// The filename used below the platform's app-local data directory.
pub const FILE_NAME: &str = "overlays.json";

/// Resolves the app-local persistence path without consulting the working
/// directory. The empty qualifier and organization are intentional: the
/// unqualified application identity is `Chikachika`.
pub fn app_local_path() -> Result<PathBuf, PersistenceError> {
    let project_dirs = ProjectDirs::from("", "", "Chikachika")
        .ok_or(PersistenceError::AppLocalDirectoryUnavailable)?;
    Ok(project_dirs.data_local_dir().join(FILE_NAME))
}

/// Loads overlays from an explicitly selected path.
///
/// A missing file is treated as an empty collection, while its parent
/// directory is still created explicitly. Every other failure is returned to
/// the caller and leaves the caller's in-memory state untouched because this
/// function only returns a new collection after complete validation.
pub fn load(path: impl AsRef<Path>) -> Result<Vec<Overlay>, PersistenceError> {
    Store::at(path.as_ref()).load()
}

/// Saves a complete overlay snapshot to an explicitly selected path.
///
/// Serialization happens before any replacement is attempted. The caller
/// retains ownership of `overlays`, so a failure cannot clear or otherwise
/// alter dirty in-memory work.
pub fn save(path: impl AsRef<Path>, overlays: &[Overlay]) -> Result<(), PersistenceError> {
    Store::at(path.as_ref()).save(overlays)
}

/// A persistence store for one versioned JSON file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    /// Creates a store at the platform app-local path.
    pub fn app_local() -> Result<Self, PersistenceError> {
        Ok(Self::at(app_local_path()?))
    }

    /// Creates a store at `path`. This constructor does not touch the
    /// filesystem, making it suitable for deterministic tests and callers
    /// that want to display the selected path before I/O begins.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the exact source path used by this store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Creates the parent app-local data directory explicitly.
    pub fn ensure_data_dir(&self) -> Result<(), PersistenceError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| PersistenceError::NoParentDirectory {
                path: self.path.clone(),
            })?;
        fs::create_dir_all(parent).map_err(|source| PersistenceError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })
    }

    /// Loads and validates the complete envelope at this store's path.
    pub fn load(&self) -> Result<Vec<Overlay>, PersistenceError> {
        self.ensure_data_dir()?;

        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(PersistenceError::Read {
                    path: self.path.clone(),
                    source,
                });
            }
        };

        let envelope: Envelope =
            serde_json::from_slice(&bytes).map_err(|source| PersistenceError::Malformed {
                path: self.path.clone(),
                source,
            })?;
        envelope.into_models()
    }

    /// Serializes and atomically replaces this store's source file.
    pub fn save(&self, overlays: &[Overlay]) -> Result<(), PersistenceError> {
        let envelope = Envelope::from_models(overlays);
        let bytes = serde_json::to_vec_pretty(&envelope).map_err(PersistenceError::Serialize)?;
        self.ensure_data_dir()?;

        let parent = self
            .path
            .parent()
            .ok_or_else(|| PersistenceError::NoParentDirectory {
                path: self.path.clone(),
            })?;
        let mut temporary =
            NamedTempFile::new_in(parent).map_err(|source| PersistenceError::CreateTemporary {
                directory: parent.to_path_buf(),
                source,
            })?;

        temporary
            .write_all(&bytes)
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|source| PersistenceError::WriteTemporary {
                path: temporary.path().to_path_buf(),
                source,
            })?;

        // Close the temporary file before replacement. This is required on
        // platforms that do not allow renaming an open file. TempPath removes
        // the file automatically if replacement fails.
        let temporary = temporary.into_temp_path();
        replace_file(&temporary, &self.path).map_err(|source| PersistenceError::Replace {
            path: self.path.clone(),
            source,
        })?;

        // The temporary path no longer exists after replacement, so dropping
        // TempPath is harmless and keeps cleanup automatic on earlier errors.
        Ok(())
    }
}

/// The stable top-level JSON representation. Fields are private so callers
/// use the model-facing API rather than coupling to the on-disk DTO.
#[derive(Debug, Deserialize, Serialize)]
struct Envelope {
    format_version: u32,
    overlays: Vec<PersistedOverlay>,
}

impl Envelope {
    fn from_models(overlays: &[Overlay]) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            overlays: overlays.iter().map(PersistedOverlay::from_model).collect(),
        }
    }

    fn into_models(self) -> Result<Vec<Overlay>, PersistenceError> {
        if self.format_version != FORMAT_VERSION {
            return Err(PersistenceError::UnsupportedVersion {
                found: self.format_version,
                supported: FORMAT_VERSION,
            });
        }

        let mut ids = HashSet::with_capacity(self.overlays.len());
        self.overlays
            .into_iter()
            .map(|overlay| {
                if !ids.insert(overlay.id) {
                    return Err(PersistenceError::DuplicateOverlayId { id: overlay.id });
                }
                overlay.into_model()
            })
            .collect()
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedOverlay {
    id: Uuid,
    name: String,
    canvas: PersistedCanvasSize,
    text_widget: Option<PersistedTextWidget>,
    revision: u64,
}

impl PersistedOverlay {
    fn from_model(overlay: &Overlay) -> Self {
        Self {
            id: overlay.id().as_uuid(),
            name: overlay.name().to_owned(),
            canvas: PersistedCanvasSize::from_model(overlay.canvas()),
            text_widget: overlay.text_widget().map(PersistedTextWidget::from_model),
            revision: overlay.revision(),
        }
    }

    fn into_model(self) -> Result<Overlay, PersistenceError> {
        let text_widget = self
            .text_widget
            .map(PersistedTextWidget::into_model)
            .transpose()?;
        Overlay::from_parts(
            crate::model::OverlayId::from_uuid(self.id),
            self.name,
            self.canvas.into_model()?,
            text_widget,
            self.revision,
        )
        .map_err(PersistenceError::InvalidModel)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedCanvasSize {
    width: u32,
    height: u32,
}

impl PersistedCanvasSize {
    fn from_model(canvas: CanvasSize) -> Self {
        Self {
            width: canvas.width(),
            height: canvas.height(),
        }
    }

    fn into_model(self) -> Result<CanvasSize, PersistenceError> {
        CanvasSize::new(self.width, self.height).map_err(PersistenceError::InvalidModel)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedTextWidget {
    id: Uuid,
    content: String,
    position: PersistedPosition,
    font_size: f32,
    color: PersistedColor,
    alignment: PersistedAlignment,
}

impl PersistedTextWidget {
    fn from_model(widget: &TextWidget) -> Self {
        Self {
            id: widget.id().as_uuid(),
            content: widget.content().to_owned(),
            position: PersistedPosition::from_model(widget.position()),
            font_size: widget.font_size(),
            color: PersistedColor::from_model(widget.color()),
            alignment: PersistedAlignment::from_model(widget.alignment()),
        }
    }

    fn into_model(self) -> Result<TextWidget, PersistenceError> {
        TextWidget::from_parts(
            crate::model::TextWidgetId::from_uuid(self.id),
            self.content,
            self.position.into_model(),
            self.font_size,
            self.color.into_model(),
            self.alignment.into_model(),
        )
        .map_err(PersistenceError::InvalidModel)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedPosition {
    x: f32,
    y: f32,
}

impl PersistedPosition {
    fn from_model(position: Position) -> Self {
        Self {
            x: position.x(),
            y: position.y(),
        }
    }

    fn into_model(self) -> Position {
        Position::new(self.x, self.y)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedColor {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl PersistedColor {
    fn from_model(color: Color) -> Self {
        Self {
            red: color.red(),
            green: color.green(),
            blue: color.blue(),
            alpha: color.alpha(),
        }
    }

    fn into_model(self) -> Color {
        Color::rgba(self.red, self.green, self.blue, self.alpha)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum PersistedAlignment {
    Left,
    Center,
    Right,
}

impl PersistedAlignment {
    fn from_model(alignment: Alignment) -> Self {
        match alignment {
            Alignment::Left => Self::Left,
            Alignment::Center => Self::Center,
            Alignment::Right => Self::Right,
        }
    }

    fn into_model(self) -> Alignment {
        match self {
            Self::Left => Alignment::Left,
            Self::Center => Alignment::Center,
            Self::Right => Alignment::Right,
        }
    }
}

/// An error reported by path resolution, JSON validation, or file I/O.
#[derive(Debug)]
pub enum PersistenceError {
    AppLocalDirectoryUnavailable,
    NoParentDirectory {
        path: PathBuf,
    },
    CreateDirectory {
        path: PathBuf,
        source: io::Error,
    },
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Malformed {
        path: PathBuf,
        source: serde_json::Error,
    },
    Serialize(serde_json::Error),
    UnsupportedVersion {
        found: u32,
        supported: u32,
    },
    InvalidModel(ModelError),
    DuplicateOverlayId {
        id: Uuid,
    },
    CreateTemporary {
        directory: PathBuf,
        source: io::Error,
    },
    WriteTemporary {
        path: PathBuf,
        source: io::Error,
    },
    Replace {
        path: PathBuf,
        source: io::Error,
    },
    CleanupTemporary {
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppLocalDirectoryUnavailable => {
                formatter.write_str("the platform app-local data directory is unavailable")
            }
            Self::NoParentDirectory { path } => {
                write!(
                    formatter,
                    "persistence path has no parent directory: {}",
                    path.display()
                )
            }
            Self::CreateDirectory { path, source } => {
                write!(
                    formatter,
                    "could not create persistence directory {}: {source}",
                    path.display()
                )
            }
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "could not read persisted overlays {}: {source}",
                    path.display()
                )
            }
            Self::Malformed { path, source } => {
                write!(
                    formatter,
                    "persisted overlays {} are malformed: {source}",
                    path.display()
                )
            }
            Self::Serialize(source) => write!(formatter, "could not serialize overlays: {source}"),
            Self::UnsupportedVersion { found, supported } => write!(
                formatter,
                "persisted overlays use unsupported format version {found}; supported version is {supported}"
            ),
            Self::InvalidModel(source) => {
                write!(formatter, "persisted overlay data is invalid: {source}")
            }
            Self::DuplicateOverlayId { id } => {
                write!(
                    formatter,
                    "persisted overlays contain duplicate overlay ID {id}"
                )
            }
            Self::CreateTemporary { directory, source } => write!(
                formatter,
                "could not create temporary persistence file in {}: {source}",
                directory.display()
            ),
            Self::WriteTemporary { path, source } => {
                write!(
                    formatter,
                    "could not write temporary persistence file {}: {source}",
                    path.display()
                )
            }
            Self::Replace { path, source } => write!(
                formatter,
                "could not atomically replace persisted overlays {}: {source}",
                path.display()
            ),
            Self::CleanupTemporary { path, source } => write!(
                formatter,
                "could not finalize temporary persistence file {}: {source}",
                path.display()
            ),
        }
    }
}

impl Error for PersistenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateDirectory { source, .. }
            | Self::Read { source, .. }
            | Self::CreateTemporary { source, .. }
            | Self::WriteTemporary { source, .. }
            | Self::Replace { source, .. }
            | Self::CleanupTemporary { source, .. } => Some(source),
            Self::Malformed { source, .. } | Self::Serialize(source) => Some(source),
            Self::InvalidModel(source) => Some(source),
            _ => None,
        }
    }
}

fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        replace_file_windows(source, destination)
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, destination)
    }
}

#[cfg(windows)]
fn replace_file_windows(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    unsafe extern "system" {
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let replaced = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_overlay() -> Overlay {
        let mut overlay = Overlay::with_dimensions("Starting Soon", 1920, 1080).unwrap();
        let widget = TextWidget::with_properties(
            "Be right back",
            Position::new(123.5, 456.25),
            32.0,
            Color::rgba(10, 20, 30, 40),
            Alignment::Right,
        )
        .unwrap();
        overlay.add_text_widget(widget).unwrap();
        let id = overlay.text_widget().unwrap().id();
        overlay.rename("Live Soon").unwrap();
        overlay.set_text_content(id, "Live in five").unwrap();
        overlay
            .set_text_position(id, Position::new(800.0, 600.0))
            .unwrap();
        overlay.set_text_font_size(id, 48.0).unwrap();
        overlay.set_text_color(id, Color::rgba(1, 2, 3, 4)).unwrap();
        overlay.set_text_alignment(id, Alignment::Center).unwrap();
        overlay
    }

    #[test]
    fn round_trip_preserves_all_fields_and_stable_ids() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested").join(FILE_NAME);
        let original = sample_overlay();
        let overlay_id = original.id();
        let widget_id = original.text_widget().unwrap().id();
        let store = Store::at(&path);

        store.save(std::slice::from_ref(&original)).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded, vec![original]);
        assert_eq!(loaded[0].id(), overlay_id);
        assert_eq!(loaded[0].text_widget().unwrap().id(), widget_id);
        let json = fs::read_to_string(path).unwrap();
        assert!(json.contains("\"format_version\": 1"));
        assert!(json.contains("\"alignment\": \"center\""));
    }

    #[test]
    fn missing_file_creates_parent_directory_and_loads_empty_collection() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("created").join(FILE_NAME);
        let store = Store::at(&path);

        assert!(!path.parent().unwrap().exists());
        assert_eq!(store.load().unwrap(), Vec::<Overlay>::new());
        assert!(path.parent().unwrap().is_dir());
    }

    #[test]
    fn malformed_input_is_rejected_without_changing_source() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE_NAME);
        let source = b"{ not json";
        fs::write(&path, source).unwrap();

        let result = Store::at(&path).load();

        assert!(matches!(result, Err(PersistenceError::Malformed { .. })));
        assert_eq!(fs::read(&path).unwrap(), source);
    }

    #[test]
    fn unsupported_version_is_rejected_without_changing_source() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE_NAME);
        let source = br#"{"format_version":99,"overlays":[]}"#;
        fs::write(&path, source).unwrap();

        let result = Store::at(&path).load();

        assert!(matches!(
            result,
            Err(PersistenceError::UnsupportedVersion {
                found: 99,
                supported: FORMAT_VERSION
            })
        ));
        assert_eq!(fs::read(&path).unwrap(), source);
    }

    #[test]
    fn failed_save_preserves_source_and_dirty_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("existing-directory");
        fs::create_dir(&path).unwrap();
        let dirty = sample_overlay();
        let before = dirty.clone();

        let result = Store::at(&path).save(std::slice::from_ref(&dirty));

        assert!(matches!(result, Err(PersistenceError::Replace { .. })));
        assert_eq!(dirty, before);
        assert!(path.is_dir());
    }

    #[test]
    fn app_local_path_is_named_and_not_relative() {
        let path = app_local_path().unwrap();
        assert_eq!(path.file_name().unwrap(), FILE_NAME);
        assert!(path.is_absolute());
        assert!(!path.starts_with(std::env::current_dir().unwrap()));
    }
}
