mod gui;
pub mod model;
pub mod persistence;
mod server;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let server = server::start()?;
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
