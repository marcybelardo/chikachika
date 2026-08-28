pub mod app;
pub mod browser;
mod gui;
pub mod model;
pub mod persistence;
pub mod server;

use std::error::Error;
use std::net::SocketAddr;

use app::{BootstrapFailure, BootstrapOutcome, HeadlessCoordinator};
use persistence::Store;
use server::{ServerError, ServerHandle};

fn main() -> Result<(), Box<dyn Error>> {
    let Startup { outcome, server } = startup_with(Store::app_local, |endpoint, hub| {
        server::start_on_port_with_hub(endpoint.port(), hub)
    });
    if let Some(server) = server.as_ref() {
        println!(
            "Chikachika web server listening at http://{}/ping",
            server.local_addr()
        );
    }

    run_gui_with_shutdown(outcome, server, |outcome| {
        gui::run(outcome).map_err(|error| -> Box<dyn Error> { Box::new(error) })
    })
}

struct Startup {
    outcome: BootstrapOutcome,
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

fn startup_with<ResolveStore, StartServer>(
    resolve_store: ResolveStore,
    start_server: StartServer,
) -> Startup
where
    ResolveStore: FnOnce() -> Result<Store, persistence::PersistenceError>,
    StartServer: FnOnce(SocketAddr, crate::server::OverlayHub) -> Result<ServerHandle, ServerError>,
{
    let outcome = match resolve_store() {
        Ok(store) => HeadlessCoordinator::<crate::server::OverlayHub>::bootstrap_outcome(store),
        Err(error) => BootstrapOutcome::Blocked(BootstrapFailure::without_store(error)),
    };
    let BootstrapOutcome::Ready(mut coordinator) = outcome else {
        return Startup {
            outcome,
            server: None,
        };
    };

    let endpoint = SocketAddr::from((server::DEFAULT_BIND_ADDRESS, server::DEFAULT_PORT));
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
        server,
    }
}

fn run_gui_with_shutdown<H, RunGui>(
    outcome: BootstrapOutcome,
    server: Option<H>,
    run_gui: RunGui,
) -> Result<(), Box<dyn Error>>
where
    H: ShutdownCapable,
    RunGui: FnOnce(BootstrapOutcome) -> Result<(), Box<dyn Error>>,
{
    let gui_result = run_gui(outcome);
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

    #[test]
    fn normal_gui_return_shuts_down_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let result = run_gui_with_shutdown(
            test_outcome(),
            Some(CountingShutdown {
                calls: calls.clone(),
                error: None,
            }),
            |_outcome| Ok(()),
        );
        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn gui_error_still_shuts_down_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let result = run_gui_with_shutdown(
            test_outcome(),
            Some(CountingShutdown {
                calls: calls.clone(),
                error: None,
            }),
            |_outcome| Err(Box::new(TestError("GUI"))),
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
            Some(CountingShutdown {
                calls: calls.clone(),
                error: Some("shutdown"),
            }),
            |_outcome| Ok(()),
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
            Some(CountingShutdown {
                calls: calls.clone(),
                error: Some("shutdown"),
            }),
            |_outcome| Err(Box::new(TestError("GUI"))),
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
