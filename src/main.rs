pub mod app;
pub mod browser;
mod gui;
pub mod model;
pub mod persistence;
pub mod server;

use std::error::Error;

use app::HeadlessCoordinator;
use persistence::Store;

fn main() -> Result<(), Box<dyn Error>> {
    let mut coordinator = HeadlessCoordinator::bootstrap(Store::app_local())?;
    coordinator.start()?;
    if let Some(address) = coordinator.server_address() {
        println!("Chikachika web server listening at http://{address}/ping");
    }

    let gui_result = gui::run().map_err(|error| -> Box<dyn Error> { Box::new(error) });
    let shutdown_result = coordinator.shutdown().map_err(|error| -> Box<dyn Error> {
        Box::new(error)
    });

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

    #[derive(Debug)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl Error for TestError {}

    #[test]
    fn lifecycle_returns_success_only_after_gui_and_shutdown_succeed() {
        assert!(combine_lifecycle_results(Ok(()), Ok(())).is_ok());
        assert_eq!(
            combine_lifecycle_results(
                Err(Box::new(TestError("GUI"))),
                Ok(()),
            )
            .expect_err("GUI failure is returned")
            .to_string(),
            "GUI"
        );
        assert_eq!(
            combine_lifecycle_results(
                Ok(()),
                Err(Box::new(TestError("shutdown"))),
            )
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
