mod app;
mod args;
mod client;
mod components;
mod connection;
mod input;
mod network;
mod render_scene;
mod replication;

use std::error::Error;

use app::SandboxApp;
use args::ClientArgs;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn Error>> {
    let Some(args) = ClientArgs::parse()
        .map_err(|message| -> Box<dyn Error> { message.into() })?
    else {
        return Ok(());
    };

    let event_loop = EventLoop::new()?;
    let mut app = SandboxApp::new(args)?;

    event_loop.run_app(&mut app)?;
    app.join_server()?;

    println!("[cliente] aplicação finalizada");

    Ok(())
}
