//! Versioned, app-local persistence for application settings.
//!
//! Settings deliberately live in a different file and directory from the
//! overlay document.  The overlay file is user content, while this adapter
//! owns only the application-level local server port.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

/// The only settings format currently understood by this version.
pub const FORMAT_VERSION: u32 = 1;

/// The settings filename below the platform's app-local config directory.
pub const FILE_NAME: &str = "settings.json";

/// The normal local-server port used when no settings file exists.
pub const DEFAULT_PORT: u16 = 51_737;

/// The smallest valid configured server port.
pub const MIN_PORT: u16 = 1;

/// The largest valid configured server port.
pub const MAX_PORT: u16 = u16::MAX;

/// Resolves the application settings path through the platform config-local
/// directory.  It never consults the working directory.
pub fn app_local_path() -> Result<PathBuf, SettingsError> {
    let project_dirs = ProjectDirs::from("", "", "Chikachika")
        .ok_or(SettingsError::AppLocalDirectoryUnavailable)?;
    Ok(project_dirs.config_local_dir().join(FILE_NAME))
}

/// Loads application settings from an explicitly selected path.
pub fn load(path: impl AsRef<Path>) -> Result<Settings, SettingsError> {
    Store::at(path.as_ref()).load()
}

/// Saves a validated next-launch server port to an explicitly selected path.
pub fn save(path: impl AsRef<Path>, port: u16) -> Result<(), SettingsError> {
    Store::at(path.as_ref()).save_port(port)
}

/// A validated set of application settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Settings {
    server_port: u16,
}

impl Settings {
    /// Creates settings with a validated server port.
    pub fn new(server_port: u16) -> Result<Self, SettingsError> {
        validate_port(server_port)?;
        Ok(Self { server_port })
    }

    /// Returns the configured local-server port.
    pub const fn server_port(self) -> u16 {
        self.server_port
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_port: DEFAULT_PORT,
        }
    }
}

/// A settings store for one versioned JSON file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    /// Creates a store at the platform application config-local path.
    pub fn app_local() -> Result<Self, SettingsError> {
        Ok(Self::at(app_local_path()?))
    }

    /// Creates a store at `path` without touching the filesystem.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the exact settings path used by this store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Creates the parent settings directory explicitly.
    pub fn ensure_config_dir(&self) -> Result<(), SettingsError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| SettingsError::NoParentDirectory {
                path: self.path.clone(),
            })?;
        fs::create_dir_all(parent).map_err(|source| SettingsError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })
    }

    /// Loads validated settings.  A missing file means the default port; all
    /// other malformed, unsupported, or invalid data is returned as an error.
    pub fn load(&self) -> Result<Settings, SettingsError> {
        self.ensure_config_dir()?;

        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                return Ok(Settings::default());
            }
            Err(source) => {
                return Err(SettingsError::Read {
                    path: self.path.clone(),
                    source,
                });
            }
        };

        let envelope: Envelope =
            serde_json::from_slice(&bytes).map_err(|source| SettingsError::Malformed {
                path: self.path.clone(),
                source,
            })?;
        envelope.into_settings()
    }

    /// Serializes and atomically replaces this settings file.
    ///
    /// The temporary file is created in the destination directory, flushed to
    /// disk, closed, and then replaced.  If replacement fails, the previous
    /// destination remains untouched and the temporary path is cleaned up.
    pub fn save(&self, settings: Settings) -> Result<(), SettingsError> {
        validate_port(settings.server_port)?;
        let envelope = Envelope::from_settings(settings);
        let bytes = serde_json::to_vec_pretty(&envelope).map_err(SettingsError::Serialize)?;
        self.ensure_config_dir()?;

        let parent = self
            .path
            .parent()
            .ok_or_else(|| SettingsError::NoParentDirectory {
                path: self.path.clone(),
            })?;
        let mut temporary =
            NamedTempFile::new_in(parent).map_err(|source| SettingsError::CreateTemporary {
                directory: parent.to_path_buf(),
                source,
            })?;

        temporary
            .write_all(&bytes)
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|source| SettingsError::WriteTemporary {
                path: temporary.path().to_path_buf(),
                source,
            })?;

        // Close before replacement for platforms that reject renaming an open
        // file.  TempPath removes the temporary file if replacement fails.
        let temporary = temporary.into_temp_path();
        replace_file(&temporary, &self.path).map_err(|source| SettingsError::Replace {
            path: self.path.clone(),
            source,
        })?;
        Ok(())
    }

    /// Saves a validated port for the next application launch.
    pub fn save_port(&self, server_port: u16) -> Result<(), SettingsError> {
        self.save(Settings::new(server_port)?)
    }
}

/// The application-facing settings lifecycle exposed to the GUI.
///
/// `configured_port` is `Some` for a successfully loaded file or the missing
/// file default, and `None` when settings could not be loaded.  In the latter
/// case `display_port` still reports the safe default for a useful UI label,
/// but startup must not use it; `settings_error` explains why startup was
/// blocked.  Calling `save_port` updates only the next-launch setting and
/// never rebinds a currently running server or changes its readiness URL.
#[derive(Debug)]
pub struct SettingsState {
    store: Option<Store>,
    configured_port: Option<u16>,
    error: Option<SettingsError>,
}

impl SettingsState {
    /// Loads settings from a store for application startup.
    pub fn load(store: Store) -> Self {
        match store.load() {
            Ok(settings) => Self {
                store: Some(store),
                configured_port: Some(settings.server_port()),
                error: None,
            },
            Err(error) => Self {
                store: Some(store),
                configured_port: None,
                error: Some(error),
            },
        }
    }

    /// Creates a state for an unavailable settings path and its error.
    pub fn unavailable(error: SettingsError) -> Self {
        Self {
            store: None,
            configured_port: None,
            error: Some(error),
        }
    }

    /// Creates a successful state with an explicit store and validated port.
    /// This is useful for deterministic application tests and adapters.
    pub fn from_settings(store: Store, settings: Settings) -> Self {
        Self {
            store: Some(store),
            configured_port: Some(settings.server_port()),
            error: None,
        }
    }

    /// Returns the currently loaded configured port, if settings loaded.
    pub const fn configured_port(&self) -> Option<u16> {
        self.configured_port
    }

    /// Returns the port suitable for display in settings UI.
    ///
    /// This returns the default when loading failed, but that value is only a
    /// display value; callers must check [`Self::settings_error`] before
    /// starting a server.
    pub const fn display_port(&self) -> u16 {
        match self.configured_port {
            Some(port) => port,
            None => DEFAULT_PORT,
        }
    }

    /// Returns the path being used, when path resolution succeeded.
    pub fn settings_path(&self) -> Option<&Path> {
        self.store.as_ref().map(Store::path)
    }

    /// Returns the settings load error currently shown to the GUI. Save
    /// failures are returned directly from [`Self::save_port`].
    pub fn settings_error(&self) -> Option<&SettingsError> {
        self.error.as_ref()
    }

    /// Returns whether the state loaded a valid port and can start the server.
    pub const fn is_valid(&self) -> bool {
        self.configured_port.is_some() && self.error.is_none()
    }

    /// Saves a validated port for the next launch.
    ///
    /// This operation does not rebind a server that is already running.  The
    /// state is updated only after the atomic replacement succeeds.
    pub fn save_port(&mut self, port: u16) -> Result<(), SettingsError> {
        let store = self
            .store
            .as_ref()
            .ok_or(SettingsError::AppLocalDirectoryUnavailable)?;
        store.save_port(port)?;
        self.configured_port = Some(port);
        self.error = None;
        Ok(())
    }

    /// Alias that makes next-launch semantics explicit at GUI call sites.
    pub fn save_port_for_next_launch(&mut self, port: u16) -> Result<(), SettingsError> {
        self.save_port(port)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format_version: u32,
    server_port: u32,
}

impl Envelope {
    fn from_settings(settings: Settings) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            server_port: u32::from(settings.server_port()),
        }
    }

    fn into_settings(self) -> Result<Settings, SettingsError> {
        if self.format_version != FORMAT_VERSION {
            return Err(SettingsError::UnsupportedVersion {
                found: self.format_version,
                supported: FORMAT_VERSION,
            });
        }
        let port = u16::try_from(self.server_port).map_err(|_| SettingsError::InvalidPort {
            port: self.server_port,
        })?;
        Settings::new(port)
    }
}

fn validate_port(port: u16) -> Result<(), SettingsError> {
    if (MIN_PORT..=MAX_PORT).contains(&port) {
        Ok(())
    } else {
        Err(SettingsError::InvalidPort {
            port: u32::from(port),
        })
    }
}

/// An error reported by settings path resolution, JSON validation, or I/O.
#[derive(Debug)]
pub enum SettingsError {
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
    InvalidPort {
        port: u32,
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

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppLocalDirectoryUnavailable => {
                formatter.write_str("the platform app-local settings directory is unavailable")
            }
            Self::NoParentDirectory { path } => {
                write!(
                    formatter,
                    "settings path has no parent directory: {}",
                    path.display()
                )
            }
            Self::CreateDirectory { path, source } => {
                write!(
                    formatter,
                    "could not create settings directory {}: {source}",
                    path.display()
                )
            }
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "could not read settings {}: {source}",
                    path.display()
                )
            }
            Self::Malformed { path, source } => {
                write!(
                    formatter,
                    "settings {} are malformed: {source}",
                    path.display()
                )
            }
            Self::Serialize(source) => write!(formatter, "could not serialize settings: {source}"),
            Self::UnsupportedVersion { found, supported } => write!(
                formatter,
                "settings use unsupported format version {found}; supported version is {supported}"
            ),
            Self::InvalidPort { port } => write!(
                formatter,
                "settings specify invalid server port {port}; valid ports are {MIN_PORT}..={MAX_PORT}"
            ),
            Self::CreateTemporary { directory, source } => write!(
                formatter,
                "could not create temporary settings file in {}: {source}",
                directory.display()
            ),
            Self::WriteTemporary { path, source } => write!(
                formatter,
                "could not write temporary settings file {}: {source}",
                path.display()
            ),
            Self::Replace { path, source } => write!(
                formatter,
                "could not atomically replace settings {}: {source}",
                path.display()
            ),
        }
    }
}

impl Error for SettingsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateDirectory { source, .. }
            | Self::Read { source, .. }
            | Self::CreateTemporary { source, .. }
            | Self::WriteTemporary { source, .. }
            | Self::Replace { source, .. } => Some(source),
            Self::Malformed { source, .. } | Self::Serialize(source) => Some(source),
            Self::AppLocalDirectoryUnavailable
            | Self::NoParentDirectory { .. }
            | Self::UnsupportedVersion { .. }
            | Self::InvalidPort { .. } => None,
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

    fn settings_path(directory: &tempfile::TempDir) -> PathBuf {
        directory.path().join(FILE_NAME)
    }

    #[test]
    fn missing_file_uses_default_and_creates_parent_directory() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested").join(FILE_NAME);
        let store = Store::at(&path);

        assert_eq!(store.load().unwrap(), Settings::default());
        assert!(path.parent().unwrap().is_dir());
        assert_eq!(store.load().unwrap().server_port(), DEFAULT_PORT);
    }

    #[test]
    fn round_trip_preserves_port_and_versioned_shape() {
        let directory = tempfile::tempdir().unwrap();
        let path = settings_path(&directory);
        let store = Store::at(&path);
        let settings = Settings::new(42_424).unwrap();

        store.save(settings).unwrap();

        assert_eq!(store.load().unwrap(), settings);
        let json = fs::read_to_string(path).unwrap();
        assert!(json.contains("\"format_version\": 1"));
        assert!(json.contains("\"server_port\": 42424"));
    }

    #[test]
    fn malformed_settings_are_rejected_without_changing_source() {
        let directory = tempfile::tempdir().unwrap();
        let path = settings_path(&directory);
        let source = b"{ not json";
        fs::write(&path, source).unwrap();

        assert!(matches!(
            Store::at(&path).load(),
            Err(SettingsError::Malformed { .. })
        ));
        assert_eq!(fs::read(path).unwrap(), source);
    }

    #[test]
    fn unsupported_version_is_rejected_without_changing_source() {
        let directory = tempfile::tempdir().unwrap();
        let path = settings_path(&directory);
        let source = br#"{"format_version":99,"server_port":42424}"#;
        fs::write(&path, source).unwrap();

        assert!(matches!(
            Store::at(&path).load(),
            Err(SettingsError::UnsupportedVersion {
                found: 99,
                supported: FORMAT_VERSION
            })
        ));
        assert_eq!(fs::read(path).unwrap(), source);
    }

    #[test]
    fn invalid_port_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let path = settings_path(&directory);
        fs::write(&path, br#"{"format_version":1,"server_port":65536}"#).unwrap();

        assert!(matches!(
            Store::at(&path).load(),
            Err(SettingsError::InvalidPort { port: 65_536 })
        ));

        fs::write(&path, br#"{"format_version":1,"server_port":0}"#).unwrap();
        assert!(matches!(
            Store::at(&path).load(),
            Err(SettingsError::InvalidPort { port: 0 })
        ));
        assert!(matches!(
            Settings::new(0),
            Err(SettingsError::InvalidPort { port: 0 })
        ));
    }

    #[test]
    fn failed_save_preserves_prior_source() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("existing-directory");
        fs::create_dir(&path).unwrap();
        let result = Store::at(&path).save(Settings::new(42_424).unwrap());

        assert!(matches!(result, Err(SettingsError::Replace { .. })));
        assert!(path.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn failed_save_preserves_existing_source_file() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let path = settings_path(&directory);
        let store = Store::at(&path);
        store.save_port(12_345).unwrap();
        let before = fs::read(&path).unwrap();

        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let result = store.save_port(54_321);
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();

        assert!(result.is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }

    #[test]
    fn state_save_updates_next_launch_only_after_success() {
        let directory = tempfile::tempdir().unwrap();
        let path = settings_path(&directory);
        let mut state = SettingsState::from_settings(Store::at(&path), Settings::default());

        state.save_port_for_next_launch(42_424).unwrap();

        assert_eq!(state.configured_port(), Some(42_424));
        assert_eq!(state.display_port(), 42_424);
        assert_eq!(Store::at(path).load().unwrap().server_port(), 42_424);
    }

    #[test]
    fn app_local_path_is_named_and_uses_config_local_directory() {
        let path = app_local_path().unwrap();
        assert_eq!(path.file_name().unwrap(), FILE_NAME);
        assert!(path.is_absolute());
        assert!(!path.starts_with(std::env::current_dir().unwrap()));
    }
}
