pub mod app;
pub mod browser;
mod gui;
pub mod model;
pub mod persistence;
pub mod server;
pub mod settings;

use std::error::Error;
use std::net::SocketAddr;

use app::{BootstrapFailure, BootstrapOutcome, HeadlessCoordinator};
use persistence::Store;
use server::{ServerError, ServerHandle};
use settings::{SettingsState, Store as SettingsStore};

fn main() -> Result<(), Box<dyn Error>> {
    let Startup {
        outcome,
        settings,
        server,
    } = startup_with_resolvers(
        Store::app_local,
        SettingsStore::app_local,
        |endpoint, hub| server::start_on_port_with_hub(endpoint.port(), hub),
    );
    if let Some(server) = server.as_ref() {
        println!(
            "Chikachika web server listening at http://{}/ping",
            server.local_addr()
        );
    }

    run_gui_with_shutdown(outcome, settings, server, |outcome, settings| {
        gui::run(outcome.with_settings(settings))
            .map_err(|error| -> Box<dyn Error> { Box::new(error) })
    })
}

struct Startup {
    outcome: BootstrapOutcome,
    settings: SettingsState,
    server: Option<ServerHandle>,
}

/// A small ownership seam around the consuming server shutdown operation.
/// Production uses [`ServerHandle`], while unit tests can provide a counting
/// handle and prove that shutdown is attempted exactly once after GUI return.
trait ShutdownCapable {
    fn shutdown(self) -> Result<(), Box<dyn Error>>;
}

impl ShutdownCapable for ServerHandle {
    fn shutdown(self) -> Result<(), Box<dyn Error>> {
        ServerHandle::shutdown(self).map_err(|error| Box::new(error) as Box<dyn Error>)
    }
}

#[cfg(test)]
fn startup_with<ResolveStore, StartServer>(
    resolve_store: ResolveStore,
    start_server: StartServer,
) -> Startup
where
    ResolveStore: FnOnce() -> Result<Store, persistence::PersistenceError>,
    StartServer: FnOnce(SocketAddr, crate::server::OverlayHub) -> Result<ServerHandle, ServerError>,
{
    startup_with_resolvers(
        resolve_store,
        || Ok(SettingsStore::at("test-settings.json")),
        start_server,
    )
}

fn startup_with_resolvers<ResolveStore, ResolveSettings, StartServer>(
    resolve_store: ResolveStore,
    resolve_settings: ResolveSettings,
    start_server: StartServer,
) -> Startup
where
    ResolveStore: FnOnce() -> Result<Store, persistence::PersistenceError>,
    ResolveSettings: FnOnce() -> Result<SettingsStore, settings::SettingsError>,
    StartServer: FnOnce(SocketAddr, crate::server::OverlayHub) -> Result<ServerHandle, ServerError>,
{
    let settings = match resolve_settings() {
        Ok(store) => SettingsState::load(store),
        Err(error) => SettingsState::unavailable(error),
    };
    let outcome = match resolve_store() {
        Ok(store) => HeadlessCoordinator::<crate::server::OverlayHub>::bootstrap_outcome(store),
        Err(error) => BootstrapOutcome::Blocked(BootstrapFailure::without_store(error)),
    };
    let BootstrapOutcome::Ready(mut coordinator) = outcome else {
        return Startup {
            outcome,
            settings,
            server: None,
        };
    };

    let Some(port) = settings.configured_port() else {
        if let Some(error) = settings.settings_error() {
            coordinator.record_settings_error(error);
        }
        return Startup {
            outcome: BootstrapOutcome::Ready(coordinator),
            settings,
            server: None,
        };
    };
    let endpoint = SocketAddr::from((server::DEFAULT_BIND_ADDRESS, port));
    let server = match start_server(endpoint, coordinator.hub()) {
        Ok(server) => {
            coordinator.set_server_address(server.local_addr());
            Some(server)
        }
        Err(error) => {
            coordinator.record_server_error(error);
            None
        }
    };

    Startup {
        outcome: BootstrapOutcome::Ready(coordinator),
        settings,
        server,
    }
}

fn run_gui_with_shutdown<H, RunGui>(
    outcome: BootstrapOutcome,
    settings: SettingsState,
    server: Option<H>,
    run_gui: RunGui,
) -> Result<(), Box<dyn Error>>
where
    H: ShutdownCapable,
    RunGui: FnOnce(BootstrapOutcome, SettingsState) -> Result<(), Box<dyn Error>>,
{
    let gui_result = run_gui(outcome, settings);
    let shutdown_result = server.map(|server| server.shutdown()).unwrap_or(Ok(()));

    combine_lifecycle_results(gui_result, shutdown_result)
}

fn combine_lifecycle_results(
    gui_result: Result<(), Box<dyn Error>>,
    shutdown_result: Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    match (gui_result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(gui_error), Ok(())) => Err(gui_error),
        (Ok(()), Err(shutdown_error)) => Err(shutdown_error),
        (Err(gui_error), Err(shutdown_error)) => Err(Box::new(LifecycleError {
            gui_error,
            shutdown_error,
        })),
    }
}

#[derive(Debug)]
struct LifecycleError {
    gui_error: Box<dyn Error>,
    shutdown_error: Box<dyn Error>,
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "GUI failed ({}) and server shutdown failed ({})",
            self.gui_error, self.shutdown_error
        )
    }
}

impl Error for LifecycleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.gui_error.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Debug)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl Error for TestError {}

    #[test]
    fn startup_does_not_invoke_server_for_blocked_persistence() {
        let result = startup_with(
            || Err(persistence::PersistenceError::AppLocalDirectoryUnavailable),
            |_endpoint, _hub| panic!("server starter must not run for blocked persistence"),
        );
        assert!(matches!(result.outcome, BootstrapOutcome::Blocked(_)));
        assert!(result.server.is_none());
    }

    #[test]
    fn startup_keeps_loaded_workspace_when_server_start_fails() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("overlays.json");
        let overlay = crate::model::Overlay::with_dimensions("Saved", 320, 240).unwrap();
        persistence::save(&path, std::slice::from_ref(&overlay)).unwrap();
        let result = startup_with(
            || Ok(Store::at(&path)),
            |_endpoint, _hub| {
                Err(ServerError::Bind(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    "occupied",
                )))
            },
        );
        let BootstrapOutcome::Ready(coordinator) = result.outcome else {
            panic!("workspace should remain usable")
        };
        assert_eq!(coordinator.overlays(), std::slice::from_ref(&overlay));
        assert!(coordinator.last_error().unwrap().contains("occupied"));
        assert!(result.server.is_none());
        assert!(coordinator.selected_url().is_none());
    }

    fn available_port() -> u16 {
        let listener = std::net::TcpListener::bind((server::DEFAULT_BIND_ADDRESS, 0)).unwrap();
        listener.local_addr().unwrap().port()
    }

    fn save_settings(path: &std::path::Path, port: u16) {
        SettingsStore::at(path).save_port(port).unwrap();
    }

    #[test]
    fn startup_uses_the_configured_settings_port() {
        let directory = tempfile::tempdir().unwrap();
        let settings_path = directory.path().join(settings::FILE_NAME);
        let port = available_port();
        save_settings(&settings_path, port);

        let seen_endpoint = Arc::new(std::sync::Mutex::new(None));
        let seen_endpoint_for_start = seen_endpoint.clone();
        let result = startup_with_resolvers(
            || Ok(Store::at(directory.path().join("overlays.json"))),
            || Ok(SettingsStore::at(&settings_path)),
            move |endpoint, hub| {
                *seen_endpoint_for_start.lock().unwrap() = Some(endpoint);
                server::start_on_port_with_hub(endpoint.port(), hub)
            },
        );

        assert_eq!(result.settings.configured_port(), Some(port));
        assert_eq!(seen_endpoint.lock().unwrap().unwrap().port(), port);
        assert_eq!(result.server.as_ref().unwrap().local_addr().port(), port);
        result.server.unwrap().shutdown().unwrap();
    }

    #[test]
    fn settings_failure_keeps_loaded_workspace_but_blocks_only_server_startup() {
        let directory = tempfile::tempdir().unwrap();
        let overlays_path = directory.path().join("overlays.json");
        let settings_path = directory.path().join(settings::FILE_NAME);
        let overlay = crate::model::Overlay::with_dimensions("Saved", 320, 240).unwrap();
        persistence::save(&overlays_path, std::slice::from_ref(&overlay)).unwrap();
        std::fs::write(&settings_path, b"{ not json").unwrap();

        let result = startup_with_resolvers(
            || Ok(Store::at(&overlays_path)),
            || Ok(SettingsStore::at(&settings_path)),
            |_endpoint, _hub| panic!("server starter must not run for invalid settings"),
        );

        assert!(result.settings.configured_port().is_none());
        assert!(matches!(
            result.settings.settings_error(),
            Some(settings::SettingsError::Malformed { .. })
        ));
        let BootstrapOutcome::Ready(coordinator) = result.outcome else {
            panic!("valid overlay workspace should remain usable")
        };
        assert_eq!(coordinator.overlays(), std::slice::from_ref(&overlay));
        assert!(coordinator.server_address().is_none());
        assert!(coordinator.last_error().unwrap().contains("settings"));
        assert!(result.server.is_none());
    }

    #[test]
    fn configured_port_conflict_does_not_fallback_to_another_port() {
        let directory = tempfile::tempdir().unwrap();
        let settings_path = directory.path().join(settings::FILE_NAME);
        let port = available_port();
        save_settings(&settings_path, port);
        let seen_endpoint = Arc::new(std::sync::Mutex::new(None));
        let seen_endpoint_for_start = seen_endpoint.clone();

        let result = startup_with_resolvers(
            || Ok(Store::at(directory.path().join("overlays.json"))),
            || Ok(SettingsStore::at(&settings_path)),
            move |endpoint, _hub| {
                *seen_endpoint_for_start.lock().unwrap() = Some(endpoint);
                Err(ServerError::Bind(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    "occupied",
                )))
            },
        );

        assert_eq!(seen_endpoint.lock().unwrap().unwrap().port(), port);
        assert!(result.server.is_none());
        let BootstrapOutcome::Ready(coordinator) = result.outcome else {
            panic!("bind conflict must retain the loaded workspace")
        };
        assert!(coordinator.server_address().is_none());
        assert!(coordinator.last_error().unwrap().contains("occupied"));
    }

    struct CountingShutdown {
        calls: Arc<AtomicUsize>,
        error: Option<&'static str>,
    }

    impl ShutdownCapable for CountingShutdown {
        fn shutdown(self) -> Result<(), Box<dyn Error>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            match self.error {
                Some(error) => Err(Box::new(TestError(error))),
                None => Ok(()),
            }
        }
    }

    fn test_outcome() -> BootstrapOutcome {
        BootstrapOutcome::Ready(HeadlessCoordinator::empty(Store::at(
            "test-lifecycle-overlays.json",
        )))
    }

    fn test_settings() -> SettingsState {
        SettingsState::from_settings(
            SettingsStore::at("test-lifecycle-settings.json"),
            settings::Settings::default(),
        )
    }

    #[test]
    fn normal_gui_return_shuts_down_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let result = run_gui_with_shutdown(
            test_outcome(),
            test_settings(),
            Some(CountingShutdown {
                calls: calls.clone(),
                error: None,
            }),
            |_outcome, _settings| Ok(()),
        );
        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn gui_error_still_shuts_down_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let result = run_gui_with_shutdown(
            test_outcome(),
            test_settings(),
            Some(CountingShutdown {
                calls: calls.clone(),
                error: None,
            }),
            |_outcome, _settings| Err(Box::new(TestError("GUI"))),
        );
        assert_eq!(
            result.expect_err("GUI error is returned").to_string(),
            "GUI"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn shutdown_error_is_preserved() {
        let calls = Arc::new(AtomicUsize::new(0));
        let result = run_gui_with_shutdown(
            test_outcome(),
            test_settings(),
            Some(CountingShutdown {
                calls: calls.clone(),
                error: Some("shutdown"),
            }),
            |_outcome, _settings| Ok(()),
        );
        assert_eq!(
            result.expect_err("shutdown error is returned").to_string(),
            "shutdown"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn combined_gui_and_shutdown_errors_are_preserved() {
        let calls = Arc::new(AtomicUsize::new(0));
        let result = run_gui_with_shutdown(
            test_outcome(),
            test_settings(),
            Some(CountingShutdown {
                calls: calls.clone(),
                error: Some("shutdown"),
            }),
            |_outcome, _settings| Err(Box::new(TestError("GUI"))),
        );
        assert_eq!(
            result.expect_err("combined error is returned").to_string(),
            "GUI failed (GUI) and server shutdown failed (shutdown)"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn lifecycle_returns_success_only_after_gui_and_shutdown_succeed() {
        assert!(combine_lifecycle_results(Ok(()), Ok(())).is_ok());
        assert_eq!(
            combine_lifecycle_results(Err(Box::new(TestError("GUI"))), Ok(()),)
                .expect_err("GUI failure is returned")
                .to_string(),
            "GUI"
        );
        assert_eq!(
            combine_lifecycle_results(Ok(()), Err(Box::new(TestError("shutdown"))),)
                .expect_err("shutdown failure is returned")
                .to_string(),
            "shutdown"
        );
    }

    #[test]
    fn lifecycle_keeps_both_failures_visible() {
        let error = combine_lifecycle_results(
            Err(Box::new(TestError("GUI"))),
            Err(Box::new(TestError("shutdown"))),
        )
        .expect_err("both failures are returned");
        assert_eq!(
            error.to_string(),
            "GUI failed (GUI) and server shutdown failed (shutdown)"
        );
    }
}
