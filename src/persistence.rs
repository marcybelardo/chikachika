//! Version-2 app-local persistence for ordered overlay documents.
//!
//! This adapter uses private strongly typed Serde DTOs so the domain model stays
//! framework-independent. Failed loads and saves are non-destructive.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::model::{
    Alignment, CanvasSize, Color, FontFamily, ModelError, Overlay, Position, TextWidget,
    validate_collection,
};

pub const FORMAT_VERSION: u32 = 2;
pub const FILE_NAME: &str = "overlays.json";

pub fn app_local_path() -> Result<PathBuf, PersistenceError> {
    let project_dirs = ProjectDirs::from("", "", "Chikachika")
        .ok_or(PersistenceError::AppLocalDirectoryUnavailable)?;
    Ok(project_dirs.data_local_dir().join(FILE_NAME))
}
pub fn load(path: impl AsRef<Path>) -> Result<Vec<Overlay>, PersistenceError> {
    Store::at(path.as_ref()).load()
}
pub fn save(path: impl AsRef<Path>, overlays: &[Overlay]) -> Result<(), PersistenceError> {
    Store::at(path.as_ref()).save(overlays)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Store {
    path: PathBuf,
}
impl Store {
    pub fn app_local() -> Result<Self, PersistenceError> {
        Ok(Self::at(app_local_path()?))
    }
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
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
        // Inspect the envelope version before decoding any version-specific
        // widget structure. This makes a real format-1 file UnsupportedVersion,
        // even though its fields cannot be decoded by the format-2 DTO.
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|source| PersistenceError::Malformed {
                path: self.path.clone(),
                source,
            })?;
        let found = match value.get("format_version").and_then(Value::as_u64) {
            Some(found) => found,
            None => {
                let source = serde_json::from_value::<Envelope>(value.clone())
                    .expect_err("missing or invalid format version must fail envelope decode");
                return Err(PersistenceError::Malformed {
                    path: self.path.clone(),
                    source,
                });
            }
        };
        if found > u32::MAX as u64 {
            return Err(PersistenceError::UnsupportedVersion {
                found: u32::MAX,
                supported: FORMAT_VERSION,
            });
        }
        if found as u32 != FORMAT_VERSION {
            return Err(PersistenceError::UnsupportedVersion {
                found: found as u32,
                supported: FORMAT_VERSION,
            });
        }
        let envelope: Envelope =
            serde_json::from_value(value).map_err(|source| PersistenceError::Malformed {
                path: self.path.clone(),
                source,
            })?;
        envelope.into_models()
    }
    pub fn save(&self, overlays: &[Overlay]) -> Result<(), PersistenceError> {
        self.save_internal(overlays, None)
    }

    #[cfg(test)]
    pub fn save_with_failure(
        &self,
        overlays: &[Overlay],
        stage: SaveFailureStage,
    ) -> Result<(), PersistenceError> {
        self.save_internal(overlays, Some(stage.into()))
    }

    fn save_internal(
        &self,
        overlays: &[Overlay],
        failure: Option<FailureStage>,
    ) -> Result<(), PersistenceError> {
        validate_collection(overlays).map_err(PersistenceError::InvalidModel)?;
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
        if failure == Some(FailureStage::Write) {
            return Err(PersistenceError::WriteTemporary {
                path: temporary.path().to_path_buf(),
                source: injected_error("write"),
            });
        }
        temporary
            .write_all(&bytes)
            .and_then(|()| {
                if failure == Some(FailureStage::Sync) {
                    return Err(injected_error("sync"));
                }
                temporary.as_file().sync_all()
            })
            .map_err(|source| PersistenceError::WriteTemporary {
                path: temporary.path().to_path_buf(),
                source,
            })?;
        let temporary = temporary.into_temp_path();
        if failure == Some(FailureStage::Replace) {
            return Err(PersistenceError::Replace {
                path: self.path.clone(),
                source: injected_error("replace"),
            });
        }
        replace_file(&temporary, &self.path).map_err(|source| PersistenceError::Replace {
            path: self.path.clone(),
            source,
        })?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailureStage {
    Write,
    Sync,
    Replace,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveFailureStage {
    Write,
    Sync,
    Replace,
}

#[cfg(test)]
impl From<SaveFailureStage> for FailureStage {
    fn from(stage: SaveFailureStage) -> Self {
        match stage {
            SaveFailureStage::Write => Self::Write,
            SaveFailureStage::Sync => Self::Sync,
            SaveFailureStage::Replace => Self::Replace,
        }
    }
}

fn injected_error(stage: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("injected {stage} failure"))
}

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
        let mut ids = HashSet::with_capacity(self.overlays.len());
        let mut overlays = Vec::with_capacity(self.overlays.len());
        for overlay in self.overlays {
            if !ids.insert(overlay.id) {
                return Err(PersistenceError::DuplicateOverlayId { id: overlay.id });
            }
            overlays.push(overlay.into_model()?);
        }
        validate_collection(&overlays).map_err(PersistenceError::InvalidModel)?;
        Ok(overlays)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedOverlay {
    id: Uuid,
    name: String,
    canvas: PersistedCanvasSize,
    widgets: Vec<PersistedTextWidget>,
}
impl PersistedOverlay {
    fn from_model(overlay: &Overlay) -> Self {
        Self {
            id: overlay.id().as_uuid(),
            name: overlay.name().to_owned(),
            canvas: PersistedCanvasSize::from_model(overlay.canvas()),
            widgets: overlay
                .widgets()
                .iter()
                .map(PersistedTextWidget::from_model)
                .collect(),
        }
    }
    fn into_model(self) -> Result<Overlay, PersistenceError> {
        let widgets = self
            .widgets
            .into_iter()
            .map(PersistedTextWidget::into_model)
            .collect::<Result<Vec<_>, _>>()?;
        Overlay::from_parts(
            crate::model::OverlayId::from_uuid(self.id),
            self.name,
            self.canvas.into_model()?,
            widgets,
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
    name: String,
    content: String,
    font_family: String,
    position: PersistedPosition,
    font_size: f32,
    color: PersistedColor,
    alignment: PersistedAlignment,
}
impl PersistedTextWidget {
    fn from_model(widget: &TextWidget) -> Self {
        Self {
            id: widget.id().as_uuid(),
            name: widget.name().to_owned(),
            content: widget.content().to_owned(),
            font_family: widget.font_family().id().to_owned(),
            position: PersistedPosition::from_model(widget.position()),
            font_size: widget.font_size(),
            color: PersistedColor::from_model(widget.color()),
            alignment: PersistedAlignment::from_model(widget.alignment()),
        }
    }
    fn into_model(self) -> Result<TextWidget, PersistenceError> {
        let font_family =
            FontFamily::parse(&self.font_family).map_err(PersistenceError::InvalidModel)?;
        TextWidget::from_parts(
            crate::model::TextWidgetId::from_uuid(self.id),
            self.name,
            self.content,
            font_family,
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
}
impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppLocalDirectoryUnavailable => {
                write!(f, "the platform app-local data directory is unavailable")
            }
            Self::NoParentDirectory { path } => write!(
                f,
                "persistence path has no parent directory: {}",
                path.display()
            ),
            Self::CreateDirectory { path, source } => write!(
                f,
                "could not create persistence directory {}: {source}",
                path.display()
            ),
            Self::Read { path, source } => write!(
                f,
                "could not read persisted overlays {}: {source}",
                path.display()
            ),
            Self::Malformed { path, source } => write!(
                f,
                "persisted overlays {} are malformed: {source}",
                path.display()
            ),
            Self::Serialize(source) => write!(f, "could not serialize overlays: {source}"),
            Self::UnsupportedVersion { found, supported } => write!(
                f,
                "persisted overlays use unsupported format version {found}; supported version is {supported}"
            ),
            Self::InvalidModel(source) => write!(f, "persisted overlay data is invalid: {source}"),
            Self::DuplicateOverlayId { id } => {
                write!(f, "persisted overlays contain duplicate overlay ID {id}")
            }
            Self::CreateTemporary { directory, source } => write!(
                f,
                "could not create temporary persistence file in {}: {source}",
                directory.display()
            ),
            Self::WriteTemporary { path, source } => write!(
                f,
                "could not write temporary persistence file {}: {source}",
                path.display()
            ),
            Self::Replace { path, source } => write!(
                f,
                "could not atomically replace persisted overlays {}: {source}",
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
            | Self::Replace { source, .. } => Some(source),
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
    const MOVEFILE_REPLACE_EXISTING: u32 = 1;
    const MOVEFILE_WRITE_THROUGH: u32 = 8;
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
    use crate::model::{Color, Position};
    fn sample_overlay() -> Overlay {
        let mut overlay = Overlay::with_dimensions("Live Soon", 1920, 1080).unwrap();
        let first = TextWidget::with_all_properties(
            "First",
            "Be right back",
            FontFamily::JetBrainsMono,
            Position::new(123.5, 456.25),
            32.0,
            Color::rgba(10, 20, 30, 40),
            Alignment::Right,
        )
        .unwrap();
        let first_id = overlay.add_widget(first).unwrap();
        overlay.duplicate_widget(first_id).unwrap();
        overlay.rename("Saved").unwrap();
        overlay
    }
    #[test]
    fn format_two_round_trip_and_transient_omission() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("nested").join(FILE_NAME);
        let original = sample_overlay();
        let store = Store::at(&path);
        store.save(std::slice::from_ref(&original)).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded, vec![original]);
        let json = fs::read_to_string(path).unwrap();
        assert!(json.contains("\"format_version\": 2"));
        assert!(json.contains("\"widgets\""));
        assert!(json.contains("\"font_family\": \"jetbrains-mono\""));
        assert!(!json.contains("revision"));
        assert!(!json.contains("selected"));
    }
    #[test]
    fn format_one_rejected_non_destructively() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join(FILE_NAME);
        let source = br#"{"format_version":1,"overlays":[{"text_widget":null}]}"#;
        fs::write(&path, source).unwrap();
        assert!(matches!(
            Store::at(&path).load(),
            Err(PersistenceError::UnsupportedVersion {
                found: 1,
                supported: 2
            })
        ));
        assert_eq!(fs::read(path).unwrap(), source);
    }
    #[test]
    fn invalid_collection_load_preserves_source() {
        let d = tempfile::tempdir().unwrap();
        for source in [br#"{"format_version":2,"overlays":[{"id":"00000000-0000-0000-0000-000000000000","name":"x","canvas":{"width":1,"height":1},"widgets":[] }]}"#.to_vec(), br#"{"format_version":2,"overlays":[{"id":"00000000-0000-4000-8000-000000000001","name":"x","canvas":{"width":1,"height":1},"widgets":[{"id":"00000000-0000-4000-8000-000000000002","name":"x","content":"x","font_family":"bad","position":{"x":0.0,"y":0.0},"font_size":16.0,"color":{"red":0,"green":0,"blue":0,"alpha":255},"alignment":"left"}]}]}"#.to_vec()] {
            let path = d.path().join("invalid.json"); fs::write(&path, &source).unwrap(); let result = Store::at(&path).load(); assert!(result.is_err()); assert_eq!(fs::read(&path).unwrap(), source); fs::remove_file(&path).unwrap();
        }
    }
    #[test]
    fn failed_save_preserves_previous_regular_file_at_each_stage() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join(FILE_NAME);
        let original = sample_overlay();
        let changed = Overlay::with_dimensions("Changed", 1, 1).unwrap();
        let store = Store::at(&path);
        store.save(&[original]).unwrap();
        let source = fs::read(&path).unwrap();
        for stage in [
            SaveFailureStage::Write,
            SaveFailureStage::Sync,
            SaveFailureStage::Replace,
        ] {
            assert!(store.save_with_failure(&[changed.clone()], stage).is_err());
            assert_eq!(fs::read(&path).unwrap(), source);
        }
    }
    #[test]
    fn missing_file_creates_parent_directory_and_loads_empty_collection() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("created").join(FILE_NAME);
        let store = Store::at(&path);
        assert_eq!(store.load().unwrap(), Vec::<Overlay>::new());
        assert!(path.parent().unwrap().is_dir());
    }
    #[test]
    fn malformed_input_is_rejected_without_changing_source() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join(FILE_NAME);
        let source = b"{ not json";
        fs::write(&path, source).unwrap();
        assert!(matches!(
            Store::at(&path).load(),
            Err(PersistenceError::Malformed { .. })
        ));
        assert_eq!(fs::read(&path).unwrap(), source);
    }
    #[test]
    fn app_local_path_is_absolute_and_named() {
        let path = app_local_path().unwrap();
        assert_eq!(path.file_name().unwrap(), FILE_NAME);
        assert!(path.is_absolute());
    }
}
