mod app;
mod assets;
mod client;
mod components;
mod input;
mod replication;
mod render_scene;

use anyhow::Result;
use app::SandboxApp;
use winit::event_loop::EventLoop;

fn main() -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = SandboxApp::new()?;
    event_loop.run_app(&mut app)?;
    app.join_server()?;
    Ok(())
}
