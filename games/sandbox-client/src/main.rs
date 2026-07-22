mod app;
mod client;
mod components;
mod input;
mod network;
mod render_scene;
mod replication;

use std::error::Error;

use app::SandboxApp;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = SandboxApp::new()?;

    event_loop.run_app(&mut app)?;

    app.join_server()?;

    println!("[cliente] aplicação finalizada");

    Ok(())
}
