pub mod browser;
mod gui;
pub mod model;
pub mod persistence;
pub mod server;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let hub = server::OverlayHub::new();
    let server = server::start_with_hub(hub)?;
    println!(
        "Chikachika web server listening at http://{}/ping",
        server.local_addr()
    );

    let gui_result = gui::run();
    let shutdown_result = server.shutdown();

    gui_result?;
    shutdown_result?;
    Ok(())
}
